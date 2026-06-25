use std::collections::HashSet;

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
    editing: bool,
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
            editing: true,
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
}

enum Dialog {
    None,
    Member(MemberForm),
    Payment(PaymentForm),
    ConfirmDeleteMember { id: i64, name: String },
}

pub struct MembersState {
    search: String,
    show_inactive: bool,
    members: Vec<Member>,
    paid: HashSet<i64>,
    loaded: bool,
    dialog: Dialog,
}

impl Default for MembersState {
    fn default() -> Self {
        Self {
            search: String::new(),
            show_inactive: false,
            members: Vec::new(),
            paid: HashSet::new(),
            loaded: false,
            dialog: Dialog::None,
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
        self.paid = repo.paid_member_ids(&dates::current_month()).unwrap_or_default();
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

        let month = dates::current_month();
        let currency = repo.currency();
        let mut action: Option<Action> = None;

        let row_height = 26.0;
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(160.0).clip(true)) // name
            .column(Column::auto().at_least(110.0)) // phone
            .column(Column::auto().at_least(80.0)) // status
            .column(Column::auto().at_least(100.0)) // join
            .column(Column::remainder().at_least(220.0)) // actions
            .header(22.0, |mut h| {
                h.col(|ui| { ui.strong("Name"); });
                h.col(|ui| { ui.strong("Phone"); });
                h.col(|ui| { ui.strong("Status"); });
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
                        let (txt, color) = match status {
                            MemberStatus::Paid => ("Paid", egui::Color32::from_rgb(40, 170, 90)),
                            MemberStatus::Due => ("Due", egui::Color32::from_rgb(210, 120, 40)),
                            MemberStatus::Inactive => ("Inactive", egui::Color32::GRAY),
                        };
                        ui.colored_label(color, txt);
                    });
                    row.col(|ui| { ui.label(&m.join_date); });
                    row.col(|ui| {
                        if m.active && status == MemberStatus::Due {
                            if ui.small_button("Record payment").clicked() {
                                action = Some(Action::OpenPayment(m.id));
                            }
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
                    });
                }
            }
            Action::Edit(id) => {
                if let Ok(Some(m)) = repo.get_member(id) {
                    self.dialog = Dialog::Member(MemberForm::from(&m));
                }
            }
            Action::ToggleActive(id, active) => {
                let _ = repo.set_member_active(id, active);
                self.reload(repo);
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
        let mut open_payment_for: Option<i64> = None;
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::Member(form) => {
                let title = if form.editing { "Edit member" } else { "Add member" };
                egui::Window::new(title)
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        egui::Grid::new("member_form").num_columns(2).show(ui, |ui| {
                            ui.label("Name *");
                            ui.text_edit_singleline(&mut form.name);
                            ui.end_row();
                            ui.label("Phone");
                            ui.text_edit_singleline(&mut form.phone);
                            ui.end_row();
                            ui.label("Join date");
                            ui.text_edit_singleline(&mut form.join_date);
                            ui.end_row();
                            ui.label("Notes");
                            ui.text_edit_multiline(&mut form.notes);
                            ui.end_row();
                        });
                        ui.add_space(6.0);
                        let valid = !form.name.trim().is_empty();
                        ui.horizontal(|ui| {
                            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                                let m = form.to_member();
                                if form.editing {
                                    let _ = repo.update_member(&m);
                                    close = true;
                                } else {
                                    if let Ok(id) = repo.insert_member(&m) {
                                        open_payment_for = Some(id);
                                    }
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                            if !valid {
                                ui.colored_label(egui::Color32::from_rgb(210, 120, 40), "Name required");
                            }
                        });
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
                            ui.label("Month (YYYY-MM)");
                            ui.text_edit_singleline(&mut form.month);
                            ui.end_row();
                            ui.label("Date");
                            ui.text_edit_singleline(&mut form.date);
                            ui.end_row();
                            ui.label("Note");
                            ui.text_edit_singleline(&mut form.note);
                            ui.end_row();
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let amt_ok = form.amount.trim().parse::<f64>().is_ok();
                            if ui.add_enabled(amt_ok, egui::Button::new("Save payment")).clicked() {
                                let p = Payment {
                                    id: 0,
                                    member_id: form.member_id,
                                    period_month: form.month.trim().to_string(),
                                    amount: form.amount.trim().parse().unwrap_or(0.0),
                                    date: form.date.trim().to_string(),
                                    note: opt(&form.note),
                                };
                                let _ = repo.insert_payment(&p);
                                close = true;
                            }
                            if ui.button("Skip").clicked() {
                                close = true;
                            }
                        });
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
                                let _ = repo.delete_member(id);
                                close = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                        });
                    });
            }
        }

        if let Some(id) = open_payment_for {
            if let Ok(Some(m)) = repo.get_member(id) {
                self.dialog = Dialog::Payment(PaymentForm {
                    member_id: m.id,
                    member_name: m.name,
                    amount: format!("{}", repo.default_monthly_fee()),
                    month: month.to_string(),
                    date: dates::today(),
                    note: String::new(),
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
    Edit(i64),
    ToggleActive(i64, bool),
    AskDelete(i64, String),
}

