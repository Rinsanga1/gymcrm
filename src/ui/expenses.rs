use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::core::dates;
use crate::core::models::Expense;
use crate::core::Repository;

#[derive(Default)]
struct ExpenseForm {
    id: i64,
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

    fn from(e: &Expense) -> Self {
        Self {
            id: e.id,
            name: e.name.clone(),
            amount: format!("{}", e.amount),
            date: e.date.clone(),
            note: e.note.clone().unwrap_or_default(),
            editing: true,
        }
    }
}

enum Dialog {
    None,
    Edit(ExpenseForm),
    ConfirmDelete { id: i64 },
}

pub struct ExpensesState {
    rows: Vec<Expense>,
    dirty: bool,
    dialog: Dialog,
}

impl Default for ExpensesState {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            dirty: true,
            dialog: Dialog::None,
        }
    }
}

enum Action {
    Edit(i64),
    AskDelete(i64),
}

impl ExpensesState {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    fn reload(&mut self, repo: &Repository) {
        self.rows = repo.list_expenses().unwrap_or_default();
        self.dirty = false;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) {
        if self.dirty {
            self.reload(repo);
        }

        ui.horizontal(|ui| {
            ui.heading("Expenses");
            ui.add_space(12.0);
            if ui.button("+ Add expense").clicked() {
                self.dialog = Dialog::Edit(ExpenseForm::new_expense());
            }
        });
        ui.separator();

        let mut action: Option<Action> = None;
        if self.rows.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No expenses yet.").weak());
            });
        } else {
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
        }

        if let Some(a) = action {
            match a {
                Action::Edit(id) => {
                    if let Some(e) = self.rows.iter().find(|e| e.id == id) {
                        self.dialog = Dialog::Edit(ExpenseForm::from(e));
                    }
                }
                Action::AskDelete(id) => {
                    self.dialog = Dialog::ConfirmDelete { id };
                }
            }
        }

        self.draw_dialog(ui.ctx(), repo);
    }

    fn draw_dialog(&mut self, ctx: &egui::Context, repo: &mut Repository) {
        let mut close = false;
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::Edit(form) => {
                egui::Window::new(if form.editing { "Edit expense" } else { "Add expense" })
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
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
        }
        if close {
            self.dialog = Dialog::None;
        }
    }
}
