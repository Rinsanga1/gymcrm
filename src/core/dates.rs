use chrono::{Datelike, Days, Local, Months, NaiveDate};

/// Current month as `YYYY-MM`.
pub fn current_month() -> String {
    Local::now().format("%Y-%m").to_string()
}

/// Today as `YYYY-MM-DD`.
pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Current local timestamp as `YYYY-MM-DD HH:MM:SS`, used to order same-day
/// transactions by the moment they were recorded.
pub fn now() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// `YYYY-MM` → e.g. `June 2026`. Falls back to the input if unparseable.
pub fn pretty_month(ym: &str) -> String {
    if let Some((y, m)) = ym.split_once('-') {
        if let (Ok(y), Ok(m)) = (y.parse::<i32>(), m.parse::<u32>()) {
            if let Some(d) = NaiveDate::from_ymd_opt(y, m, 1) {
                return d.format("%B %Y").to_string();
            }
        }
    }
    ym.to_string()
}

fn today_naive() -> NaiveDate {
    Local::now().date_naive()
}

fn fmt(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Predefined dashboard time windows. `Custom` carries inclusive [start, end] strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Period {
    AllTime,
    Today,
    ThisWeek,
    ThisMonth,
    ThisQuarter,
    ThisYear,
    Custom { start: String, end: String },
}

impl Period {
    pub fn label(&self) -> &'static str {
        match self {
            Period::AllTime => "All Time",
            Period::Today => "Today",
            Period::ThisWeek => "This Week",
            Period::ThisMonth => "This Month",
            Period::ThisQuarter => "This Quarter",
            Period::ThisYear => "This Year",
            Period::Custom { .. } => "Custom",
        }
    }

    /// Inclusive [start, end] date strings (YYYY-MM-DD) for SQL filtering.
    /// `AllTime` uses a very wide range so callers can pass the same params.
    pub fn range(&self) -> (String, String) {
        let today = today_naive();
        match self {
            Period::AllTime => ("0000-01-01".into(), "9999-12-31".into()),
            Period::Today => (fmt(today), fmt(today)),
            Period::ThisWeek => {
                let weekday = today.weekday().num_days_from_monday();
                let start = today - Days::new(weekday as u64);
                (fmt(start), fmt(today))
            }
            Period::ThisMonth => {
                let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
                (fmt(start), fmt(today))
            }
            Period::ThisQuarter => {
                let q_start_month = ((today.month() - 1) / 3) * 3 + 1;
                let start = NaiveDate::from_ymd_opt(today.year(), q_start_month, 1).unwrap();
                (fmt(start), fmt(today))
            }
            Period::ThisYear => {
                let start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
                (fmt(start), fmt(today))
            }
            Period::Custom { start, end } => (start.clone(), end.clone()),
        }
    }
}

/// Inclusive iterator of day-by-day dates for a [start, end] string range.
/// Returns empty if either parse fails or end < start.
pub fn days_inclusive(start: &str, end: &str) -> Vec<NaiveDate> {
    let (Ok(s), Ok(e)) = (
        NaiveDate::parse_from_str(start, "%Y-%m-%d"),
        NaiveDate::parse_from_str(end, "%Y-%m-%d"),
    ) else {
        return Vec::new();
    };
    if e < s {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut d = s;
    while d <= e {
        out.push(d);
        match d.checked_add_days(Days::new(1)) {
            Some(next) => d = next,
            None => break, // hit NaiveDate::MAX
        }
    }
    out
}

/// Whole months from `from` to `to`, both `YYYY-MM`. Same month = 0,
/// later `to` = positive, earlier `to` = negative. Unparseable inputs = 0.
pub fn month_diff(from: &str, to: &str) -> i64 {
    fn parse(s: &str) -> Option<(i64, i64)> {
        let (y, m) = s.trim().split_once('-')?;
        Some((y.parse().ok()?, m.parse().ok()?))
    }
    match (parse(from), parse(to)) {
        (Some((y1, m1)), Some((y2, m2))) => (y2 - y1) * 12 + (m2 - m1),
        _ => 0,
    }
}

/// Convenience: subtract n months from today, returning ISO string.
pub fn months_ago(n: u32) -> String {
    fmt(today_naive() - Months::new(n))
}

/// `YYYY-MM-DD` for `n` days before today — a rolling window's start.
pub fn days_ago(n: u64) -> String {
    fmt(today_naive() - Days::new(n))
}

/// `YYYY-MM-DD` → `Jul 08` for a human-scannable ledger. The year is redundant
/// under a month header, so it's dropped. Falls back to the raw string.
pub fn short_date(s: &str) -> String {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map(|d| d.format("%b %d").to_string())
        .unwrap_or_else(|_| s.to_string())
}

/// Inclusive `YYYY-MM` months from `from` to `to` (both `YYYY-MM`), oldest
/// first. Empty if either is unparseable or `to` is before `from`.
pub fn months_between(from: &str, to: &str) -> Vec<String> {
    fn first_of(s: &str) -> Option<NaiveDate> {
        let (y, m) = s.trim().split_once('-')?;
        NaiveDate::from_ymd_opt(y.parse().ok()?, m.parse().ok()?, 1)
    }
    let (Some(a), Some(b)) = (first_of(from), first_of(to)) else {
        return Vec::new();
    };
    if b < a {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut d = a;
    while d <= b {
        out.push(d.format("%Y-%m").to_string());
        d = d + Months::new(1);
    }
    out
}

/// `YYYY-MM` months for a picker: `forward` months ahead down to `back` months
/// behind the current month, newest first. The current month sits near the top
/// so the common case is a short reach.
pub fn month_options(back: u32, forward: u32) -> Vec<String> {
    let today = today_naive();
    let base = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let mut out = Vec::with_capacity((back + forward + 1) as usize);
    for i in 0..=(back + forward) {
        let offset = forward as i64 - i as i64; // +forward .. -back
        let d = if offset >= 0 {
            base + Months::new(offset as u32)
        } else {
            base - Months::new((-offset) as u32)
        };
        out.push(d.format("%Y-%m").to_string());
    }
    out
}

/// True if `s` is a real calendar date in `YYYY-MM-DD` form.
pub fn is_valid_date(s: &str) -> bool {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").is_ok()
}

/// True if `s` is a `YYYY-MM` month with a month in 01..=12.
pub fn is_valid_month(s: &str) -> bool {
    let s = s.trim();
    let Some((y, m)) = s.split_once('-') else {
        return false;
    };
    let (Ok(_), Ok(mm)) = (y.parse::<i32>(), m.parse::<u32>()) else {
        return false;
    };
    y.len() == 4 && m.len() == 2 && (1..=12).contains(&mm)
}
