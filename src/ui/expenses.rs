use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::core::dates;
use crate::core::models::{Expense, RecurringExpense};
use crate::core::Repository;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Expenses,
    Recurring,
}

#[derive(Default)]
struct ExpenseForm {
    id: i64,
    picked: i64, // selected recurring template, 0 = custom
    name: String,
    amount: String,
    date: String,
    note: String,
    editing: bool,
}

impl ExpenseForm {
    fn new_expense() -> Self {
        Self {
            date: dates::today(),
            amount: "0".into(),
            ..Default::default()
        }
    }

    /// A new expense pre-filled from a recurring template.
    fn from_recurring(r: &RecurringExpense) -> Self {
        Self {
            picked: r.id,
            name: r.name.clone(),
            amount: format!("{}", r.amount),
            date: dates::today(),
            ..Default::default()
        }
    }

    fn from(e: &Expense) -> Self {
        Self {
            id: e.id,
            picked: 0,
            name: e.name.clone(),
            amount: format!("{}", e.amount),
            date: e.date.clone(),
            note: e.note.clone().unwrap_or_default(),
            editing: true,
        }
    }
}

#[derive(Default)]
struct RecurringForm {
    id: i64,
    name: String,
    amount: String,
    editing: bool,
}

impl RecurringForm {
    fn new() -> Self {
        Self {
            amount: "0".into(),
            ..Default::default()
        }
    }

    fn from(r: &RecurringExpense) -> Self {
        Self {
            id: r.id,
            name: r.name.clone(),
            amount: format!("{}", r.amount),
            editing: true,
        }
    }
}

enum Dialog {
    None,
    Edit(ExpenseForm),
    ConfirmDelete { id: i64 },
    EditRecurring(RecurringForm),
    ConfirmDeleteRecurring { id: i64 },
}

pub struct ExpensesState {
    rows: Vec<Expense>,
    recurring: Vec<RecurringExpense>,
    tab: Tab,
    dirty: bool,
    dialog: Dialog,
}

impl Default for ExpensesState {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            recurring: Vec::new(),
            tab: Tab::Expenses,
            dirty: true,
            dialog: Dialog::None,
        }
    }
}

enum Action {
    Edit(i64),
    AskDelete(i64),
    EditRecurring(i64),
    AskDeleteRecurring(i64),
    LogRecurring(i64),
}

impl ExpensesState {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    fn reload(&mut self, repo: &Repository) {
        self.rows = repo.list_expenses().unwrap_or_default();
        self.recurring = repo.list_recurring_expenses().unwrap_or_default();
        self.dirty = false;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) {
        if self.dirty {
            self.reload(repo);
        }

        let mut action: Option<Action> = None;

        ui.horizontal(|ui| {
            ui.heading("Expenses");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match self.tab {
                    Tab::Expenses => {
                        if ui.button("+ Add expense").clicked() {
                            self.dialog = Dialog::Edit(ExpenseForm::new_expense());
                        }
                    }
                    Tab::Recurring => {
                        if ui.button("+ Add recurring").clicked() {
                            self.dialog = Dialog::EditRecurring(RecurringForm::new());
                        }
                    }
                }
            });
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.selectable_label(self.tab == Tab::Expenses, "Expenses").clicked() {
                self.tab = Tab::Expenses;
            }
            if ui.selectable_label(self.tab == Tab::Recurring, "Recurring").clicked() {
                self.tab = Tab::Recurring;
            }
        });
        ui.separator();
        ui.add_space(6.0);

        match self.tab {
            Tab::Expenses => action = self.expenses_table(ui).or(action),
            Tab::Recurring => action = self.recurring_table(ui).or(action),
        }

        if let Some(a) = action {
            self.handle_action(a);
        }

        self.draw_dialog(ui.ctx(), repo);
    }

    fn expenses_table(&self, ui: &mut egui::Ui) -> Option<Action> {
        if self.rows.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No expenses yet.").weak());
            });
            return None;
        }
        let mut action: Option<Action> = None;
        crate::ui::wide_table(ui, 720.0, |ui| {
            TableBuilder::new(ui)
                .striped(false)
                .resizable(false)
                .column(Column::auto().at_least(120.0))
                .column(Column::auto().at_least(160.0))
                .column(Column::auto().at_least(100.0))
                .column(Column::remainder().at_least(200.0))
                .column(Column::auto().at_least(140.0))
                .header(30.0, |mut h| {
                    h.col(|ui| {
                        ui.strong("Date");
                    });
                    h.col(|ui| {
                        ui.strong("Name");
                    });
                    h.col(|ui| {
                        ui.strong("Amount");
                    });
                    h.col(|ui| {
                        ui.strong("Note");
                    });
                    h.col(|ui| {
                        ui.strong("Actions");
                    });
                })
                .body(|body| {
                    body.rows(34.0, self.rows.len(), |mut row| {
                        let e = &self.rows[row.index()];
                        row.col(|ui| {
                            ui.label(&e.date);
                        });
                        row.col(|ui| {
                            ui.label(&e.name);
                        });
                        row.col(|ui| {
                            ui.label(format!("{:.2}", e.amount));
                        });
                        row.col(|ui| {
                            ui.label(e.note.as_deref().unwrap_or(""));
                        });
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                if ui.small_button("Edit").clicked() {
                                    action = Some(Action::Edit(e.id));
                                }
                                if ui.small_button("Delete").clicked() {
                                    action = Some(Action::AskDelete(e.id));
                                }
                            });
                        });
                    });
                });
        });
        action
    }

    fn recurring_table(&self, ui: &mut egui::Ui) -> Option<Action> {
        if self.recurring.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No recurring expenses yet.").weak());
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Save the bills you pay every month — rent, salaries, utilities — \
                         then pick them when adding an expense.",
                    )
                    .weak()
                    .size(12.0),
                );
            });
            return None;
        }
        let mut action: Option<Action> = None;
        crate::ui::wide_table(ui, 620.0, |ui| {
            TableBuilder::new(ui)
                .striped(false)
                .resizable(false)
                .column(Column::auto().at_least(220.0))
                .column(Column::auto().at_least(120.0))
                .column(Column::remainder().at_least(220.0))
                .header(30.0, |mut h| {
                    h.col(|ui| {
                        ui.strong("Name");
                    });
                    h.col(|ui| {
                        ui.strong("Amount");
                    });
                    h.col(|ui| {
                        ui.strong("Actions");
                    });
                })
                .body(|body| {
                    body.rows(34.0, self.recurring.len(), |mut row| {
                        let r = &self.recurring[row.index()];
                        row.col(|ui| {
                            ui.label(&r.name);
                        });
                        row.col(|ui| {
                            ui.label(format!("{:.2}", r.amount));
                        });
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                if ui.small_button("Log expense").clicked() {
                                    action = Some(Action::LogRecurring(r.id));
                                }
                                if ui.small_button("Edit").clicked() {
                                    action = Some(Action::EditRecurring(r.id));
                                }
                                if ui.small_button("Delete").clicked() {
                                    action = Some(Action::AskDeleteRecurring(r.id));
                                }
                            });
                        });
                    });
                });
        });
        action
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Edit(id) => {
                if let Some(e) = self.rows.iter().find(|e| e.id == id) {
                    self.dialog = Dialog::Edit(ExpenseForm::from(e));
                }
            }
            Action::AskDelete(id) => {
                self.dialog = Dialog::ConfirmDelete { id };
            }
            Action::EditRecurring(id) => {
                if let Some(r) = self.recurring.iter().find(|r| r.id == id) {
                    self.dialog = Dialog::EditRecurring(RecurringForm::from(r));
                }
            }
            Action::AskDeleteRecurring(id) => {
                self.dialog = Dialog::ConfirmDeleteRecurring { id };
            }
            Action::LogRecurring(id) => {
                if let Some(r) = self.recurring.iter().find(|r| r.id == id) {
                    self.dialog = Dialog::Edit(ExpenseForm::from_recurring(r));
                }
            }
        }
    }

    fn draw_dialog(&mut self, ctx: &egui::Context, repo: &mut Repository) {
        let mut close = false;
        let recurring = self.recurring.clone();
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::Edit(form) => {
                egui::Window::new(if form.editing { "Edit expense" } else { "Add expense" })
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        if !form.editing && !recurring.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label("From recurring");
                                let selected = if form.picked == 0 {
                                    "Custom (write your own)".to_string()
                                } else {
                                    recurring
                                        .iter()
                                        .find(|r| r.id == form.picked)
                                        .map(|r| r.name.clone())
                                        .unwrap_or_else(|| "Custom (write your own)".into())
                                };
                                egui::ComboBox::from_id_salt("exp_recurring")
                                    .selected_text(selected)
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_label(form.picked == 0, "Custom (write your own)")
                                            .clicked()
                                        {
                                            form.picked = 0;
                                        }
                                        for r in &recurring {
                                            if ui
                                                .selectable_label(
                                                    form.picked == r.id,
                                                    format!("{} — {:.2}", r.name, r.amount),
                                                )
                                                .clicked()
                                            {
                                                form.picked = r.id;
                                                form.name = r.name.clone();
                                                form.amount = format!("{}", r.amount);
                                            }
                                        }
                                    });
                            });
                            ui.separator();
                        }
                        egui::Grid::new("exp_form").num_columns(2).show(ui, |ui| {
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
                        let valid = !form.name.trim().is_empty()
                            && !form.date.trim().is_empty()
                            && form.amount.parse::<f64>().is_ok();
                        ui.horizontal(|ui| {
                            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                                let e = Expense {
                                    id: form.id,
                                    name: form.name.trim().to_string(),
                                    amount: form.amount.parse().unwrap_or(0.0),
                                    date: form.date.trim().to_string(),
                                    note: if form.note.trim().is_empty() {
                                        None
                                    } else {
                                        Some(form.note.trim().to_string())
                                    },
                                };
                                let r = if form.editing {
                                    repo.update_expense(&e)
                                } else {
                                    repo.insert_expense(&e).map(|_| ())
                                };
                                if r.is_ok() {
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
            Dialog::ConfirmDelete { id } => {
                egui::Window::new("Delete expense")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Delete this expense? This cannot be undone.");
                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                if repo.delete_expense(*id).is_ok() {
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
            Dialog::EditRecurring(form) => {
                egui::Window::new(if form.editing {
                    "Edit recurring"
                } else {
                    "Add recurring"
                })
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("recurring_form").num_columns(2).show(ui, |ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut form.name);
                        ui.end_row();
                        ui.label("Amount");
                        ui.text_edit_singleline(&mut form.amount);
                        ui.end_row();
                    });
                    let valid =
                        !form.name.trim().is_empty() && form.amount.parse::<f64>().is_ok();
                    ui.horizontal(|ui| {
                        if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                            let r = RecurringExpense {
                                id: form.id,
                                name: form.name.trim().to_string(),
                                amount: form.amount.parse().unwrap_or(0.0),
                            };
                            let res = if form.editing {
                                repo.update_recurring_expense(&r)
                            } else {
                                repo.insert_recurring_expense(&r).map(|_| ())
                            };
                            if res.is_ok() {
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
            Dialog::ConfirmDeleteRecurring { id } => {
                egui::Window::new("Delete recurring")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(
                            "Delete this recurring expense? Expenses you already logged from it \
                             are kept.",
                        );
                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                if repo.delete_recurring_expense(*id).is_ok() {
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
        }
    }
}
