use std::collections::HashMap;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::core::dates::{self, MonthFilter};
use crate::core::models::{Product, Sale};
use crate::core::Repository;

const LOW_STOCK_THRESHOLD: i64 = 5;

/// Like `ui::wide_table` (horizontal scroll so wide columns stay reachable) but
/// hugs its content height instead of filling the panel — so a second table
/// stacked below it stays on screen. Each table keeps its own vertical scroll.
fn scroll_table(ui: &mut egui::Ui, min_width: f32, add: impl FnOnce(&mut egui::Ui)) {
    let target = min_width.max(ui.available_width());
    egui::ScrollArea::horizontal()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.set_width(target);
            add(ui);
        });
}

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

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Products,
    Sales,
}

pub struct MerchandiseState {
    products: Vec<Product>,
    sales: Vec<Sale>,
    sale_summaries: HashMap<i64, String>,
    filter: MonthFilter,
    years: Vec<i32>,
    tab: Tab,
    dirty: bool,
    dialog: Dialog,
    // A sale to open for editing on the next `show` (tapped from Transactions).
    pending_edit_sale: Option<i64>,
}

impl Default for MerchandiseState {
    fn default() -> Self {
        Self {
            products: Vec::new(),
            sales: Vec::new(),
            sale_summaries: HashMap::new(),
            filter: MonthFilter::current(),
            years: Vec::new(),
            tab: Tab::Products,
            dirty: true,
            dialog: Dialog::None,
            pending_edit_sale: None,
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

    /// Open a specific sale for editing — used when a Transactions sale row is
    /// tapped.
    pub fn focus_sale(&mut self, id: i64) {
        self.tab = Tab::Sales;
        self.dirty = true;
        self.pending_edit_sale = Some(id);
    }

    fn reload(&mut self, repo: &Repository) {
        self.products = repo.list_products().unwrap_or_default();
        self.years =
            crate::ui::year_options(repo.sale_years().unwrap_or_default(), self.filter.year);
        let (start, end) = self.filter.range();
        self.sales = repo.list_sales_between(&start, &end).unwrap_or_default();
        self.sale_summaries = self.build_summaries(repo, &start, &end);
        self.dirty = false;
    }

    /// One-line "2× Water · 1× Protein" summary per sale, built for the whole
    /// period in a single query so a 800-sale period is one round-trip, not 800.
    fn build_summaries(&self, repo: &Repository, start: &str, end: &str) -> HashMap<i64, String> {
        let names: HashMap<i64, &str> =
            self.products.iter().map(|p| (p.id, p.name.as_str())).collect();
        let mut out: HashMap<i64, String> = HashMap::new();
        for (sale_id, product_id, qty) in
            repo.sale_item_lines_between(start, end).unwrap_or_default()
        {
            let name = product_id
                .and_then(|pid| names.get(&pid).copied())
                .unwrap_or("(removed)");
            let line = out.entry(sale_id).or_default();
            if !line.is_empty() {
                line.push_str(" · ");
            }
            line.push_str(&format!("{}× {}", qty, name));
        }
        out
    }

    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) {
        if self.dirty {
            self.reload(repo);
        }
        if let Some(id) = self.pending_edit_sale.take() {
            self.handle_action(Action::EditSale(id), repo);
        }

        let currency = repo.currency();
        let mut action: Option<Action> = None;

        ui.horizontal(|ui| {
            ui.heading("Shop");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ Record sale").clicked() {
                    action = Some(Action::NewSale);
                }
                if ui.button("+ Add product").clicked() {
                    action = Some(Action::NewProduct);
                }
            });
        });
        ui.separator();

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.selectable_label(self.tab == Tab::Products, "Products").clicked() {
                self.tab = Tab::Products;
            }
            if ui.selectable_label(self.tab == Tab::Sales, "Sales").clicked() {
                self.tab = Tab::Sales;
            }
        });
        ui.separator();
        ui.add_space(6.0);

        match self.tab {
            Tab::Products => {
                let low = self
                    .products
                    .iter()
                    .filter(|p| p.active && p.stock <= LOW_STOCK_THRESHOLD)
                    .count();
                if low > 0 {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 140, 40),
                        format!("{} product(s) running low", low),
                    );
                    ui.add_space(4.0);
                }
                action = ui
                    .push_id("merch_products", |ui| self.products_table(ui, &currency))
                    .inner
                    .or(action);
            }
            Tab::Sales => {
                ui.horizontal(|ui| {
                    if crate::ui::year_month_filter(
                        ui,
                        "sales_filter",
                        &mut self.filter,
                        &self.years,
                    ) {
                        self.dirty = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.weak("Full history lives in Transactions.");
                    });
                });
                ui.add_space(6.0);
                action = ui
                    .push_id("merch_sales", |ui| self.sales_table(ui, &currency))
                    .inner
                    .or(action);
            }
        }

        if let Some(a) = action {
            self.handle_action(a, repo);
        }

        self.draw_dialog(ui.ctx(), repo);
    }

    fn products_table(&self, ui: &mut egui::Ui, currency: &str) -> Option<Action> {
        if self.products.is_empty() {
            let mut action = None;
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No products yet.").weak());
                ui.add_space(6.0);
                if ui.button("+ Add your first product").clicked() {
                    action = Some(Action::NewProduct);
                }
            });
            return action;
        }
        let mut action: Option<Action> = None;
        let row_height = 34.0;
        let table_height = ui.available_height().max(row_height * 5.0);
        scroll_table(ui, 620.0, |ui| {
        TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .max_scroll_height(table_height)
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
                        ui.label(crate::ui::money(currency, p.price));
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
                        ui.label(if p.active { "Available" } else { "Hidden" });
                    });
                    row.col(|ui| {
                        ui.menu_button("⋯", |ui| {
                            if ui.button("Edit").clicked() {
                                action = Some(Action::EditProduct(p.id));
                                ui.close();
                            }
                            if ui.button("Delete").clicked() {
                                action = Some(Action::AskDeleteProduct(p.id, p.name.clone()));
                                ui.close();
                            }
                        });
                    });
                });
            });
        });
        action
    }

    fn sales_table(&self, ui: &mut egui::Ui, currency: &str) -> Option<Action> {
        if self.sales.is_empty() {
            let mut action = None;
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(format!("No sales in {}.", self.filter.label())).weak());
                ui.add_space(6.0);
                if ui.button("+ Record a sale").clicked() {
                    action = Some(Action::NewSale);
                }
            });
            return action;
        }
        let mut action: Option<Action> = None;
        let row_height = 34.0;
        let table_height = ui.available_height().max(row_height * 5.0);
        scroll_table(ui, 640.0, |ui| {
        TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .max_scroll_height(table_height)
            .column(Column::auto().at_least(120.0))
            .column(Column::auto().at_least(200.0).clip(true))
            .column(Column::auto().at_least(120.0))
            .column(Column::remainder().at_least(180.0))
            .header(30.0, |mut h| {
                h.col(|ui| {
                    ui.strong("Date");
                });
                h.col(|ui| {
                    ui.strong("Items");
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
                        let summary = self
                            .sale_summaries
                            .get(&s.id)
                            .map(String::as_str)
                            .unwrap_or("");
                        ui.label(egui::RichText::new(summary).weak())
                            .on_hover_text(summary);
                    });
                    row.col(|ui| {
                        ui.label(crate::ui::money(currency, s.total));
                    });
                    row.col(|ui| {
                        ui.menu_button("⋯", |ui| {
                            if ui.button("Edit").clicked() {
                                action = Some(Action::EditSale(s.id));
                                ui.close();
                            }
                            if ui.button("Delete").clicked() {
                                action = Some(Action::AskDeleteSale(s.id));
                                ui.close();
                            }
                        });
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
                // Fall back to a direct lookup so a sale outside the current month
                // filter (e.g. tapped from Transactions) still opens.
                let sale = self
                    .sales
                    .iter()
                    .find(|s| s.id == id)
                    .cloned()
                    .or_else(|| repo.list_sales().ok().and_then(|v| v.into_iter().find(|s| s.id == id)));
                if let Some(s) = sale {
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
        let currency = repo.currency();
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::Product(form) => {
                egui::Window::new(if form.editing { "Edit product" } else { "Add product" })
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        egui::Grid::new("prod_form").num_columns(2).show(ui, |ui| {
                            ui.label("Name");
                            ui.text_edit_singleline(&mut form.name);
                            ui.end_row();
                            ui.label("Price");
                            ui.text_edit_singleline(&mut form.price);
                            ui.end_row();
                            ui.label("Stock");
                            ui.text_edit_singleline(&mut form.stock);
                            ui.end_row();
                            ui.label("Available");
                            ui.checkbox(&mut form.active, "");
                            ui.end_row();
                        });
                        let valid = !form.name.trim().is_empty()
                            && form.price.parse::<f64>().is_ok()
                            && form.stock.parse::<i64>().is_ok();
                        ui.horizontal(|ui| {
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
                            if !valid {
                                ui.colored_label(
                                    egui::Color32::from_rgb(210, 120, 40),
                                    "Name required; price and stock must be numbers",
                                );
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
                                                format!("{} ({}, stock {})", p.name, crate::ui::money(&currency, p.price), p.stock),
                                            );
                                        }
                                    });
                                ui.label("Qty");
                                ui.add(egui::TextEdit::singleline(&mut line.qty).desired_width(50.0));
                                if let (Some(p), Ok(q)) = (
                                    products.iter().find(|p| p.id == line.product_id),
                                    line.qty.parse::<i64>(),
                                ) {
                                    let sub = p.price * q as f64;
                                    total += sub;
                                    ui.label(format!("= {}", crate::ui::money(&currency, sub)));
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
                            ui.strong(format!("Total: {}", crate::ui::money(&currency, total)));
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
                            if !valid {
                                ui.colored_label(
                                    egui::Color32::from_rgb(210, 120, 40),
                                    "Add at least one line with a quantity",
                                );
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
