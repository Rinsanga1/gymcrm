pub mod dashboard;
pub mod members;
pub mod merchandise;
pub mod expenses;
pub mod settings;
pub mod theme;
pub mod transactions;

use eframe::egui;

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
