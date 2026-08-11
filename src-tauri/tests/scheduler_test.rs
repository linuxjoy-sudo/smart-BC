use chrono::NaiveDateTime;
use smart_bc::db::reminders::ReminderRow;
use smart_bc::scheduler::due_reminders_for_notification;

fn dt(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
}

fn row(id: i64, due: &str) -> ReminderRow {
    ReminderRow {
        id, content: format!("任务{id}"), due_at: Some(due.to_string()),
        status: "pending".into(), needs_time: false, conversation_id: 1,
    }
}

#[test]
fn picks_only_reminders_within_15min() {
    let now = dt("2026-08-10 15:00");
    let soon = row(1, "2026-08-10 15:10:00");
    let ok = row(2, "2026-08-10 15:14:00");
    let too_far = row(3, "2026-08-10 15:16:00");
    let past = row(4, "2026-08-10 14:00:00");
    let due = due_reminders_for_notification(&[soon.clone(), ok.clone(), too_far, past], now);
    let ids: Vec<i64> = due.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![1, 2, 4]);
}

#[test]
fn ignores_null_due() {
    let now = dt("2026-08-10 15:00");
    let r = ReminderRow { id: 9, content: "无时间".into(), due_at: None,
        status: "pending".into(), needs_time: true, conversation_id: 1 };
    assert!(due_reminders_for_notification(&[r], now).is_empty());
}
