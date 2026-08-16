use crate::db::reminders::{ReminderRow, reminders_due_soon};
use chrono::{Duration, NaiveDateTime};
use rusqlite::Connection;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

/// 待语音响应的提醒 id（0=无）：触发后由 dialog 聆听"完成/延后"指令。
pub static PENDING_REMINDER_ID: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
pub fn due_reminders_for_notification(reminders: &[ReminderRow], now: NaiveDateTime) -> Vec<ReminderRow> {
    let cutoff = now + Duration::minutes(15);
    reminders
        .iter()
        .filter(|r| {
            if r.status != "pending" { return false; }
            match &r.due_at {
                Some(d) => match NaiveDateTime::parse_from_str(d, "%Y-%m-%d %H:%M:%S") {
                    Ok(dt) => dt <= cutoff,
                    Err(_) => false,
                },
                None => false,
            }
        })
        .cloned()
        .collect()
}

pub fn mark_notified(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE reminders SET notified_at = datetime('now', 'localtime') WHERE id = ?1",
        rusqlite::params![id],
    )?;
    Ok(())
}

fn friendly_due(due_at: &str) -> String {
    let Ok(dt) = NaiveDateTime::parse_from_str(due_at, "%Y-%m-%d %H:%M:%S") else {
        return due_at.to_string();
    };
    let now = chrono::Local::now().naive_local();
    if dt.date() == now.date() {
        dt.format("%H:%M").to_string()
    } else if dt.date() == now.date().succ_opt().unwrap_or(now.date()) {
        format!("明天{}", dt.format("%H:%M"))
    } else {
        dt.format("%m月%d日 %H:%M").to_string()
    }
}

pub fn spawn(
    conn: Arc<Mutex<Connection>>,
    app: AppHandle,
    data_dir: std::path::PathBuf,
) {
    std::thread::spawn(move || {
        // 循环扫描：覆盖启动后新增的提醒（30s 内发现并调度）
        let mut scheduled: HashSet<i64> = HashSet::new();
        loop {
            sync_due(&conn, &app, &data_dir, &mut scheduled);
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
    });
}

fn sync_due(conn: &Arc<Mutex<Connection>>, app: &AppHandle, data_dir: &std::path::Path, scheduled: &mut HashSet<i64>) {
    let now = chrono::Local::now().naive_local();
    let now_iso = now.format("%Y-%m-%d %H:%M:%S").to_string();
    // 已通知超过1天仍未完成的提醒自动流转 expired
    if let Ok(guard) = conn.lock() {
        let _ = guard.execute(
            "UPDATE reminders SET status = 'expired'
             WHERE status = 'pending' AND notified_at IS NOT NULL
               AND due_at IS NOT NULL AND due_at < datetime('now', 'localtime', '-1 day')",
            [],
        );
    }
    let due = {
        let guard = conn.lock().unwrap();
        reminders_due_soon(&guard, &now_iso).unwrap_or_default()
    };
    for r in due {
        let cfg = crate::config::load_config(data_dir);
        let content = crate::memory::extract::clean_reminder_content(&r.content);
        let due_txt = r.due_at.as_deref().map(friendly_due).unwrap_or_default();
        crate::voice::reply::deliver_reply(app, data_dir, &cfg.reply_mode, format!("补发提醒：{}，原定{}", content, due_txt));
        PENDING_REMINDER_ID.store(r.id, std::sync::atomic::Ordering::SeqCst);
        if let Ok(guard) = conn.lock() {
            let _ = mark_notified(&guard, r.id);
        }
    }
    let future = {
        let guard = conn.lock().unwrap();
        crate::db::reminders::list_future_reminders(&guard, &now_iso).unwrap_or_default()
    };
    for r in future {
        if scheduled.insert(r.id) {
            if let Some(due_at) = r.due_at.clone() {
                schedule_timer(conn.clone(), app.clone(), data_dir.to_path_buf(), r, due_at);
            }
        }
    }
}

fn schedule_timer(
    conn: Arc<Mutex<Connection>>,
    app: AppHandle,
    data_dir: std::path::PathBuf,
    reminder: ReminderRow,
    due_at: String,
) {
    tauri::async_runtime::spawn(async move {
        let wait = match NaiveDateTime::parse_from_str(&due_at, "%Y-%m-%d %H:%M:%S") {
            Ok(dt) => {
                let now = chrono::Local::now().naive_local();
                (dt - now).to_std().unwrap_or_default()
            }
            Err(_) => std::time::Duration::ZERO,
        };
        tokio::time::sleep(wait).await;
        let still_pending = {
            let guard = conn.lock().unwrap();
            matches!(
                crate::db::reminders::get_reminder(&guard, reminder.id),
                Ok(Some(r)) if r.status == "pending"
            )
        };
        if still_pending {
            let cfg = crate::config::load_config(&data_dir);
            let content = crate::memory::extract::clean_reminder_content(&reminder.content);
            crate::voice::reply::deliver_reply(
                &app,
                &data_dir,
                &cfg.reply_mode,
                format!("到时间了，该{}了", content),
            );
            PENDING_REMINDER_ID.store(reminder.id, std::sync::atomic::Ordering::SeqCst);
            if let Ok(guard) = conn.lock() {
                let _ = mark_notified(&guard, reminder.id);
            }
        }
    });
}
