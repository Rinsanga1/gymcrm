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

/// A `YYYY-MM` text field with a "This month" shortcut button beside it.
pub fn month_edit(ui: &mut egui::Ui, value: &mut String) {
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(value).desired_width(90.0));
        if ui.small_button("This month").clicked() {
            *value = crate::core::dates::current_month();
        }
    });
}
