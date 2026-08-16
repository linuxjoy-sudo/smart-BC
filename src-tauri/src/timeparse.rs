use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

/// 解析中文相对时间表达；失败返回 None。
/// 默认时间：今天=18:00，其他日期=09:00。
pub fn parse_due(expr: &str, now: NaiveDateTime) -> Option<NaiveDateTime> {
    let s = expr.trim();
    if s.is_empty() {
        return None;
    }
    // 相对时间：N分钟后/N小时后/N天后/半小时
    if let Some(m) = parse_relative_minutes(s) {
        return now.checked_add_signed(chrono::Duration::minutes(m));
    }
    if let Some(h) = parse_relative_hours(s) {
        return now.checked_add_signed(chrono::Duration::hours(h));
    }
    if let Some(d) = parse_relative_days(s) {
        return now.checked_add_signed(chrono::Duration::days(d));
    }
    let today = now.date();
    let weekday_names = ["一", "二", "三", "四", "五", "六", "日", "天"];

    // 日期基准
    let date = if s.contains("后天") {
        today.succ_opt()?.succ_opt()?
    } else if s.contains("明天") || s.contains("明日") {
        today.succ_opt()?
    } else if s.contains("今天") || s.contains("今日") || s.contains("今晚") {
        today
    } else if s.starts_with("下") && weekday_names.iter().any(|d| {
        s.contains(&format!("周{d}")) || s.contains(&format!("星期{d}"))
    }) {
        let wd = weekday_from_str(s)?;
        next_weekday(today, wd)?
    } else if let Some(day) = weekday_names.iter().find(|d| {
        let pat = format!("周{d}");
        s.contains(&pat) || s.contains(&format!("星期{d}"))
    }) {
        let wd = match *day {
            "一" => Weekday::Mon, "二" => Weekday::Tue, "三" => Weekday::Wed,
            "四" => Weekday::Thu, "五" => Weekday::Fri, "六" => Weekday::Sat,
            _ => Weekday::Sun,
        };
        next_or_today_weekday(today, wd)?
    } else if let Some((m, d)) = parse_md(s) {
        NaiveDate::from_ymd_opt(now.year(), m, d)?
    } else if has_time_expr(s) || s.contains("今天") || s.contains("今日") || s.contains("今晚")
        || s.contains("明天") || s.contains("明日") || s.contains("后天")
    {
        today
    } else {
        return None;
    };

    // 时间
    let time = parse_time(s, date == today, now);
    Some(date.and_time(time))
}

fn parse_relative_minutes(s: &str) -> Option<i64> {
    if s.contains("半小时") {
        return Some(30);
    }
    // 优先"X分钟"，fallback"X分"（如"一分"）；钟点"X点Y分"排除（交给 parse_time）
    for marker in ["分钟", "分"] {
        if let Some(pos) = s.find(marker) {
            let before = &s[..pos];
            if marker == "分" && before.contains('点') {
                continue;
            }
            if let Some(n) = extract_trailing_number(before) {
                return Some(n);
            }
        }
    }
    None
}

/// 从 marker 前提取紧邻数字（阿拉伯多位 / 中文单个）
fn extract_trailing_number(s: &str) -> Option<i64> {
    let chars: Vec<char> = s.chars().rev().collect();
    let mut ascii_digits = String::new();
    for &c in &chars {
        if c.is_ascii_digit() {
            ascii_digits.insert(0, c);
        } else {
            break;
        }
    }
    if !ascii_digits.is_empty() {
        return ascii_digits.parse().ok();
    }
    chars.first().and_then(|&c| chinese_digit(c))
}

fn chinese_digit(c: char) -> Option<i64> {
    match c {
        '一' => Some(1),
        '二' | '两' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        '十' => Some(10),
        _ => None,
    }
}

fn parse_relative_hours(s: &str) -> Option<i64> {
    if s.contains("小时") {
        return extract_number(s);
    }
    None
}

fn parse_relative_days(s: &str) -> Option<i64> {
    if s.contains("天后") {
        return extract_number(s);
    }
    None
}

fn extract_number(s: &str) -> Option<i64> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        return digits.parse().ok();
    }
    match s.chars().find(|c| "一二两三四五六七八九十".contains(*c)) {
        Some('一') => Some(1),
        Some('二') | Some('两') => Some(2),
        Some('三') => Some(3),
        Some('四') => Some(4),
        Some('五') => Some(5),
        Some('六') => Some(6),
        Some('七') => Some(7),
        Some('八') => Some(8),
        Some('九') => Some(9),
        Some('十') => Some(10),
        _ => None,
    }
}

fn weekday_from_str(s: &str) -> Option<Weekday> {
    for (name, wd) in [("一", Weekday::Mon), ("二", Weekday::Tue), ("三", Weekday::Wed),
                       ("四", Weekday::Thu), ("五", Weekday::Fri), ("六", Weekday::Sat),
                       ("日", Weekday::Sun), ("天", Weekday::Sun)] {
        if s.contains(&format!("周{name}")) || s.contains(&format!("星期{name}")) {
            return Some(wd);
        }
    }
    None
}

fn has_time_expr(s: &str) -> bool {
    s.contains('点') || s.contains("上午") || s.contains("下午") || s.contains("晚上")
}

fn next_weekday(from: NaiveDate, wd: Weekday) -> Option<NaiveDate> {
    let mut d = from.succ_opt()?;
    while d.weekday() != wd {
        d = d.succ_opt()?;
    }
    Some(d)
}

fn next_or_today_weekday(from: NaiveDate, wd: Weekday) -> Option<NaiveDate> {
    if from.weekday() == wd {
        return Some(from);
    }
    next_weekday(from, wd)
}

fn parse_md(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.splitn(2, '月').collect();
    if parts.len() != 2 {
        return None;
    }
    let m: u32 = parts[0].parse().ok()?;
    let d_str: String = parts[1].chars().take_while(|c| c.is_ascii_digit()).collect();
    if d_str.is_empty() {
        return None;
    }
    Some((m, d_str.parse().ok()?))
}

/// 提取紧邻数字（阿拉伯多位 / 中文 1-2 字，如"三"=3、"十二"=12），用于钟点/分钟
fn extract_clock_num(s: &str) -> Option<u32> {
    let chars: Vec<char> = s.chars().rev().collect();
    let mut ascii = String::new();
    for &c in &chars {
        if c.is_ascii_digit() {
            ascii.insert(0, c);
        } else {
            break;
        }
    }
    if !ascii.is_empty() {
        return ascii.parse().ok();
    }
    let mut zh = String::new();
    for &c in &chars {
        if "一二两三四五六七八九十".contains(c) {
            zh.insert(0, c);
        } else {
            break;
        }
    }
    zh_num_parse(&zh)
}

fn zh_num_parse(s: &str) -> Option<u32> {
    let d = |c: char| match c {
        '一' | '幺' => Some(1),
        '二' | '两' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    };
    let cs: Vec<char> = s.chars().collect();
    match cs.len() {
        1 => {
            let c = cs[0];
            if c == '十' {
                Some(10)
            } else {
                d(c)
            }
        }
        2 => {
            if cs[0] == '十' {
                d(cs[1]).map(|x| 10 + x)
            } else if cs[1] == '十' {
                d(cs[0]).map(|x| x * 10)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// 提取开头紧邻数字（阿拉伯多位 / 中文 1-2 字），用于"点"后的分钟
fn extract_clock_num_prefix(s: &str) -> Option<u32> {
    let chars: Vec<char> = s.chars().collect();
    let mut ascii = String::new();
    for &c in &chars {
        if c.is_ascii_digit() {
            ascii.push(c);
        } else {
            break;
        }
    }
    if !ascii.is_empty() {
        return ascii.parse().ok();
    }
    let mut zh = String::new();
    for &c in &chars {
        if "一二两三四五六七八九十".contains(c) {
            zh.push(c);
        } else {
            break;
        }
    }
    zh_num_parse(&zh)
}

fn parse_time(s: &str, is_today: bool, now: NaiveDateTime) -> NaiveTime {
    let hour_default = if is_today { 18 } else { 9 };
    let (mut h, m) = if let Some(pos) = s.find("点半") {
        (extract_clock_num(&s[..pos]).unwrap_or(hour_default), 30)
    } else if let Some(pos) = s.find('点') {
        let mut hh = extract_clock_num(&s[..pos]).unwrap_or(hour_default);
        let after: String = s[pos..].chars().skip(1).collect();
        let mm = extract_clock_num_prefix(&after).unwrap_or(0);
        if hh <= 6 && (s.contains("下午") || s.contains("晚上") || s.contains("今晚")) {
            hh += 12;
        }
        (hh, mm)
    } else if s.contains("下午") || s.contains("晚上") {
        (15, 0)
    } else if s.contains("上午") {
        (10, 0)
    } else {
        (hour_default, 0)
    };
    if h == 24 {
        h = 0;
    }
    let h = h.min(23);
    let mm = m.min(59);
    NaiveTime::from_hms_opt(h, mm, 0).unwrap_or(now.time())
}
