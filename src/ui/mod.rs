pub mod dashboard;
pub mod members;
pub mod merchandise;
pub mod expenses;
pub mod settings;
pub mod theme;
pub mod transactions;

use eframe::egui;

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
