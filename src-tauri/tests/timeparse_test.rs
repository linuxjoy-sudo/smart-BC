use chrono::NaiveDate;
use smart_bc::timeparse::parse_due;

fn now() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 8, 10).unwrap().and_hms_opt(9, 0, 0).unwrap() // 周一 09:00
}

#[test]
fn today_and_tomorrow() {
    assert_eq!(parse_due("今天", now()), Some(now().date().and_hms_opt(18, 0, 0).unwrap()));
    assert_eq!(parse_due("明天", now()), Some(now().date().succ_opt().unwrap().and_hms_opt(9, 0, 0).unwrap()));
    assert_eq!(parse_due("后天", now()), Some(now().date().succ_opt().unwrap().succ_opt().unwrap().and_hms_opt(9, 0, 0).unwrap()));
}

#[test]
fn weekday_this_and_next_week() {
    // 2026-08-10 是周一
    assert_eq!(parse_due("周三", now()), Some(NaiveDate::from_ymd_opt(2026, 8, 12).unwrap().and_hms_opt(9, 0, 0).unwrap()));
    assert_eq!(parse_due("下周一", now()), Some(NaiveDate::from_ymd_opt(2026, 8, 17).unwrap().and_hms_opt(9, 0, 0).unwrap()));
}

#[test]
fn hour_expressions() {
    assert_eq!(parse_due("下午3点", now()), Some(now().date().and_hms_opt(15, 0, 0).unwrap()));
    assert_eq!(parse_due("10点半", now()), Some(now().date().and_hms_opt(10, 30, 0).unwrap()));
}

#[test]
fn date_expressions() {
    assert_eq!(parse_due("8月15日", now()), Some(NaiveDate::from_ymd_opt(2026, 8, 15).unwrap().and_hms_opt(9, 0, 0).unwrap()));
}

#[test]
fn unsupported_returns_none() {
    assert_eq!(parse_due("尽快", now()), None);
    assert_eq!(parse_due("", now()), None);
}

#[test]
fn relative_minutes_hours_days() {
    let base = now();
    assert_eq!(parse_due("1分钟后", base), base.checked_add_signed(chrono::Duration::minutes(1)));
    assert_eq!(parse_due("10分钟", base), base.checked_add_signed(chrono::Duration::minutes(10)));
    assert_eq!(parse_due("半小时后", base), base.checked_add_signed(chrono::Duration::minutes(30)));
    assert_eq!(parse_due("2小时后", base), base.checked_add_signed(chrono::Duration::hours(2)));
    assert_eq!(parse_due("3天后", base), base.checked_add_signed(chrono::Duration::days(3)));
}
