pub mod dashboard;
pub mod members;
pub mod merchandise;
pub mod expenses;
pub mod settings;
pub mod theme;
pub mod transactions;

use eframe::egui;

/// The one place money reads on screen: `Rs 1,500` — currency prefix, grouped
/// thousands, no decimals (a rupee gym doesn't bill paise). Every figure in the
/// app routes through here so they all match and all honor the currency setting.
pub fn money(currency: &str, amount: f64) -> String {
    let n = amount.round().abs() as u64;
    let mut digits = n.to_string();
    let mut grouped = String::new();
    while digits.len() > 3 {
        let split = digits.len() - 3;
        grouped = format!(",{}{}", &digits[split..], grouped);
        digits.truncate(split);
    }
    grouped = format!("{digits}{grouped}");
    let sign = if amount.round() < 0.0 { "-" } else { "" };
    format!("{sign}{currency} {grouped}")
}

/// Keep a wide table usable on small screens: it fills the panel when there is
/// room and scrolls horizontally when the columns don't fit. egui_extras tables
/// have no horizontal scroll of their own, so without this the rightmost column
/// (usually Actions) clips off the edge with no way to reach it. `min_width` is
/// the table's natural width — the sum of its column minimums.
pub fn wide_table(ui: &mut egui::Ui, min_width: f32, add: impl FnOnce(&mut egui::Ui)) {
    let target = min_width.max(ui.available_width());
    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(target);
            add(ui);
        });
}

/// Year + month dropdowns for a calendar filter. `years` is the selectable year
/// list (newest first); the current year is always included so a fresh database
/// still has a valid choice. Month offers "All months" plus January–December.
/// Returns true when the selection changed. `salt` keeps the two combos unique.
pub fn year_month_filter(
    ui: &mut egui::Ui,
    salt: &str,
    filter: &mut crate::core::dates::MonthFilter,
    years: &[i32],
) -> bool {
    use crate::core::dates::month_name;
    let before = *filter;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt((salt, "year"))
            .selected_text(filter.year.to_string())
            .show_ui(ui, |ui| {
                for &y in years {
                    ui.selectable_value(&mut filter.year, y, y.to_string());
                }
            });
        egui::ComboBox::from_id_salt((salt, "month"))
            .selected_text(match filter.month {
                Some(m) => month_name(m).to_string(),
                None => "All months".to_string(),
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut filter.month, None, "All months");
                for m in 1..=12u32 {
                    ui.selectable_value(&mut filter.month, Some(m), month_name(m));
                }
            });
    });
    *filter != before
}

/// The selectable year list for a filter: the years present in the data plus
/// the current year, newest first, guaranteeing `also` (the current selection)
/// is present so the dropdown can always show it.
pub fn year_options(mut years: Vec<i32>, also: i32) -> Vec<i32> {
    let this_year: i32 = crate::core::dates::today()[..4].parse().unwrap_or(also);
    for y in [this_year, also] {
        if !years.contains(&y) {
            years.push(y);
        }
    }
    years.sort_unstable_by(|a, b| b.cmp(a));
    years
}

/// A `YYYY-MM-DD` text field with a "Today" shortcut button beside it, so the
/// common case (today) is one click instead of typing a full date.
pub fn date_edit(ui: &mut egui::Ui, value: &mut String) {
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(value).desired_width(110.0));
        if ui.small_button("Today").clicked() {
            *value = crate::core::dates::today();
        }
    });
}

/// A month picker dropdown (`YYYY-MM`). Since billing is monthly, a dropdown of
/// real months beats typing a date. `salt` must be unique per simultaneously
/// rendered dropdown.
pub fn month_dropdown(ui: &mut egui::Ui, salt: &str, value: &mut String) {
    use crate::core::dates;
    let mut opts = dates::month_options(24, 1);
    // Keep an out-of-window value (e.g. editing an old payment) selectable.
    if dates::is_valid_month(value) && !opts.iter().any(|m| m == value) {
        opts.insert(0, value.clone());
    }
    egui::ComboBox::from_id_salt(salt)
        .selected_text(dates::pretty_month(value))
        .show_ui(ui, |ui| {
            for m in &opts {
                ui.selectable_value(value, m.clone(), dates::pretty_month(m));
            }
        });
}
