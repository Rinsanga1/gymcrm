use eframe::egui;

use crate::core::dates;
use crate::core::models::Payment;

/// One record-payment form, shared by the Members list and the Dashboard so the
/// two entry points can't drift apart.
pub struct PaymentForm {
    pub member_id: i64,
    pub member_name: String,
    pub amount: String,
    pub month: String,
    pub date: String,
    pub note: String,
    pub category: String,
    pub error: Option<String>,
}

impl PaymentForm {
    pub fn new(member_id: i64, member_name: String, amount: f64, month: String) -> Self {
        Self {
            member_id,
            member_name,
            amount: format!("{}", amount),
            month,
            date: dates::today(),
            note: String::new(),
            category: "membership".to_string(),
            error: None,
        }
    }

    pub fn to_payment(&self) -> Payment {
        let note = self.note.trim();
        Payment {
            id: 0,
            member_id: self.member_id,
            period_month: self.month.trim().to_string(),
            amount: self.amount.trim().parse().unwrap_or(0.0),
            date: self.date.trim().to_string(),
            note: if note.is_empty() { None } else { Some(note.to_string()) },
            category: self.category.clone(),
        }
    }
}

pub enum Outcome {
    Open,
    Save,
    Cancel,
}

/// Renders the modal and reports what the user did. The caller owns persistence.
pub fn show(ctx: &egui::Context, form: &mut PaymentForm, currency: &str) -> Outcome {
    let mut outcome = Outcome::Open;
    egui::Window::new(format!("Record payment — {}", form.member_name))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Grid::new("payment_form")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label(format!("Amount ({currency})"));
                    ui.text_edit_singleline(&mut form.amount);
                    ui.end_row();
                    ui.label("Month");
                    crate::ui::month_dropdown(ui, "payment_month", &mut form.month);
                    ui.end_row();
                    ui.label("Date");
                    crate::ui::date_edit(ui, &mut form.date);
                    ui.end_row();
                    ui.label("Note");
                    ui.text_edit_singleline(&mut form.note);
                    ui.end_row();
                });
            ui.add_space(6.0);
            let amt_ok = form.amount.trim().parse::<f64>().is_ok();
            let month_ok = dates::is_valid_month(&form.month);
            let date_ok = dates::is_valid_date(&form.date);
            let valid = amt_ok && month_ok && date_ok;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(valid, egui::Button::new("Save payment"))
                    .clicked()
                {
                    outcome = Outcome::Save;
                }
                if ui.button("Cancel").clicked() {
                    outcome = Outcome::Cancel;
                }
                if !amt_ok {
                    ui.colored_label(egui::Color32::from_rgb(210, 120, 40), "Enter an amount");
                }
            });
            if let Some(err) = &form.error {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::from_rgb(210, 90, 90), err);
            }
        });
    outcome
}
