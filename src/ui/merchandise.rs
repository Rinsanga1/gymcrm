use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::core::dates;
use crate::core::models::{Product, Sale};
use crate::core::Repository;

const LOW_STOCK_THRESHOLD: i64 = 5;

#[derive(Default)]
struct ProductForm {
    id: i64,
    name: String,
    price: String,
    stock: String,
    active: bool,
    editing: bool,
}

impl ProductForm {
    fn new_product() -> Self {
        Self {
            active: true,
            stock: "0".into(),
            price: "0".into(),
            ..Default::default()
        }
    }

    fn from(p: &Product) -> Self {
        Self {
            id: p.id,
            name: p.name.clone(),
            price: format!("{}", p.price),
            stock: format!("{}", p.stock),
            active: p.active,
            editing: true,
        }
    }
}

#[derive(Clone, Default)]
struct SaleLine {
    product_id: i64,
    qty: String,
}

struct SaleForm {
    id: i64, // 0 = new
    date: String,
    lines: Vec<SaleLine>,
    editing: bool,
}

impl SaleForm {
    fn new_sale() -> Self {
        Self {
            id: 0,
            date: dates::today(),
            lines: vec![SaleLine::default()],
            editing: false,
        }
    }
}

enum Dialog {
    None,
    Product(ProductForm),
    Sale(SaleForm),
    ConfirmDeleteProduct { id: i64, name: String },
    ConfirmDeleteSale { id: i64 },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Products,
    Sales,
}

pub struct MerchandiseState {
    tab: Tab,
    products: Vec<Product>,
    sales: Vec<Sale>,
    dirty: bool,
    dialog: Dialog,
}

impl Default for MerchandiseState {
    fn default() -> Self {
        Self {
            tab: Tab::Products,
            products: Vec::new(),
            sales: Vec::new(),
            dirty: true,
            dialog: Dialog::None,
        }
    }
}

enum Action {
    NewProduct,
    EditProduct(i64),
    AskDeleteProduct(i64, String),
    NewSale,
    EditSale(i64),
    AskDeleteSale(i64),
}

impl MerchandiseState {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    fn reload(&mut self, repo: &Repository) {
        self.products = repo.list_products().unwrap_or_default();
        self.sales = repo.list_sales().unwrap_or_default();
        self.dirty = false;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) {
        if self.dirty {
            self.reload(repo);
        }

        ui.horizontal(|ui| {
            ui.heading("Merchandise");
            ui.add_space(12.0);
            ui.selectable_value(&mut self.tab, Tab::Products, "Products");
            ui.selectable_value(&mut self.tab, Tab::Sales, "Sales");
        });
        ui.separator();

        let mut action: Option<Action> = None;

        match self.tab {
            Tab::Products => {
                ui.horizontal(|ui| {
                    if ui.button("+ Add product").clicked() {
                        action = Some(Action::NewProduct);
                    }
                    let low = self
                        .products
                        .iter()
                        .filter(|p| p.active && p.stock <= LOW_STOCK_THRESHOLD)
                        .count();
                    if low > 0 {
                        ui.add_space(12.0);
                        ui.colored_label(
                            egui::Color32::from_rgb(220, 140, 40),
                            format!("{} product(s) low on stock (≤ {})", low, LOW_STOCK_THRESHOLD),
                        );
                    }
                });
                ui.add_space(6.0);
                action = self.products_table(ui).or(action);
            }
            Tab::Sales => {
                ui.horizontal(|ui| {
                    if ui.button("+ Record sale").clicked() {
                        action = Some(Action::NewSale);
                    }
                });
                ui.add_space(6.0);
                action = self.sales_table(ui).or(action);
            }
        }

        if let Some(a) = action {
            self.handle_action(a, repo);
        }

        self.draw_dialog(ui.ctx(), repo);
    }

    fn products_table(&self, ui: &mut egui::Ui) -> Option<Action> {
        if self.products.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No products yet.").weak());
            });
            return None;
        }
        let mut action: Option<Action> = None;
        let row_height = 34.0;
        TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .column(Column::auto().at_least(180.0))
            .column(Column::auto().at_least(80.0))
            .column(Column::auto().at_least(80.0))
            .column(Column::auto().at_least(80.0))
            .column(Column::remainder().at_least(180.0))
            .header(30.0, |mut h| {
                h.col(|ui| {
                    ui.strong("Name");
                });
                h.col(|ui| {
                    ui.strong("Price");
                });
                h.col(|ui| {
                    ui.strong("Stock");
                });
                h.col(|ui| {
                    ui.strong("Status");
                });
                h.col(|ui| {
                    ui.strong("Actions");
                });
            })
            .body(|body| {
                body.rows(row_height, self.products.len(), |mut row| {
                    let p = &self.products[row.index()];
                    row.col(|ui| {
                        ui.label(&p.name);
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.2}", p.price));
                    });
                    row.col(|ui| {
                        if p.active && p.stock <= LOW_STOCK_THRESHOLD {
                            ui.colored_label(
                                egui::Color32::from_rgb(220, 140, 40),
                                format!("{} (low)", p.stock),
                            );
                        } else {
                            ui.label(format!("{}", p.stock));
                        }
                    });
                    row.col(|ui| {
                        ui.label(if p.active { "Active" } else { "Inactive" });
                    });
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            if ui.small_button("Edit").clicked() {
                                action = Some(Action::EditProduct(p.id));
                            }
                            if ui.small_button("Delete").clicked() {
                                action = Some(Action::AskDeleteProduct(p.id, p.name.clone()));
                            }
                        });
                    });
                });
            });
        action
    }

    fn sales_table(&self, ui: &mut egui::Ui) -> Option<Action> {
        if self.sales.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No sales recorded yet.").weak());
            });
            return None;
        }
        let mut action: Option<Action> = None;
        let row_height = 34.0;
        TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .column(Column::auto().at_least(120.0))
            .column(Column::auto().at_least(120.0))
            .column(Column::remainder().at_least(180.0))
            .header(30.0, |mut h| {
                h.col(|ui| {
                    ui.strong("Date");
                });
                h.col(|ui| {
                    ui.strong("Total");
                });
                h.col(|ui| {
                    ui.strong("Actions");
                });
            })
            .body(|body| {
                body.rows(row_height, self.sales.len(), |mut row| {
                    let s = &self.sales[row.index()];
                    row.col(|ui| {
                        ui.label(&s.date);
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.2}", s.total));
                    });
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            if ui.small_button("Edit").clicked() {
                                action = Some(Action::EditSale(s.id));
                            }
                            if ui.small_button("Delete").clicked() {
                                action = Some(Action::AskDeleteSale(s.id));
                            }
                        });
                    });
                });
            });
        action
    }

    fn handle_action(&mut self, action: Action, repo: &mut Repository) {
        match action {
            Action::NewProduct => {
                self.dialog = Dialog::Product(ProductForm::new_product());
            }
            Action::EditProduct(id) => {
                if let Some(p) = self.products.iter().find(|p| p.id == id) {
                    self.dialog = Dialog::Product(ProductForm::from(p));
                }
            }
            Action::AskDeleteProduct(id, name) => {
                self.dialog = Dialog::ConfirmDeleteProduct { id, name };
            }
            Action::NewSale => {
                if self.products.iter().any(|p| p.active) {
                    let mut f = SaleForm::new_sale();
                    f.lines[0].product_id = self
                        .products
                        .iter()
                        .find(|p| p.active)
                        .map(|p| p.id)
                        .unwrap_or(0);
                    f.lines[0].qty = "1".into();
                    self.dialog = Dialog::Sale(f);
                }
            }
            Action::EditSale(id) => {
                let items = repo.sale_items(id).unwrap_or_default();
                if let Some(s) = self.sales.iter().find(|s| s.id == id).cloned() {
                    let lines = items
                        .iter()
                        .filter_map(|it| {
                            it.product_id.map(|pid| SaleLine {
                                product_id: pid,
                                qty: format!("{}", it.qty),
                            })
                        })
                        .collect::<Vec<_>>();
                    self.dialog = Dialog::Sale(SaleForm {
                        id,
                        date: s.date,
                        lines: if lines.is_empty() {
                            vec![SaleLine::default()]
                        } else {
                            lines
                        },
                        editing: true,
                    });
                }
            }
            Action::AskDeleteSale(id) => {
                self.dialog = Dialog::ConfirmDeleteSale { id };
            }
        }
    }

    fn draw_dialog(&mut self, ctx: &egui::Context, repo: &mut Repository) {
        let mut close = false;
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::Product(form) => {
                egui::Window::new(if form.editing { "Edit product" } else { "Add product" })
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        egui::Grid::new("prod_form").num_columns(2).show(ui, |ui| {
                            ui.label("Name *");
                            ui.text_edit_singleline(&mut form.name);
                            ui.end_row();
                            ui.label("Price");
                            ui.text_edit_singleline(&mut form.price);
                            ui.end_row();
                            ui.label("Stock");
                            ui.text_edit_singleline(&mut form.stock);
                            ui.end_row();
                            ui.label("Active");
                            ui.checkbox(&mut form.active, "");
                            ui.end_row();
                        });
                        ui.horizontal(|ui| {
                            let valid = !form.name.trim().is_empty()
                                && form.price.parse::<f64>().is_ok()
                                && form.stock.parse::<i64>().is_ok();
                            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                                let p = Product {
                                    id: form.id,
                                    name: form.name.trim().to_string(),
                                    price: form.price.parse().unwrap_or(0.0),
                                    stock: form.stock.parse().unwrap_or(0),
                                    active: form.active,
                                };
                                let r = if form.editing {
                                    repo.update_product(&p).map(|_| p.id)
                                } else {
                                    repo.insert_product(&p)
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
            Dialog::Sale(form) => {
                let products = self
                    .products
                    .iter()
                    .filter(|p| p.active)
                    .cloned()
                    .collect::<Vec<_>>();
                let title = if form.editing { "Edit sale" } else { "Record sale" };
                egui::Window::new(title)
                    .collapsible(false)
                    .resizable(true)
                    .default_width(420.0)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Date");
                            crate::ui::date_edit(ui, &mut form.date);
                        });
                        ui.separator();
                        let mut remove_idx: Option<usize> = None;
                        let mut total = 0.0f64;
                        for (i, line) in form.lines.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                let name = products
                                    .iter()
                                    .find(|p| p.id == line.product_id)
                                    .map(|p| p.name.clone())
                                    .unwrap_or_else(|| "(pick product)".into());
                                egui::ComboBox::from_id_salt(("prod_pick", i))
                                    .selected_text(name)
                                    .show_ui(ui, |ui| {
                                        for p in &products {
                                            ui.selectable_value(
                                                &mut line.product_id,
                                                p.id,
                                                format!("{} (Rs {:.0}, stock {})", p.name, p.price, p.stock),
                                            );
                                        }
                                    });
                                ui.label("qty");
                                ui.add(egui::TextEdit::singleline(&mut line.qty).desired_width(50.0));
                                if let (Some(p), Ok(q)) = (
                                    products.iter().find(|p| p.id == line.product_id),
                                    line.qty.parse::<i64>(),
                                ) {
                                    let sub = p.price * q as f64;
                                    total += sub;
                                    ui.label(format!("= Rs {:.2}", sub));
                                }
                                if ui.small_button("✕").clicked() {
                                    remove_idx = Some(i);
                                }
                            });
                        }
                        if let Some(i) = remove_idx {
                            if form.lines.len() > 1 {
                                form.lines.remove(i);
                            }
                        }
                        ui.horizontal(|ui| {
                            if ui.button("+ Add line").clicked() {
                                form.lines.push(SaleLine {
                                    product_id: products.first().map(|p| p.id).unwrap_or(0),
                                    qty: "1".into(),
                                });
                            }
                            ui.add_space(12.0);
                            ui.strong(format!("Total: Rs {:.2}", total));
                        });
                        ui.separator();
                        let items: Vec<(i64, i64, f64)> = form
                            .lines
                            .iter()
                            .filter_map(|l| {
                                let p = products.iter().find(|p| p.id == l.product_id)?;
                                let q: i64 = l.qty.parse().ok()?;
                                if q <= 0 {
                                    return None;
                                }
                                Some((p.id, q, p.price))
                            })
                            .collect();
                        let valid = !items.is_empty() && !form.date.trim().is_empty();
                        ui.horizontal(|ui| {
                            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                                let r = if form.editing {
                                    repo.update_sale(form.id, form.date.trim(), &items)
                                } else {
                                    repo.record_sale(form.date.trim(), &items).map(|_| ())
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
            Dialog::ConfirmDeleteProduct { id, name } => {
                egui::Window::new("Delete product")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(format!("Delete \"{}\"? This cannot be undone.", name));
                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                if repo.delete_product(*id).is_ok() {
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
            Dialog::ConfirmDeleteSale { id } => {
                egui::Window::new("Delete sale")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Delete this sale? Stock will be restored.");
                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                if repo.delete_sale(*id).is_ok() {
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
