use eframe::egui::{self, Color32, FontId, RichText};

use crate::core::dates;
use crate::core::Repository;

const INCOME: Color32 = Color32::from_rgb(45, 170, 95);
const OUTGOING: Color32 = Color32::from_rgb(210, 90, 90);

struct MonthGroup {
    label: String,
    net: f64,
    items: Vec<crate::core::models::Txn>,
}

use crate::core::dates::MonthFilter;

pub struct TransactionsState {
    filter: MonthFilter,
    years: Vec<i32>,
    groups: Vec<MonthGroup>,
    dirty: bool,
}

impl Default for TransactionsState {
    fn default() -> Self {
        Self {
            filter: MonthFilter::current(),
            years: Vec::new(),
            groups: Vec::new(),
            dirty: true,
        }
    }
}

impl TransactionsState {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    fn reload(&mut self, repo: &Repository) {
        self.years =
            crate::ui::year_options(repo.transaction_years().unwrap_or_default(), self.filter.year);
        let (start, end) = self.filter.range();
        let txns = repo
            .list_transactions(Some(&start), Some(&end))
            .unwrap_or_default();
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
        ui.weak("A read-only record of every payment, sale, and expense. Edit these where they live in Members, Merchandise, and Expenses.");
        ui.add_space(8.0);

        if crate::ui::year_month_filter(ui, "txn_filter", &mut self.filter, &self.years) {
            self.dirty = true;
        }
        ui.add_space(8.0);

        let currency = repo.currency();

        if self.groups.is_empty() {
            let msg = format!("No transactions in {}.", self.filter.label());
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(msg).weak());
            });
            return;
        }

        let muted = crate::ui::theme::text_muted(ui.visuals());
        let text_col = ui.visuals().text_color();
        let hover_bg = ui.visuals().widgets.hovered.weak_bg_fill;

        // Fixed column widths so dates, descriptions and — crucially — amounts
        // line up in scannable columns. The ledger is one centered column capped
        // at `cap` so amounts don't drift into dead space on wide windows.
        let spacing = ui.spacing().item_spacing.x;
        let pad = 10.0;
        let date_w = 84.0;
        let amt_w = 130.0;
        let cap = 900.0;
        let total = (ui.available_width() - 24.0).clamp(440.0, cap);
        let desc_w = (total - pad - date_w - amt_w - spacing * 2.0).max(120.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(total);

                    // Column headers
                    ui.allocate_ui_with_layout(
                        egui::vec2(total, 14.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.add_space(pad);
                            ui.add_sized([date_w, 14.0], header_label("Date", muted, egui::Align::LEFT));
                            ui.add_sized([desc_w, 14.0], header_label("Description", muted, egui::Align::LEFT));
                            ui.add_sized([amt_w, 14.0], header_label("Amount", muted, egui::Align::RIGHT));
                        },
                    );
                    ui.add_space(6.0);
                    ui.separator();
                    ui.spacing_mut().item_spacing.y = 0.0;

                    for (gi, g) in self.groups.iter().enumerate() {
                        ui.add_space(if gi == 0 { 10.0 } else { 22.0 });
                        // Month header: label spans date+description; net sits in
                        // the amount column so it aligns with every row below.
                        ui.allocate_ui_with_layout(
                            egui::vec2(total, 18.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.add_space(pad);
                                ui.add_sized(
                                    [date_w + spacing + desc_w, 18.0],
                                    egui::Label::new(
                                        RichText::new(g.label.to_uppercase())
                                            .color(muted)
                                            .size(11.0),
                                    )
                                    .halign(egui::Align::LEFT),
                                );
                                ui.add_sized(
                                    [amt_w, 18.0],
                                    egui::Label::new(amount_text(g.net, &currency).size(12.5))
                                        .halign(egui::Align::RIGHT),
                                );
                            },
                        );
                        ui.add_space(6.0);

                        for t in g.items.iter() {
                            let row_h = if t.detail.is_some() { 46.0 } else { 34.0 };
                            let row_rect = egui::Rect::from_min_size(
                                ui.next_widget_position(),
                                egui::vec2(total, row_h),
                            );
                            if ui.rect_contains_pointer(row_rect) {
                                ui.painter().rect_filled(row_rect, 6.0, hover_bg);
                            }
                            ui.allocate_ui_with_layout(
                                egui::vec2(total, row_h),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.add_space(pad);
                                    ui.add_sized(
                                        [date_w, row_h],
                                        egui::Label::new(
                                            RichText::new(dates::short_date(&t.date))
                                                .color(muted)
                                                .size(12.5),
                                        )
                                        .halign(egui::Align::LEFT),
                                    );
                                    let mut job = egui::text::LayoutJob::default();
                                    job.append(
                                        &t.label,
                                        0.0,
                                        egui::TextFormat {
                                            font_id: FontId::proportional(13.5),
                                            color: text_col,
                                            ..Default::default()
                                        },
                                    );
                                    if let Some(d) = &t.detail {
                                        job.append(
                                            &format!("\n{d}"),
                                            0.0,
                                            egui::TextFormat {
                                                font_id: FontId::proportional(11.0),
                                                color: muted,
                                                ..Default::default()
                                            },
                                        );
                                    }
                                    job.wrap = egui::text::TextWrapping {
                                        max_width: desc_w,
                                        max_rows: if t.detail.is_some() { 2 } else { 1 },
                                        break_anywhere: false,
                                        overflow_character: Some('\u{2026}'),
                                    };
                                    ui.add_sized([desc_w, row_h], egui::Label::new(job));
                                    ui.add_sized(
                                        [amt_w, row_h],
                                        egui::Label::new(
                                            amount_text(t.amount, &currency).size(13.5),
                                        )
                                        .halign(egui::Align::RIGHT),
                                    );
                                },
                            );
                        }
                    }
                    ui.add_space(16.0);
                });
            });
    }
}

fn header_label(text: &str, color: Color32, align: egui::Align) -> egui::Label {
    egui::Label::new(RichText::new(text).color(color).size(11.0)).halign(align)
}

fn amount_text(amount: f64, currency: &str) -> RichText {
    let income = amount >= 0.0;
    let sign = if income { "+" } else { "-" };
    let color = if income { INCOME } else { OUTGOING };
    RichText::new(format!("{sign}{currency} {:.0}", amount.abs())).color(color)
}
