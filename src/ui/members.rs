use std::collections::{HashMap, HashSet};

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::core::dates;
use crate::core::models::{Member, MemberStatus, Payment};
use crate::core::Repository;

#[derive(Default)]
struct MemberForm {
    id: i64,
    name: String,
    phone: String,
    join_date: String,
    notes: String,
    active: bool,
    registration_fee_paid: bool,
    editing: bool,
    error: Option<String>,
}

impl MemberForm {
    fn new_member() -> Self {
        Self {
            join_date: dates::today(),
            active: true,
            ..Default::default()
        }
    }
    fn from(m: &Member) -> Self {
        Self {
            id: m.id,
            name: m.name.clone(),
            phone: m.phone.clone().unwrap_or_default(),
            join_date: m.join_date.clone(),
            notes: m.notes.clone().unwrap_or_default(),
            active: m.active,
            registration_fee_paid: m.registration_fee_paid,
            editing: true,
            error: None,
        }
    }
    fn to_member(&self) -> Member {
        Member {
            id: self.id,
            name: self.name.trim().to_string(),
            phone: opt(&self.phone),
            join_date: if self.join_date.trim().is_empty() {
                dates::today()
            } else {
                self.join_date.trim().to_string()
            },
            active: self.active,
            notes: opt(&self.notes),
            registration_fee_paid: self.registration_fee_paid,
        }
    }
}

struct PaymentForm {
    member_id: i64,
    member_name: String,
    amount: String,
    month: String,
    date: String,
    note: String,
    category: String,
}

enum Dialog {
    None,
    Member(MemberForm),
    Payment(PaymentForm),
    Details { member: Member, payments: Vec<Payment> },
    ConfirmDeleteMember { id: i64, name: String },
}

pub struct MembersState {
    search: String,
    show_inactive: bool,
    members: Vec<Member>,
    paid: HashSet<i64>,
    arrears: HashMap<i64, (i64, f64)>,
    loaded: bool,
    dialog: Dialog,
    status: Option<String>,
}

impl Default for MembersState {
    fn default() -> Self {
        Self {
            search: String::new(),
            show_inactive: false,
            members: Vec::new(),
            paid: HashSet::new(),
            arrears: HashMap::new(),
            loaded: false,
            dialog: Dialog::None,
            status: None,
        }
    }
}

fn opt(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// Capitalize the first letter (e.g. `membership` → `Membership`).
fn cap(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// A small rounded pill used for member flags (trial, registration due).
fn badge(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.22))
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.55)))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(color).size(12.0));
        });
}

impl MembersState {
    /// Drop the cached row set so the next `show()` re-queries the DB.
    /// Call this after any external mutation (e.g. CSV import).
    pub fn invalidate(&mut self) {
        self.loaded = false;
    }

    fn reload(&mut self, repo: &Repository) {
        let active_only = !self.show_inactive;
        let q = self.search.trim();
        self.members = if q.is_empty() {
            repo.list_members(active_only)
        } else {
            repo.search_members(q, active_only)
        }
        .unwrap_or_default();
        let month = dates::current_month();
        self.paid = repo.paid_member_ids(&month).unwrap_or_default();
        let mut arrears = HashMap::new();
        for m in &self.members {
            if m.active && !self.paid.contains(&m.id) {
                if let Ok(a) = repo.membership_arrears(m.id, &m.join_date[..7], &month) {
                    arrears.insert(m.id, a);
                }
            }
        }
        self.arrears = arrears;
        self.loaded = true;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) {
        if !self.loaded {
            self.reload(repo);
        }

        ui.horizontal(|ui| {
            ui.heading("Members");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ Add member").clicked() {
                    self.dialog = Dialog::Member(MemberForm::new_member());
                }
            });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Search:");
            if ui
                .add(egui::TextEdit::singleline(&mut self.search).hint_text("name or phone"))
                .changed()
            {
                self.reload(repo);
            }
            if ui
                .checkbox(&mut self.show_inactive, "Show inactive (quit)")
                .changed()
            {
                self.reload(repo);
            }
            ui.label(
                egui::RichText::new(format!("{} shown", self.members.len())).weak(),
            );
        });

        ui.add_space(6.0);
        ui.separator();
        if let Some(s) = &self.status {
            ui.add_space(4.0);
            ui.colored_label(egui::Color32::from_rgb(210, 90, 90), s);
        }

        let month = dates::current_month();
        let currency = repo.currency();
        let mut action: Option<Action> = None;

        if self.members.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                if self.search.trim().is_empty() {
                    ui.label(egui::RichText::new("No members yet.").weak());
                    ui.add_space(6.0);
                    if ui.button("+ Add your first member").clicked() {
                        self.dialog = Dialog::Member(MemberForm::new_member());
                    }
                } else {
                    ui.label(egui::RichText::new("No members match your search.").weak());
                }
            });
        } else {
        let row_height = 34.0;
        TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(160.0).clip(true)) // name
            .column(Column::auto().at_least(110.0)) // phone
            .column(Column::auto().at_least(80.0)) // status
            .column(Column::auto().at_least(140.0)) // flags
            .column(Column::auto().at_least(100.0)) // join
            .column(Column::remainder().at_least(220.0)) // actions
            .header(30.0, |mut h| {
                h.col(|ui| { ui.strong("Name"); });
                h.col(|ui| { ui.strong("Phone"); });
                h.col(|ui| { ui.strong("Status"); });
                h.col(|ui| { ui.strong("Flags"); });
                h.col(|ui| { ui.strong("Joined"); });
                h.col(|ui| { ui.strong("Actions"); });
            })
            .body(|body| {
                body.rows(row_height, self.members.len(), |mut row| {
                    let m = &self.members[row.index()];
                    let status = if !m.active {
                        MemberStatus::Inactive
                    } else if self.paid.contains(&m.id) {
                        MemberStatus::Paid
                    } else {
                        MemberStatus::Due
                    };
                    row.col(|ui| { ui.label(&m.name); });
                    row.col(|ui| { ui.label(m.phone.clone().unwrap_or_default()); });
                    row.col(|ui| {
                        match status {
                            MemberStatus::Paid => {
                                ui.colored_label(egui::Color32::from_rgb(40, 170, 90), "Paid");
                            }
                            MemberStatus::Due => {
                                let behind =
                                    self.arrears.get(&m.id).map(|(n, _)| *n).unwrap_or(0);
                                let label = if behind >= 2 {
                                    format!("Due · {behind}m")
                                } else {
                                    "Due".to_string()
                                };
                                let color = if behind >= 3 {
                                    egui::Color32::from_rgb(200, 70, 70)
                                } else {
                                    egui::Color32::from_rgb(210, 120, 40)
                                };
                                ui.colored_label(color, label);
                            }
                            MemberStatus::Inactive => {
                                ui.colored_label(egui::Color32::GRAY, "Inactive");
                            }
                        }
                    });
                    row.col(|ui| {
                        if !m.registration_fee_paid {
                            badge(ui, "Reg due", egui::Color32::from_rgb(200, 110, 50));
                        }
                    });
                    row.col(|ui| { ui.label(&m.join_date); });
                    row.col(|ui| {
                        if m.active && status == MemberStatus::Due {
                            if ui.small_button("Record payment").clicked() {
                                action = Some(Action::OpenPayment(m.id));
                            }
                        }
                        if ui.small_button("Details").clicked() {
                            action = Some(Action::Details(m.id));
                        }
                        if ui.small_button("Edit").clicked() {
                            action = Some(Action::Edit(m.id));
                        }
                        let toggle = if m.active { "Deactivate" } else { "Reactivate" };
                        if ui.small_button(toggle).clicked() {
                            action = Some(Action::ToggleActive(m.id, !m.active));
                        }
                        if ui.small_button("Delete").clicked() {
                            action = Some(Action::AskDelete(m.id, m.name.clone()));
                        }
                    });
                });
            });
        }

        if let Some(a) = action {
            self.handle_action(a, repo);
        }

        self.show_dialog(ui.ctx(), repo, &month, &currency);
    }

    fn handle_action(&mut self, a: Action, repo: &mut Repository) {
        match a {
            Action::OpenPayment(id) => {
                if let Ok(Some(m)) = repo.get_member(id) {
                    self.dialog = Dialog::Payment(PaymentForm {
                        member_id: m.id,
                        member_name: m.name,
                        amount: format!("{}", repo.default_monthly_fee()),
                        month: dates::current_month(),
                        date: dates::today(),
                        note: String::new(),
                        category: "membership".to_string(),
                    });
                }
            }
            Action::Edit(id) => {
                if let Ok(Some(m)) = repo.get_member(id) {
                    self.dialog = Dialog::Member(MemberForm::from(&m));
                }
            }
            Action::ToggleActive(id, active) => {
                self.status = match repo.set_member_active(id, active) {
                    Ok(()) => None,
                    Err(e) => Some(format!("Couldn't update member — {e}")),
                };
                self.reload(repo);
            }
            Action::Details(id) => {
                if let Ok(Some(m)) = repo.get_member(id) {
                    let payments = repo.payments_for_member(id).unwrap_or_default();
                    self.dialog = Dialog::Details { member: m, payments };
                }
            }
            Action::AskDelete(id, name) => {
                self.dialog = Dialog::ConfirmDeleteMember { id, name };
            }
        }
    }

    fn show_dialog(
        &mut self,
        ctx: &egui::Context,
        repo: &mut Repository,
        month: &str,
        currency: &str,
    ) {
        let mut close = false;
        let mut open_payment_for: Option<(i64, f64, String)> = None;
        let mut status_update: Option<Option<String>> = None;
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::Member(form) => {
                let title = if form.editing { "Edit member" } else { "Add member" };
                let reg_fee = repo.registration_fee();
                egui::Window::new(title)
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        egui::Grid::new("member_form").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                            ui.label("Name *");
                            ui.text_edit_singleline(&mut form.name);
                            ui.end_row();
                            ui.label("Phone");
                            ui.text_edit_singleline(&mut form.phone);
                            ui.end_row();
                            ui.label("Join date");
                            crate::ui::date_edit(ui, &mut form.join_date);
                            ui.end_row();
                            ui.label("Registration fee");
                            ui.checkbox(
                                &mut form.registration_fee_paid,
                                format!("Paid ({} {:.0})", currency, reg_fee),
                            );
                            ui.end_row();
                            ui.label("Notes");
                            ui.text_edit_multiline(&mut form.notes);
                            ui.end_row();
                        });
                        ui.add_space(6.0);
                        let name_ok = !form.name.trim().is_empty();
                        let date_ok = form.join_date.trim().is_empty()
                            || dates::is_valid_date(&form.join_date);
                        let valid = name_ok && date_ok;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                                let m = form.to_member();
                                if form.editing {
                                    match repo.update_member(&m) {
                                        Ok(()) => {
                                            if form.registration_fee_paid {
                                                let _ = repo.ensure_registration_payment(
                                                    m.id,
                                                    reg_fee,
                                                    &m.join_date,
                                                    &m.join_date[..7],
                                                );
                                            }
                                            close = true;
                                        }
                                        Err(e) => form.error = Some(format!("Save failed: {e}")),
                                    }
                                } else {
                                    match repo.insert_member(&m) {
                                        Ok(id) => {
                                            if form.registration_fee_paid {
                                                let _ = repo.ensure_registration_payment(
                                                    id,
                                                    reg_fee,
                                                    &m.join_date,
                                                    &m.join_date[..7],
                                                );
                                            }
                                            open_payment_for = Some((
                                                id,
                                                repo.default_monthly_fee(),
                                                "membership".to_string(),
                                            ));
                                        }
                                        Err(e) => form.error = Some(format!("Save failed: {e}")),
                                    }
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                            if !name_ok {
                                ui.colored_label(egui::Color32::from_rgb(210, 120, 40), "Name required");
                            } else if !date_ok {
                                ui.colored_label(egui::Color32::from_rgb(210, 120, 40), "Date must be YYYY-MM-DD");
                            }
                        });
                        if let Some(err) = &form.error {
                            ui.add_space(4.0);
                            ui.colored_label(egui::Color32::from_rgb(210, 90, 90), err);
                        }
                    });
            }
            Dialog::Payment(form) => {
                egui::Window::new("Record payment")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(format!("Member: {}", form.member_name));
                        egui::Grid::new("pay_form").num_columns(2).show(ui, |ui| {
                            ui.label(format!("Amount ({currency})"));
                            ui.text_edit_singleline(&mut form.amount);
                            ui.end_row();
                            ui.label("Month");
                            crate::ui::month_edit(ui, &mut form.month);
                            ui.end_row();
                            ui.label("Date");
                            crate::ui::date_edit(ui, &mut form.date);
                            ui.end_row();
                            ui.label("Note");
                            ui.text_edit_singleline(&mut form.note);
                            ui.end_row();
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let amt_ok = form.amount.trim().parse::<f64>().is_ok();
                            let month_ok = dates::is_valid_month(&form.month);
                            let date_ok = dates::is_valid_date(&form.date);
                            let pay_ok = amt_ok && month_ok && date_ok;
                            if ui.add_enabled(pay_ok, egui::Button::new("Save payment")).clicked() {
                                let p = Payment {
                                    id: 0,
                                    member_id: form.member_id,
                                    period_month: form.month.trim().to_string(),
                                    amount: form.amount.trim().parse().unwrap_or(0.0),
                                    date: form.date.trim().to_string(),
                                    note: opt(&form.note),
                                    category: form.category.clone(),
                                };
                                match repo.insert_payment(&p) {
                                    Ok(_) => {
                                        status_update = Some(None);
                                        close = true;
                                    }
                                    Err(e) => {
                                        status_update =
                                            Some(Some(format!("Couldn't save payment — {e}")));
                                    }
                                }
                            }
                            if ui.button("Skip").clicked() {
                                close = true;
                            }
                            if !month_ok {
                                ui.colored_label(egui::Color32::from_rgb(210, 120, 40), "Month should look like 2026-07");
                            }
                        });
                    });
            }
            Dialog::Details { member, payments } => {
                let m = &*member;
                let status = if !m.active {
                    MemberStatus::Inactive
                } else if self.paid.contains(&m.id) {
                    MemberStatus::Paid
                } else {
                    MemberStatus::Due
                };
                let total_paid: f64 = payments.iter().map(|p| p.amount).sum();
                egui::Window::new(format!("Member — {}", m.name))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.set_min_width(420.0);
                        egui::Grid::new("member_details")
                            .num_columns(2)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Name").weak());
                                ui.label(&m.name);
                                ui.end_row();
                                ui.label(egui::RichText::new("Phone").weak());
                                ui.label(m.phone.as_deref().unwrap_or("—"));
                                ui.end_row();
                                ui.label(egui::RichText::new("Joined").weak());
                                ui.label(&m.join_date);
                                ui.end_row();
                                ui.label(egui::RichText::new("Status").weak());
                                let (txt, color) = match status {
                                    MemberStatus::Paid => ("Paid (this month)", egui::Color32::from_rgb(40, 170, 90)),
                                    MemberStatus::Due => ("Due (this month)", egui::Color32::from_rgb(210, 120, 40)),
                                    MemberStatus::Inactive => ("Inactive", egui::Color32::GRAY),
                                };
                                ui.colored_label(color, txt);
                                ui.end_row();
                                ui.label(egui::RichText::new("Reg. fee").weak());
                                ui.label(if m.registration_fee_paid { "Paid" } else { "Due" });
                                ui.end_row();
                                ui.label(egui::RichText::new("Total paid").weak());
                                ui.label(format!("{} {:.0}", currency, total_paid));
                                ui.end_row();
                                ui.label(egui::RichText::new("Notes").weak());
                                ui.label(m.notes.as_deref().unwrap_or("—"));
                                ui.end_row();
                            });

                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!("Payment history ({})", payments.len()))
                                .strong(),
                        );
                        ui.add_space(4.0);
                        ui.separator();
                        if payments.is_empty() {
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("No payments recorded yet.").weak());
                        } else {
                            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                egui::Grid::new("pay_history")
                                    .num_columns(4)
                                    .spacing([14.0, 6.0])
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Date").weak().size(11.0));
                                        ui.label(egui::RichText::new("Period").weak().size(11.0));
                                        ui.label(egui::RichText::new("Category").weak().size(11.0));
                                        ui.label(egui::RichText::new("Amount").weak().size(11.0));
                                        ui.end_row();
                                        for p in payments.iter() {
                                            ui.label(&p.date);
                                            ui.label(&p.period_month);
                                            ui.label(cap(&p.category));
                                            ui.label(format!("{} {:.0}", currency, p.amount));
                                            ui.end_row();
                                        }
                                    });
                            });
                        }

                        ui.add_space(10.0);
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
            }
            Dialog::ConfirmDeleteMember { id, name } => {
                let id = *id;
                egui::Window::new("Delete member?")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(format!("Delete \"{name}\" and all their payments? This cannot be undone."));
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::Button::new("Delete").fill(egui::Color32::from_rgb(170, 50, 50)))
                                .clicked()
                            {
                                status_update = match repo.delete_member(id) {
                                    Ok(()) => Some(None),
                                    Err(e) => Some(Some(format!("Couldn't delete member — {e}"))),
                                };
                                close = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                        });
                    });
            }
        }

        if let Some(u) = status_update {
            self.status = u;
        }

        if let Some((id, amt, cat)) = open_payment_for {
            if let Ok(Some(m)) = repo.get_member(id) {
                self.dialog = Dialog::Payment(PaymentForm {
                    member_id: m.id,
                    member_name: m.name,
                    amount: format!("{}", amt),
                    month: month.to_string(),
                    date: dates::today(),
                    note: String::new(),
                    category: cat,
                });
            }
            self.reload(repo);
        } else if close {
            self.dialog = Dialog::None;
            self.reload(repo);
        }
    }
}

enum Action {
    OpenPayment(i64),
    Details(i64),
    Edit(i64),
    ToggleActive(i64, bool),
    AskDelete(i64, String),
}

