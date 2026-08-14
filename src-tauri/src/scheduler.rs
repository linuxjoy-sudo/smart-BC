use crate::db::reminders::{ReminderRow, reminders_due_soon};
use chrono::{Duration, NaiveDateTime};
use rusqlite::Connection;
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
        loop {
            {
                let guard = conn.lock().unwrap();
                let now = chrono::Local::now().naive_local();
                let now_iso = now.format("%Y-%m-%d %H:%M:%S").to_string();
                if let Ok(reminders) = reminders_due_soon(&guard, &now_iso) {
                    let due = due_reminders_for_notification(&reminders, now);
                    for r in due {
                        let cfg = crate::config::load_config(&data_dir);
                        crate::voice::reply::deliver_reply(
                            &app,
                            &data_dir,
                            &cfg.reply_mode,
                            format!("提醒：{}", r.content),
                        );
                        let _ = mark_notified(&guard, r.id);
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}
