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

pub struct TransactionsState {
    groups: Vec<MonthGroup>,
    dirty: bool,
}

impl Default for TransactionsState {
    fn default() -> Self {
        Self {
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
        ui.weak("A read-only record of every payment, sale, and expense. Edit these where they live in Members, Merchandise, and Expenses.");
        ui.add_space(8.0);

        let currency = repo.currency();

        if self.groups.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No transactions yet.").weak());
            });
            return;
        }

        let muted = crate::ui::theme::text_muted(ui.visuals());
        let text_col = ui.visuals().text_color();
        let hover_bg = ui.visuals().widgets.hovered.weak_bg_fill;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(860.0);
                    let spacing = ui.spacing().item_spacing.x;
                    let avail = (ui.available_width() - 4.0).max(440.0);
                    let pad = 10.0;
                    let date_w = 92.0;
                    let amt_w = 120.0;
                    let desc_w =
                        (avail - date_w - amt_w - pad * 2.0 - spacing * 2.0).max(150.0);

                    // Column headers
                    ui.horizontal(|ui| {
                        ui.add_space(pad);
                        ui.add_sized(
                            [date_w, 14.0],
                            egui::Label::new(RichText::new("Date").color(muted).size(11.0))
                                .halign(egui::Align::LEFT),
                        );
                        ui.add_sized(
                            [desc_w, 14.0],
                            egui::Label::new(
                                RichText::new("Description").color(muted).size(11.0),
                            )
                            .halign(egui::Align::LEFT),
                        );
                        ui.allocate_ui_with_layout(
                            egui::vec2(amt_w, 14.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(RichText::new("Amount").color(muted).size(11.0));
                            },
                        );
                    });
                    ui.add_space(6.0);
                    ui.separator();
                    ui.spacing_mut().item_spacing.y = 0.0;

                    for (gi, g) in self.groups.iter().enumerate() {
                        ui.add_space(if gi == 0 { 10.0 } else { 22.0 });
                        ui.horizontal(|ui| {
                            ui.add_space(pad);
                            ui.label(
                                RichText::new(g.label.to_uppercase())
                                    .color(muted)
                                    .size(11.0),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(amount_text(g.net, &currency).size(12.5));
                                },
                            );
                        });
                        ui.add_space(6.0);

                        for t in g.items.iter() {
                            let row_h = if t.detail.is_some() { 46.0 } else { 34.0 };
                            let full_w = ui.available_width();
                            let row_rect = egui::Rect::from_min_size(
                                ui.next_widget_position(),
                                egui::vec2(full_w, row_h),
                            );
                            if ui.rect_contains_pointer(row_rect) {
                                ui.painter().rect_filled(row_rect, 6.0, hover_bg);
                            }
                            ui.allocate_ui_with_layout(
                                egui::vec2(full_w, row_h),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.add_space(pad);
                                    ui.add_sized(
                                        [date_w, row_h],
                                        egui::Label::new(
                                            RichText::new(&t.date).color(muted).size(12.5),
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
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(amt_w, row_h),
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                amount_text(t.amount, &currency).size(13.5),
                                            );
                                        },
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

fn amount_text(amount: f64, currency: &str) -> RichText {
    let income = amount >= 0.0;
    let sign = if income { "+" } else { "-" };
    let color = if income { INCOME } else { OUTGOING };
    RichText::new(format!("{sign}{currency} {:.0}", amount.abs())).color(color)
}
