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

/// Full month name for `m` in 1..=12, else empty.
pub fn month_name(m: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "January", "February", "March", "April", "May", "June", "July", "August", "September",
        "October", "November", "December",
    ];
    NAMES.get((m as usize).wrapping_sub(1)).copied().unwrap_or("")
}

/// A calendar filter: a whole `year`, or one `month` within it (`None` = whole
/// year). More intuitive than rolling windows for "show me July" or "show me
/// 2025". Shared by the Transactions and Merchandise (Sales) views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthFilter {
    pub year: i32,
    pub month: Option<u32>,
}

impl MonthFilter {
    /// Defaults to the current calendar month.
    pub fn current() -> Self {
        let t = today_naive();
        Self {
            year: t.year(),
            month: Some(t.month()),
        }
    }

    /// Inclusive [start, end] `YYYY-MM-DD` for SQL filtering.
    pub fn range(&self) -> (String, String) {
        match self.month {
            Some(m) => {
                let start = NaiveDate::from_ymd_opt(self.year, m, 1).unwrap();
                let end = start + Months::new(1) - Days::new(1);
                (fmt(start), fmt(end))
            }
            None => (
                fmt(NaiveDate::from_ymd_opt(self.year, 1, 1).unwrap()),
                fmt(NaiveDate::from_ymd_opt(self.year, 12, 31).unwrap()),
            ),
        }
    }

    /// The equivalent period one step back: the previous month when a month is
    /// selected (rolling the year over at January), or the previous year when
    /// viewing a whole year. The honest baseline for a "vs last period" trend.
    pub fn previous(&self) -> MonthFilter {
        match self.month {
            Some(1) => MonthFilter { year: self.year - 1, month: Some(12) },
            Some(m) => MonthFilter { year: self.year, month: Some(m - 1) },
            None => MonthFilter { year: self.year - 1, month: None },
        }
    }

    /// Human label, e.g. "July 2026" or "2025".
    pub fn label(&self) -> String {
        match self.month {
            Some(m) => format!("{} {}", month_name(m), self.year),
            None => self.year.to_string(),
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
