use chrono::{Datelike, Days, Local, Months, NaiveDate};

/// Current month as `YYYY-MM`.
pub fn current_month() -> String {
    Local::now().format("%Y-%m").to_string()
}

/// Today as `YYYY-MM-DD`.
pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// `YYYY-MM` for the month containing `date`.
pub fn month_of(date: NaiveDate) -> String {
    format!("{:04}-{:02}", date.year(), date.month())
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
        d = d + Days::new(1);
    }
    out
}

/// Convenience: subtract n months from today, returning ISO string.
pub fn months_ago(n: u32) -> String {
    fmt(today_naive() - Months::new(n))
}
