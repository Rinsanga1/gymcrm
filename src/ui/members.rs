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
    // New-member only: the mandatory joining fee and an optional first month.
    joining_fee: String,
    month_fee: String,
    month: String,
}

/// "August 2026" from a `YYYY-MM`, falling back to the raw string.
fn month_label(month: &str) -> String {
    month
        .get(5..7)
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| (1..=12).contains(n))
        .map(|n| format!("{} {}", MONTH_NAMES[n - 1], &month[..4]))
        .unwrap_or_else(|| month.to_string())
}

impl MemberForm {
    fn new_member(repo: &Repository) -> Self {
        Self {
            join_date: dates::today(),
            active: true,
            joining_fee: format!("{:.0}", repo.registration_fee()),
            month_fee: format!("{:.0}", repo.default_monthly_fee()),
            month: dates::current_month(),
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
            ..Default::default()
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
    error: Option<String>,
    // Amounts as they stood at the last successful save. The "Saved" confirmation
    // shows only while the current amounts still match this, so any edit clears it.
    saved_amounts: Option<Vec<(String, bool)>>,
}

impl PaymentsEditor {
    fn snapshot(&self) -> Vec<(String, bool)> {
        self.entries.iter().map(|e| (e.amount.clone(), e.covered)).collect()
    }

    fn is_saved(&self) -> bool {
        self.saved_amounts.as_ref() == Some(&self.snapshot())
    }

    fn new(m: &Member, payments: &[Payment]) -> Self {
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
            error: None,
            saved_amounts: None,
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
                        date: dates::today(),
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

enum Dialog {
    None,
    Member(MemberForm),
    Details { member: Member, payments: Vec<Payment> },
    Payments(PaymentsEditor),
}

#[derive(Clone, Copy, PartialEq)]
enum MemberFilter {
    All,
    PaidUp,
    Due,
}

impl MemberFilter {
    const ALL: [MemberFilter; 3] = [
        MemberFilter::All,
        MemberFilter::PaidUp,
        MemberFilter::Due,
    ];
    fn label(self) -> &'static str {
        match self {
            MemberFilter::All => "All members",
            MemberFilter::PaidUp => "Paid this month",
            MemberFilter::Due => "Due",
        }
    }
    fn matches(self, m: &Member, arrears: &HashMap<i64, (i64, f64)>) -> bool {
        match self {
            MemberFilter::All => true,
            MemberFilter::PaidUp => !arrears.contains_key(&m.id),
            MemberFilter::Due => arrears.contains_key(&m.id),
        }
    }
}

pub struct MembersState {
    search: String,
    show_inactive: bool,
    filter: MemberFilter,
    members: Vec<Member>,
    arrears: HashMap<i64, (i64, f64)>,
    loaded: bool,
    dialog: Dialog,
    status: Option<String>,
    // A member to open the payment book for on the next `show` (set when a
    // Transactions row is tapped). Deferred because opening needs `repo`.
    pending_payments: Option<i64>,
}

impl Default for MembersState {
    fn default() -> Self {
        Self {
            search: String::new(),
            show_inactive: false,
            filter: MemberFilter::All,
            members: Vec::new(),
            arrears: HashMap::new(),
            loaded: false,
            dialog: Dialog::None,
            status: None,
            pending_payments: None,
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

    /// Open the roster pre-filtered to members who owe payment. Used by the
    /// dashboard's "Members due" hero so the number drills through to the list.
    pub fn focus_due(&mut self) {
        self.filter = MemberFilter::Due;
        self.show_inactive = false;
        self.search.clear();
        self.loaded = false;
    }

    /// Open a specific member's payment book — used when a Transactions payment
    /// row is tapped. Shows the whole roster so the member is findable behind it.
    pub fn focus_member(&mut self, id: i64) {
        self.filter = MemberFilter::All;
        self.show_inactive = true;
        self.search.clear();
        self.loaded = false;
        self.pending_payments = Some(id);
    }

    /// Indices into `self.members` that pass the current filter. Recomputed at
    /// the point of use because the search box can reload (and shrink) the roster
    /// mid-frame — holding indices across a reload would point past the new list.
    fn visible(&self) -> Vec<usize> {
        (0..self.members.len())
            .filter(|&i| self.filter.matches(&self.members[i], &self.arrears))
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
        self.loaded = true;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) {
        if !self.loaded {
            self.reload(repo);
        }
        if let Some(id) = self.pending_payments.take() {
            self.handle_action(Action::OpenPayment(id), repo);
        }

        ui.horizontal(|ui| {
            ui.heading("Members");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ New member").clicked() {
                    self.dialog = Dialog::Member(MemberForm::new_member(repo));
                }
            });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .add(egui::TextEdit::singleline(&mut self.search).hint_text("Search name or phone"))
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
                .checkbox(&mut self.show_inactive, "Include past members")
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
                        self.dialog = Dialog::Member(MemberForm::new_member(repo));
                    }
                } else {
                    ui.label(egui::RichText::new("No members match this filter.").weak());
                }
            });
        } else {
        let row_height = 34.0;
        crate::ui::wide_table(ui, 820.0, |ui| {
        TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(160.0).clip(true)) // name
            .column(Column::auto().at_least(110.0)) // phone
            .column(Column::auto().at_least(90.0)) // status
            .column(Column::auto().at_least(100.0)) // join
            .column(Column::remainder().at_least(220.0)) // actions
            .header(30.0, |mut h| {
                h.col(|ui| { ui.strong("Name"); });
                h.col(|ui| { ui.strong("Phone"); });
                h.col(|ui| { ui.strong("Status"); });
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
                                    format!("{behind} months behind")
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
                                ui.colored_label(egui::Color32::GRAY, "Past member");
                            }
                        }
                    });
                    row.col(|ui| { ui.label(&m.join_date); });
                    row.col(|ui| {
                        if m.active && ui.button("Payments").clicked() {
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
                            let toggle = if m.active { "Mark as left" } else { "Mark as active" };
                            if ui.button(toggle).clicked() {
                                action = Some(Action::ToggleActive(m.id, !m.active));
                                ui.close();
                            }
                        });
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
                    self.dialog = Dialog::Payments(PaymentsEditor::new(&m, &payments));
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
                            ui.label("Full name");
                            ui.text_edit_singleline(&mut form.name);
                            ui.end_row();
                            ui.label("Phone");
                            ui.text_edit_singleline(&mut form.phone);
                            ui.end_row();
                            ui.label("Joined on");
                            crate::ui::date_edit(ui, &mut form.join_date);
                            ui.end_row();
                            ui.label("Notes");
                            ui.text_edit_multiline(&mut form.notes);
                            ui.end_row();
                            if !form.editing {
                                ui.label("Joining fee");
                                ui.add(egui::TextEdit::singleline(&mut form.joining_fee).desired_width(100.0));
                                ui.end_row();
                                ui.label(format!("This month ({})", month_label(&form.month)));
                                ui.add(egui::TextEdit::singleline(&mut form.month_fee).desired_width(100.0));
                                ui.end_row();
                            }
                        });
                        ui.add_space(6.0);
                        let name_ok = !form.name.trim().is_empty();
                        let date_ok = form.join_date.trim().is_empty()
                            || dates::is_valid_date(&form.join_date);
                        // Joining fee is mandatory for a new member: it must be a
                        // number (0 = a deliberate, logged waiver). All values are
                        // owned so nothing borrows `form` across the Save closure.
                        let joining_amt = form.joining_fee.trim().parse::<f64>().ok();
                        let month_empty = form.month_fee.trim().is_empty();
                        let month_amt = form.month_fee.trim().parse::<f64>().ok();
                        let fee_ok = form.editing
                            || (joining_amt.is_some() && (month_empty || month_amt.is_some()));
                        let valid = name_ok && date_ok && fee_ok;
                        let first = if month_empty { None } else { month_amt.filter(|v| *v > 0.0) };
                        let joining = joining_amt.unwrap_or(0.0);
                        if !form.editing {
                            let collecting = joining + first.unwrap_or(0.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "Collecting {} today",
                                    crate::ui::money(currency, collecting)
                                ))
                                .strong(),
                            );
                            ui.add_space(4.0);
                        }
                        ui.horizontal(|ui| {
                            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                                let m = form.to_member();
                                if form.editing {
                                    match repo.update_member(&m) {
                                        Ok(()) => close = true,
                                        Err(e) => form.error = Some(format!("Save failed: {e}")),
                                    }
                                } else {
                                    match repo.create_member(&m, joining, first, &form.month, &dates::today()) {
                                        Ok(_) => close = true,
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
                            } else if !fee_ok {
                                ui.colored_label(egui::Color32::from_rgb(210, 120, 40), "Fees must be numbers");
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
                                    MemberStatus::Paid => ("Paid this month", egui::Color32::from_rgb(40, 170, 90)),
                                    MemberStatus::Due => ("Due this month", egui::Color32::from_rgb(210, 120, 40)),
                                    MemberStatus::Inactive => ("Past member", egui::Color32::GRAY),
                                };
                                ui.colored_label(color, txt);
                                ui.end_row();
                                ui.label(egui::RichText::new("Joining fee").weak());
                                let reg = payments.iter().find(|p| p.category == "registration");
                                ui.label(match reg {
                                    Some(p) => format!("{} · {}", crate::ui::money(currency, p.amount), p.date),
                                    None => "—".to_string(),
                                });
                                ui.end_row();
                                ui.label(egui::RichText::new("Total paid").weak());
                                ui.label(crate::ui::money(currency, total_paid));
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
                                            ui.label(crate::core::models::category_label(&p.category));
                                            ui.label(crate::ui::money(currency, p.amount));
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
            Dialog::Payments(ed) => {
                let mut save = false;
                let mut new_year = ed.year;
                egui::Window::new(format!("Payments — {}", ed.member_name))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.set_min_width(340.0);
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
                        ui.weak("Enter the amount in the month you received it. Paid a few months up front? Mark those months Paid ahead - no need to split the money.");
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
                                                            egui::RichText::new("Paid ahead")
                                                                .color(egui::Color32::from_rgb(
                                                                    90, 150, 210,
                                                                ))
                                                                .size(12.0),
                                                        )
                                                        .small(),
                                                    )
                                                    .on_hover_text("Paid ahead or comped - no money this month. Click to mark Due.")
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
                                                .on_hover_text("Settled without money (e.g. paid ahead earlier)? Click to mark Paid ahead.")
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
                                .add_enabled(!invalid, egui::Button::new("Save"))
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
                            } else if ed.is_saved() {
                                ui.colored_label(
                                    egui::Color32::from_rgb(40, 170, 90),
                                    "✓ Saved",
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
                            ed.saved_amounts = None;
                        }
                        Err(e) => {
                            ed.error = Some(format!(
                                "Couldn't save {} before switching — {e}",
                                ed.year
                            ));
                        }
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
                            ed.saved_amounts = Some(ed.snapshot());
                        }
                        Err(e) => ed.error = Some(format!("Couldn't save changes — {e}")),
                    }
                }
            }
        }

        if let Some(u) = status_update {
            self.status = u;
        }

        if close {
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

