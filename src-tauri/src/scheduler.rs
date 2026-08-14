use crate::db::reminders::{ReminderRow, reminders_due_soon};
use chrono::{Duration, NaiveDateTime};
use rusqlite::Connection;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
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
    let due = {
        let guard = conn.lock().unwrap();
        reminders_due_soon(&guard, &now_iso).unwrap_or_default()
    };
    for r in due {
        let cfg = crate::config::load_config(data_dir);
        crate::voice::reply::deliver_reply(app, data_dir, &cfg.reply_mode, format!("提醒：{}", r.content));
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
            crate::voice::reply::deliver_reply(
                &app,
                &data_dir,
                &cfg.reply_mode,
                format!("提醒：{}", reminder.content),
            );
            if let Ok(guard) = conn.lock() {
                let _ = mark_notified(&guard, reminder.id);
            }
        }
    });
}
