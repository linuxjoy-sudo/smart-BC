use rusqlite::Connection;
use smart_bc::db;
use smart_bc::memory::types::ReminderExtract;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

fn fixed_now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 8, 10).unwrap().and_hms_opt(9, 0, 0).unwrap()
}

#[test]
fn save_and_list_reminders() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "周三交方案", None).unwrap();
    let ids = db::reminders::save_reminders(&conn, &[
        ReminderExtract { content: "周三交方案".into(), due: Some("周三".into()) },
        ReminderExtract { content: "买牛奶".into(), due: None },
    ], cid, fixed_now()).unwrap();
    assert_eq!(ids.len(), 2);
    let all = db::reminders::list_reminders(&conn).unwrap();
    assert_eq!(all.len(), 2);
    // 买牛奶 due 解析失败 → needs_time
    assert!(all.iter().any(|r| r.content == "买牛奶" && r.needs_time));
}

#[test]
fn status_whitelist() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "x", None).unwrap();
    let ids = db::reminders::save_reminders(&conn, &[ReminderExtract { content: "任务".into(), due: Some("明天".into()) }], cid, fixed_now()).unwrap();
    db::reminders::set_status(&conn, ids[0], "done").unwrap();
    assert!(db::reminders::set_status(&conn, ids[0], "hacked").is_err());
}

#[test]
fn due_soon_filter() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "x", None).unwrap();
    db::reminders::save_reminders(&conn, &[ReminderExtract { content: "马上要做的".into(), due: Some("今天下午3点".into()) }], cid, fixed_now()).unwrap();
    let now = chrono::NaiveDate::from_ymd_opt(2026, 8, 10).unwrap().and_hms_opt(15, 30, 0).unwrap();
    let soon = db::reminders::reminders_due_soon(&conn, &now.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap();
    assert_eq!(soon.len(), 1);
    assert_eq!(soon[0].content, "马上要做的");
}
