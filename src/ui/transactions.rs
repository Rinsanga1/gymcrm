use eframe::egui::{self, Color32, RichText};

use crate::core::dates;
use crate::core::models::{Expense, Payment, Txn, TxnKind};
use crate::core::Repository;

const INCOME: Color32 = Color32::from_rgb(45, 170, 95);
const OUTGOING: Color32 = Color32::from_rgb(210, 90, 90);

struct MonthGroup {
    label: String,
    net: f64,
    items: Vec<Txn>,
}

struct PaymentForm {
    id: i64,
    title: String,
    amount: String,
    month: String,
    date: String,
    note: String,
    category: String,
}

struct ExpenseForm {
    id: i64,
    name: String,
    amount: String,
    date: String,
    note: String,
}

enum Dialog {
    None,
    EditPayment(PaymentForm),
    EditExpense(ExpenseForm),
    ConfirmDelete { kind: TxnKind, id: i64, label: String },
}

enum Action {
    Edit(TxnKind, i64),
    AskDelete(TxnKind, i64, String),
}

pub struct TransactionsState {
    groups: Vec<MonthGroup>,
    dirty: bool,
    dialog: Dialog,
}

impl Default for TransactionsState {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            dirty: true,
            dialog: Dialog::None,
        }
    }
}

impl TransactionsState {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    fn reload(&mut self, repo: &Repository) {
        let txns = repo.list_transactions().unwrap_or_default();
        let mut groups: Vec<MonthGroup> = Vec::new();
        let mut cur: Option<String> = None;
        for t in txns {
            let ym = if t.date.len() >= 7 {
                t.date[..7].to_string()
            } else {
                t.date.clone()
            };
            if cur.as_deref() != Some(ym.as_str()) {
                groups.push(MonthGroup {
                    label: dates::pretty_month(&ym),
                    net: 0.0,
                    items: Vec::new(),
                });
                cur = Some(ym);
            }
            let g = groups.last_mut().unwrap();
            g.net += t.amount;
            g.items.push(t);
        }
        self.groups = groups;
        self.dirty = false;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) {
        if self.dirty {
            self.reload(repo);
        }

        ui.add_space(4.0);
        ui.heading("Transactions");
        ui.add_space(8.0);

        let currency = repo.currency();
        let mut action: Option<Action> = None;

        if self.groups.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No transactions yet.").weak());
            });
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(820.0);
                        let spacing = ui.spacing().item_spacing.x;
                        let avail = (ui.available_width() - 4.0).max(420.0);
                        let date_w = 96.0;
                        let amt_w = 110.0;
                        let act_w = 132.0;
                        let desc_w =
                            (avail - date_w - amt_w - act_w - spacing * 3.0).max(150.0);

                        for (gi, g) in self.groups.iter().enumerate() {
                            ui.add_space(if gi == 0 { 2.0 } else { 20.0 });
                            egui::Frame::new()
                                .inner_margin(egui::Margin::symmetric(8, 0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&g.label).strong().size(16.0));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    amount_text(g.net, &currency)
                                                        .strong()
                                                        .size(14.0),
                                                );
                                            },
                                        );
                                    });
                                });
                            ui.add_space(2.0);
                            ui.separator();

                            for (ri, t) in g.items.iter().enumerate() {
                                let bg = if ri % 2 == 1 {
                                    ui.visuals().faint_bg_color
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                egui::Frame::new()
                                    .fill(bg)
                                    .corner_radius(6.0)
                                    .inner_margin(egui::Margin::symmetric(8, 7))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.set_min_height(30.0);
                                            ui.add_sized(
                                                [date_w, 28.0],
                                                egui::Label::new(
                                                    RichText::new(&t.date).weak(),
                                                )
                                                .halign(egui::Align::LEFT),
                                            );
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(desc_w, 28.0),
                                                egui::Layout::top_down(egui::Align::Min),
                                                |ui| {
                                                    ui.add(
                                                        egui::Label::new(
                                                            RichText::new(&t.label).size(13.5),
                                                        )
                                                        .truncate(),
                                                    );
                                                    if let Some(d) = &t.detail {
                                                        ui.add(
                                                            egui::Label::new(
                                                                RichText::new(d)
                                                                    .weak()
                                                                    .size(11.0),
                                                            )
                                                            .truncate(),
                                                        );
                                                    }
                                                },
                                            );
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(amt_w, 28.0),
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        amount_text(t.amount, &currency)
                                                            .size(13.5),
                                                    );
                                                },
                                            );
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(act_w, 28.0),
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.small_button("Delete").clicked() {
                                                        action = Some(Action::AskDelete(
                                                            t.kind,
                                                            t.id,
                                                            t.label.clone(),
                                                        ));
                                                    }
                                                    if !matches!(t.kind, TxnKind::Sale)
                                                        && ui.small_button("Edit").clicked()
                                                    {
                                                        action =
                                                            Some(Action::Edit(t.kind, t.id));
                                                    }
                                                },
                                            );
                                        });
                                    });
                            }
                        }
                        ui.add_space(16.0);
                    });
                });
        }

        if let Some(a) = action {
            self.handle_action(a, repo);
        }

        self.draw_dialog(ui.ctx(), repo);
    }

    fn handle_action(&mut self, a: Action, repo: &Repository) {
        match a {
            Action::Edit(TxnKind::Payment, id) => {
                if let Ok(Some(p)) = repo.get_payment(id) {
                    self.dialog = Dialog::EditPayment(PaymentForm {
                        id: p.id,
                        title: p.category.clone(),
                        amount: format!("{}", p.amount),
                        month: p.period_month.clone(),
                        date: p.date.clone(),
                        note: p.note.clone().unwrap_or_default(),
                        category: p.category,
                    });
                }
            }
            Action::Edit(TxnKind::Expense, id) => {
                if let Ok(Some(e)) = repo.get_expense(id) {
                    self.dialog = Dialog::EditExpense(ExpenseForm {
                        id: e.id,
                        name: e.name,
                        amount: format!("{}", e.amount),
                        date: e.date,
                        note: e.note.unwrap_or_default(),
                    });
                }
            }
            Action::Edit(TxnKind::Sale, _) => {}
            Action::AskDelete(kind, id, label) => {
                self.dialog = Dialog::ConfirmDelete { kind, id, label };
            }
        }
    }

    fn draw_dialog(&mut self, ctx: &egui::Context, repo: &mut Repository) {
        let mut close = false;
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::EditPayment(form) => {
                egui::Window::new("Edit payment")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(RichText::new(format!("Category: {}", form.title)).weak());
                        egui::Grid::new("edit_pay_form").num_columns(2).show(ui, |ui| {
                            ui.label("Amount");
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
                        let amt_ok = form.amount.trim().parse::<f64>().is_ok();
                        let month_ok = dates::is_valid_month(&form.month);
                        let date_ok = dates::is_valid_date(&form.date);
                        let valid = amt_ok && month_ok && date_ok;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                                let p = Payment {
                                    id: form.id,
                                    member_id: 0,
                                    period_month: form.month.trim().to_string(),
                                    amount: form.amount.trim().parse().unwrap_or(0.0),
                                    date: form.date.trim().to_string(),
                                    note: opt(&form.note),
                                    category: form.category.clone(),
                                };
                                if repo.update_payment(&p).is_ok() {
                                    self.dirty = true;
                                    close = true;
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                            if !month_ok {
                                ui.colored_label(OUTGOING, "Month should look like 2026-07");
                            }
                        });
                    });
            }
            Dialog::EditExpense(form) => {
                egui::Window::new("Edit expense")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        egui::Grid::new("edit_exp_form").num_columns(2).show(ui, |ui| {
                            ui.label("Name");
                            ui.text_edit_singleline(&mut form.name);
                            ui.end_row();
                            ui.label("Date");
                            crate::ui::date_edit(ui, &mut form.date);
                            ui.end_row();
                            ui.label("Amount");
                            ui.text_edit_singleline(&mut form.amount);
                            ui.end_row();
                            ui.label("Note");
                            ui.text_edit_multiline(&mut form.note);
                            ui.end_row();
                        });
                        ui.add_space(6.0);
                        let valid = !form.name.trim().is_empty()
                            && dates::is_valid_date(&form.date)
                            && form.amount.trim().parse::<f64>().is_ok();
                        ui.horizontal(|ui| {
                            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                                let e = Expense {
                                    id: form.id,
                                    name: form.name.trim().to_string(),
                                    amount: form.amount.trim().parse().unwrap_or(0.0),
                                    date: form.date.trim().to_string(),
                                    note: opt(&form.note),
                                };
                                if repo.update_expense(&e).is_ok() {
                                    self.dirty = true;
                                    close = true;
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                        });
                    });
            }
            Dialog::ConfirmDelete { kind, id, label } => {
                let kind = *kind;
                let id = *id;
                egui::Window::new("Delete transaction?")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(format!("Delete \"{label}\"? This cannot be undone."));
                        if matches!(kind, TxnKind::Sale) {
                            ui.label(
                                RichText::new("Stock will be restored for this sale.")
                                    .weak()
                                    .size(11.0),
                            );
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::Button::new("Delete").fill(Color32::from_rgb(170, 50, 50)))
                                .clicked()
                            {
                                let ok = match kind {
                                    TxnKind::Payment => repo.delete_payment(id).is_ok(),
                                    TxnKind::Expense => repo.delete_expense(id).is_ok(),
                                    TxnKind::Sale => repo.delete_sale(id).is_ok(),
                                };
                                if ok {
                                    self.dirty = true;
                                }
                                close = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                        });
                    });
            }
        }
        if close {
            self.dialog = Dialog::None;
            if self.dirty {
                self.reload(repo);
            }
        }
    }
}

fn amount_text(amount: f64, currency: &str) -> RichText {
    let income = amount >= 0.0;
    let sign = if income { "+" } else { "-" };
    let color = if income { INCOME } else { OUTGOING };
    RichText::new(format!("{sign}{currency} {:.0}", amount.abs())).color(color)
}

fn opt(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}
