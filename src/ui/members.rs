use std::collections::HashMap;

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
        }
    }
}

/// One month row in the per-member payments editor. `amount` holds the money
/// received (empty = none); `covered` marks a month settled without money — a
/// prepayment or comp, stored as a zero-amount payment. `original` is the
/// existing membership payment for the month, if any.
struct MonthEntry {
    month: String,
    label: String,
    amount: String,
    covered: bool,
    original: Option<Payment>,
}

const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August",
    "September", "October", "November", "December",
];

/// A simple year-at-a-glance payment book: pick a year, see Jan–Dec and what was
/// paid each month. Switching years reloads from the last saved state.
struct PaymentsEditor {
    member_id: i64,
    member_name: String,
    payments: Vec<Payment>,
    year: i32,
    min_year: i32,
    max_year: i32,
    entries: Vec<MonthEntry>,
    reg_paid: bool,
    reg_fee: f64,
    error: Option<String>,
}

impl PaymentsEditor {
    fn new(m: &Member, payments: &[Payment], reg_fee: f64) -> Self {
        let cur_year: i32 = dates::current_month()
            .get(..4)
            .and_then(|s| s.parse().ok())
            .unwrap_or(2026);
        let join_year: i32 = m
            .join_date
            .get(..4)
            .and_then(|s| s.parse().ok())
            .unwrap_or(cur_year);
        let payments = payments.to_vec();
        let reg_paid = payments.iter().any(|p| p.category == "registration");
        // Show every year the member has existed for, plus next year so payments
        // can be recorded ahead. Widen to cover any stray out-of-range payment.
        let mut min_year = join_year.min(cur_year);
        let mut max_year = cur_year + 1;
        for p in &payments {
            if let Some(y) = p.period_month.get(..4).and_then(|s| s.parse::<i32>().ok()) {
                min_year = min_year.min(y);
                max_year = max_year.max(y);
            }
        }
        Self {
            member_id: m.id,
            member_name: m.name.clone(),
            entries: Self::build_entries(cur_year, &payments),
            payments,
            year: cur_year,
            min_year,
            max_year,
            reg_paid,
            reg_fee,
            error: None,
        }
    }

    fn build_entries(year: i32, payments: &[Payment]) -> Vec<MonthEntry> {
        (1..=12)
            .map(|m| {
                let month = format!("{year:04}-{m:02}");
                let original = payments
                    .iter()
                    .find(|p| p.category != "registration" && p.period_month == month)
                    .cloned();
                let covered = original.as_ref().map(|p| p.amount == 0.0).unwrap_or(false);
                let amount = original
                    .as_ref()
                    .filter(|p| p.amount != 0.0)
                    .map(|p| format!("{}", p.amount))
                    .unwrap_or_default();
                MonthEntry {
                    label: MONTH_NAMES[(m - 1) as usize].to_string(),
                    amount,
                    covered,
                    month,
                    original,
                }
            })
            .collect()
    }

    fn set_year(&mut self, year: i32) {
        self.year = year;
        self.entries = Self::build_entries(year, &self.payments);
    }

    fn reload(&mut self, payments: &[Payment]) {
        self.payments = payments.to_vec();
        self.reg_paid = self.payments.iter().any(|p| p.category == "registration");
        self.entries = Self::build_entries(self.year, &self.payments);
    }

    /// Reconcile the currently shown year's edited entries against the DB:
    /// insert new membership payments, update changed amounts, delete cleared
    /// months. Shared by the Save button and the year switch so no edit is lost.
    fn commit(&self, repo: &Repository) -> rusqlite::Result<()> {
        for e in &self.entries {
            // A month is settled either by money (amount > 0) or by being marked
            // Covered (stored as a zero-amount payment). Anything else = no row.
            let target = e
                .amount
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|v| *v > 0.0)
                .or(if e.covered { Some(0.0) } else { None });
            match (&e.original, target) {
                (Some(orig), Some(amt)) => {
                    if (amt - orig.amount).abs() > f64::EPSILON {
                        let mut p = orig.clone();
                        p.amount = amt;
                        repo.update_payment(&p)?;
                    }
                }
                (Some(orig), None) => repo.delete_payment(orig.id)?,
                (None, Some(amt)) => {
                    repo.insert_payment(&Payment {
                        id: 0,
                        member_id: self.member_id,
                        period_month: e.month.clone(),
                        amount: amt,
                        date: format!("{}-15", e.month),
                        note: None,
                        category: "membership".to_string(),
                    })?;
                }
                (None, None) => {}
            }
        }
        Ok(())
    }
}

/// Focused first-payment prompt shown right after a NEW member is added: this
/// month's membership fee plus an optional one-time joining fee, saved together.
struct NewMemberPrompt {
    member_id: i64,
    member_name: String,
    month: String,
    month_label: String,
    fee: String,
    reg_fee: f64,
    collect_reg: bool,
    error: Option<String>,
}

impl NewMemberPrompt {
    fn new(m: &Member, repo: &Repository) -> Self {
        let month = dates::current_month();
        let month_label = month
            .get(5..7)
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| (1..=12).contains(n))
            .map(|n| format!("{} {}", MONTH_NAMES[n - 1], &month[..4]))
            .unwrap_or_else(|| month.clone());
        Self {
            member_id: m.id,
            member_name: m.name.clone(),
            month,
            month_label,
            fee: format!("{:.0}", repo.default_monthly_fee()),
            reg_fee: repo.registration_fee(),
            collect_reg: true,
            error: None,
        }
    }
}

enum Dialog {
    None,
    Member(MemberForm),
    NewMember(NewMemberPrompt),
    Details { member: Member, payments: Vec<Payment> },
    Payments(PaymentsEditor),
}

#[derive(Clone, Copy, PartialEq)]
enum MemberFilter {
    All,
    PaidUp,
    Due,
    RegPaid,
    RegDue,
}

impl MemberFilter {
    const ALL: [MemberFilter; 5] = [
        MemberFilter::All,
        MemberFilter::PaidUp,
        MemberFilter::Due,
        MemberFilter::RegPaid,
        MemberFilter::RegDue,
    ];
    fn label(self) -> &'static str {
        match self {
            MemberFilter::All => "All members",
            MemberFilter::PaidUp => "Paid up",
            MemberFilter::Due => "Owes payment",
            MemberFilter::RegPaid => "Registration paid",
            MemberFilter::RegDue => "Registration due",
        }
    }
    fn matches(
        self,
        m: &Member,
        arrears: &HashMap<i64, (i64, f64)>,
        reg_missing: &std::collections::HashSet<i64>,
    ) -> bool {
        match self {
            MemberFilter::All => true,
            MemberFilter::PaidUp => !arrears.contains_key(&m.id),
            MemberFilter::Due => arrears.contains_key(&m.id),
            MemberFilter::RegPaid => !reg_missing.contains(&m.id),
            MemberFilter::RegDue => reg_missing.contains(&m.id),
        }
    }
}

pub struct MembersState {
    search: String,
    show_inactive: bool,
    filter: MemberFilter,
    members: Vec<Member>,
    arrears: HashMap<i64, (i64, f64)>,
    reg_missing: std::collections::HashSet<i64>,
    loaded: bool,
    dialog: Dialog,
    status: Option<String>,
}

impl Default for MembersState {
    fn default() -> Self {
        Self {
            search: String::new(),
            show_inactive: false,
            filter: MemberFilter::All,
            members: Vec::new(),
            arrears: HashMap::new(),
            reg_missing: std::collections::HashSet::new(),
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

/// A small rounded pill used for member flags (registration due).
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

    /// Indices into `self.members` that pass the current filter. Recomputed at
    /// the point of use because the search box can reload (and shrink) the roster
    /// mid-frame — holding indices across a reload would point past the new list.
    fn visible(&self) -> Vec<usize> {
        (0..self.members.len())
            .filter(|&i| {
                self.filter
                    .matches(&self.members[i], &self.arrears, &self.reg_missing)
            })
            .collect()
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
        // A member is Due if they owe any month since they joined, not just the
        // current one. `arrears_all` returns only members who owe > 0.
        self.arrears = repo.arrears_all(&month).unwrap_or_default();
        self.reg_missing = repo
            .members_missing_registration(false)
            .unwrap_or_default()
            .iter()
            .map(|m| m.id)
            .collect();
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
            egui::ComboBox::from_id_salt("member_filter")
                .selected_text(self.filter.label())
                .show_ui(ui, |ui| {
                    for f in MemberFilter::ALL {
                        ui.selectable_value(&mut self.filter, f, f.label());
                    }
                });
            if ui
                .checkbox(&mut self.show_inactive, "Show inactive (quit)")
                .changed()
            {
                self.reload(repo);
            }
            ui.label(egui::RichText::new(format!("{} shown", self.visible().len())).weak());
        });

        ui.add_space(6.0);
        ui.separator();

        let visible = self.visible();
        if let Some(s) = &self.status {
            ui.add_space(4.0);
            ui.colored_label(egui::Color32::from_rgb(210, 90, 90), s);
        }

        let currency = repo.currency();
        let mut action: Option<Action> = None;

        if visible.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                if self.search.trim().is_empty() && self.filter == MemberFilter::All {
                    ui.label(egui::RichText::new("No members yet.").weak());
                    ui.add_space(6.0);
                    if ui.button("+ Add your first member").clicked() {
                        self.dialog = Dialog::Member(MemberForm::new_member());
                    }
                } else {
                    ui.label(egui::RichText::new("No members match this filter.").weak());
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
                body.rows(row_height, visible.len(), |mut row| {
                    let m = &self.members[visible[row.index()]];
                    let status = if !m.active {
                        MemberStatus::Inactive
                    } else if self.arrears.contains_key(&m.id) {
                        MemberStatus::Due
                    } else {
                        MemberStatus::Paid
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
                        if self.reg_missing.contains(&m.id) {
                            badge(ui, "Reg due", egui::Color32::from_rgb(200, 110, 50));
                        }
                    });
                    row.col(|ui| { ui.label(&m.join_date); });
                    row.col(|ui| {
                        if m.active && ui.button("Record payment").clicked() {
                            action = Some(Action::OpenPayment(m.id));
                        }
                        ui.menu_button("⋯", |ui| {
                            if ui.button("Details").clicked() {
                                action = Some(Action::Details(m.id));
                                ui.close();
                            }
                            if ui.button("Edit").clicked() {
                                action = Some(Action::Edit(m.id));
                                ui.close();
                            }
                            let toggle = if m.active { "Deactivate" } else { "Reactivate" };
                            if ui.button(toggle).clicked() {
                                action = Some(Action::ToggleActive(m.id, !m.active));
                                ui.close();
                            }
                        });
                    });
                });
            });
        }

        if let Some(a) = action {
            self.handle_action(a, repo);
        }

        self.show_dialog(ui.ctx(), repo, &currency);
    }

    fn handle_action(&mut self, a: Action, repo: &mut Repository) {
        match a {
            Action::OpenPayment(id) => {
                if let Ok(Some(m)) = repo.get_member(id) {
                    let payments = repo.payments_for_member(id).unwrap_or_default();
                    let reg_fee = repo.registration_fee();
                    self.dialog = Dialog::Payments(PaymentsEditor::new(&m, &payments, reg_fee));
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
        }
    }

    fn show_dialog(
        &mut self,
        ctx: &egui::Context,
        repo: &mut Repository,
        currency: &str,
    ) {
        let mut close = false;
        let mut new_member_for: Option<i64> = None;
        let mut status_update: Option<Option<String>> = None;
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::Member(form) => {
                let title = if form.editing { "Edit member" } else { "Add member" };
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
                                        Ok(()) => close = true,
                                        Err(e) => form.error = Some(format!("Save failed: {e}")),
                                    }
                                } else {
                                    match repo.insert_member(&m) {
                                        Ok(id) => {
                                            new_member_for = Some(id);
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
            Dialog::Details { member, payments } => {
                let m = &*member;
                let status = if !m.active {
                    MemberStatus::Inactive
                } else if self.arrears.contains_key(&m.id) {
                    MemberStatus::Due
                } else {
                    MemberStatus::Paid
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
                                let reg_paid = payments.iter().any(|p| p.category == "registration");
                                ui.label(if reg_paid { "Paid" } else { "Due" });
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
            Dialog::NewMember(p) => {
                egui::Window::new(format!("First payment — {}", p.member_name))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.set_min_width(320.0);
                        ui.label(
                            egui::RichText::new(
                                "Record their first payment now, or skip and do it later.",
                            )
                            .weak(),
                        );
                        ui.add_space(8.0);
                        egui::Grid::new("new_member_pay")
                            .num_columns(2)
                            .spacing([10.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(format!("{} fee", p.month_label));
                                ui.add(
                                    egui::TextEdit::singleline(&mut p.fee).desired_width(100.0),
                                );
                                ui.end_row();
                                ui.label("Joining fee");
                                ui.checkbox(
                                    &mut p.collect_reg,
                                    format!("Collect {} {:.0}", currency, p.reg_fee),
                                );
                                ui.end_row();
                            });
                        ui.add_space(8.0);
                        let fee_ok = p.fee.trim().is_empty() || p.fee.trim().parse::<f64>().is_ok();
                        ui.horizontal(|ui| {
                            if ui.add_enabled(fee_ok, egui::Button::new("Save")).clicked() {
                                let mut result = Ok(());
                                if let Some(amt) =
                                    p.fee.trim().parse::<f64>().ok().filter(|v| *v > 0.0)
                                {
                                    result = repo
                                        .insert_payment(&Payment {
                                            id: 0,
                                            member_id: p.member_id,
                                            period_month: p.month.clone(),
                                            amount: amt,
                                            date: format!("{}-15", p.month),
                                            note: None,
                                            category: "membership".to_string(),
                                        })
                                        .map(|_| ());
                                }
                                if result.is_ok() && p.collect_reg {
                                    let today = dates::today();
                                    result = repo.ensure_registration_payment(
                                        p.member_id,
                                        p.reg_fee,
                                        &today,
                                        &p.month,
                                    );
                                }
                                match result {
                                    Ok(()) => close = true,
                                    Err(e) => p.error = Some(format!("Couldn't save — {e}")),
                                }
                            }
                            if ui.button("Skip").clicked() {
                                close = true;
                            }
                            if !fee_ok {
                                ui.colored_label(
                                    egui::Color32::from_rgb(210, 120, 40),
                                    "Amount must be a number",
                                );
                            }
                        });
                        if let Some(err) = &p.error {
                            ui.add_space(4.0);
                            ui.colored_label(egui::Color32::from_rgb(210, 90, 90), err);
                        }
                    });
            }
            Dialog::Payments(ed) => {
                let mut save = false;
                let mut new_year = ed.year;
                let mut toggle_reg: Option<bool> = None;
                egui::Window::new(format!("Payments — {}", ed.member_name))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.set_min_width(340.0);
                        ui.horizontal(|ui| {
                            let mut checked = ed.reg_paid;
                            if ui
                                .checkbox(
                                    &mut checked,
                                    format!(
                                        "Joining fee collected ({currency} {:.0})",
                                        ed.reg_fee
                                    ),
                                )
                                .changed()
                            {
                                toggle_reg = Some(checked);
                            }
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Year").strong());
                            egui::ComboBox::from_id_salt("pay_year")
                                .selected_text(format!("{}", ed.year))
                                .show_ui(ui, |ui| {
                                    for y in ed.min_year..=ed.max_year {
                                        ui.selectable_value(&mut new_year, y, format!("{y}"));
                                    }
                                });
                            let total: f64 = ed
                                .entries
                                .iter()
                                .filter_map(|e| e.amount.trim().parse::<f64>().ok())
                                .sum();
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("Total {currency} {total:.0}"))
                                            .strong(),
                                    );
                                },
                            );
                        });
                        ui.weak("Enter the amount in the month you received it. For a prepayment, mark the other months Covered - no need to split the money.");
                        ui.add_space(8.0);
                        let invalid = ed.entries.iter().any(|e| {
                            let t = e.amount.trim();
                            !t.is_empty() && t.parse::<f64>().is_err()
                        });
                        {
                            egui::Grid::new("pay_editor")
                                .num_columns(3)
                                .spacing([12.0, 8.0])
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Month").weak().size(11.0));
                                    ui.label(
                                        egui::RichText::new(format!("Amount ({currency})"))
                                            .weak()
                                            .size(11.0),
                                    );
                                    ui.label(egui::RichText::new("Status").weak().size(11.0));
                                    ui.end_row();
                                    for e in &mut ed.entries {
                                        ui.label(&e.label);
                                        ui.add(
                                            egui::TextEdit::singleline(&mut e.amount)
                                                .desired_width(90.0),
                                        );
                                        let t = e.amount.trim();
                                        if t.is_empty() {
                                            if e.covered {
                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            egui::RichText::new("Covered")
                                                                .color(egui::Color32::from_rgb(
                                                                    90, 150, 210,
                                                                ))
                                                                .size(12.0),
                                                        )
                                                        .small(),
                                                    )
                                                    .on_hover_text("Prepaid or comped - no money this month. Click to mark Due.")
                                                    .clicked()
                                                {
                                                    e.covered = false;
                                                }
                                            } else if ui
                                                .add(
                                                    egui::Button::new(
                                                        egui::RichText::new("Due")
                                                            .color(egui::Color32::from_rgb(
                                                                210, 120, 40,
                                                            ))
                                                            .size(12.0),
                                                    )
                                                    .small(),
                                                )
                                                .on_hover_text("Settled without money (e.g. covered by a prepayment)? Click to mark Covered.")
                                                .clicked()
                                            {
                                                e.covered = true;
                                            }
                                        } else if t.parse::<f64>().map(|v| v > 0.0).unwrap_or(false) {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(40, 170, 90),
                                                "Paid",
                                            );
                                        } else if t.parse::<f64>().is_ok() {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(210, 120, 40),
                                                "Due",
                                            );
                                        } else {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(210, 90, 90),
                                                "?",
                                            );
                                        }
                                        ui.end_row();
                                    }
                                });
                        }
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(!invalid, egui::Button::new("Save changes"))
                                .clicked()
                            {
                                save = true;
                            }
                            if ui.button("Close").clicked() {
                                close = true;
                            }
                            if invalid {
                                ui.colored_label(
                                    egui::Color32::from_rgb(210, 120, 40),
                                    "Fix highlighted amounts",
                                );
                            }
                        });
                        if let Some(err) = &ed.error {
                            ui.add_space(4.0);
                            ui.colored_label(egui::Color32::from_rgb(210, 90, 90), err);
                        }
                    });
                if new_year != ed.year {
                    match ed.commit(repo) {
                        Ok(()) => {
                            let payments =
                                repo.payments_for_member(ed.member_id).unwrap_or_default();
                            ed.reload(&payments);
                            ed.set_year(new_year);
                            ed.error = None;
                        }
                        Err(e) => {
                            ed.error = Some(format!(
                                "Couldn't save {} before switching — {e}",
                                ed.year
                            ));
                        }
                    }
                }
                if let Some(now) = toggle_reg {
                    let r = if now {
                        let today = dates::today();
                        let month = dates::current_month();
                        repo.ensure_registration_payment(ed.member_id, ed.reg_fee, &today, &month)
                    } else {
                        repo.remove_registration_payment(ed.member_id)
                    };
                    match r {
                        Ok(()) => {
                            let payments =
                                repo.payments_for_member(ed.member_id).unwrap_or_default();
                            ed.reload(&payments);
                            status_update = Some(None);
                        }
                        Err(e) => ed.error = Some(format!("Couldn't update joining fee — {e}")),
                    }
                }
                if save {
                    match ed.commit(repo) {
                        Ok(()) => {
                            status_update = Some(None);
                            let payments =
                                repo.payments_for_member(ed.member_id).unwrap_or_default();
                            ed.reload(&payments);
                            ed.error = None;
                        }
                        Err(e) => ed.error = Some(format!("Couldn't save changes — {e}")),
                    }
                }
            }
        }

        if let Some(u) = status_update {
            self.status = u;
        }

        if let Some(id) = new_member_for {
            if let Ok(Some(m)) = repo.get_member(id) {
                self.dialog = Dialog::NewMember(NewMemberPrompt::new(&m, repo));
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
}

