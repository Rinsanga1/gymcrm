use eframe::egui;

use crate::core::dates::{self, Period};
use crate::core::Repository;

pub struct DashboardState {
    period: Period,
    custom_start: String,
    custom_end: String,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self {
            period: Period::ThisMonth,
            custom_start: dates::months_ago(1),
            custom_end: dates::today(),
        }
    }
}

impl DashboardState {
    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) {
        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        ui.heading("Dashboard");
        ui.add_space(4.0);
        self.period_chips(ui);
        ui.separator();

        let (start, end) = self.period.range();
        let currency = repo.currency();

        let membership = repo.category_income("membership", &start, &end).unwrap_or(0.0);
        let registration = repo.category_income("registration", &start, &end).unwrap_or(0.0);
        let merch = repo.merch_income(&start, &end).unwrap_or(0.0);
        let merch_units = repo.merch_units(&start, &end).unwrap_or(0);
        let expenses = repo.total_expenses(&start, &end).unwrap_or(0.0);
        let total_income = membership + registration + merch;
        let net = total_income - expenses;

        let total_members = repo.count_members(false).unwrap_or(0);
        let active_members = repo.count_members(true).unwrap_or(0);
        // Due = owes any month since joining, most-behind first.
        let dues = repo
            .due_members_with_arrears(&dates::current_month())
            .unwrap_or_default();
        let due_count = dues.len();

        // The two numbers a gym owner opens the app to see.
        ui.add_space(8.0);
        let net_color = if net >= 0.0 {
            egui::Color32::from_rgb(40, 170, 90)
        } else {
            egui::Color32::from_rgb(200, 70, 70)
        };
        let due_color = if due_count > 0 {
            egui::Color32::from_rgb(210, 120, 40)
        } else {
            egui::Color32::from_rgb(40, 170, 90)
        };
        ui.horizontal(|ui| {
            hero(ui, "Net earnings", &format!("{} {:.2}", currency, net), net_color);
            ui.add_space(10.0);
            hero(ui, "Members due", &format!("{}", due_count), due_color);
        });

        ui.add_space(12.0);
        egui::Grid::new("kpis").num_columns(4).spacing([12.0, 12.0]).show(ui, |ui| {
            kpi(ui, "Total income", &format!("{} {:.2}", currency, total_income));
            kpi(ui, "Membership", &format!("{} {:.2}", currency, membership));
            kpi(ui, "Registration", &format!("{} {:.2}", currency, registration));
            kpi(ui, "Merchandise", &format!("{} {:.2} ({} units)", currency, merch, merch_units));
            ui.end_row();
            kpi(ui, "Expenses", &format!("{} {:.2}", currency, expenses));
            kpi(ui, "Members (total)", &format!("{}", total_members));
            kpi(ui, "Active members", &format!("{}", active_members));
            ui.end_row();
        });

        ui.add_space(12.0);
        let low = repo.low_stock_products(5).unwrap_or_default();
        if !low.is_empty() {
            ui.colored_label(
                egui::Color32::from_rgb(220, 140, 40),
                format!(
                    "Low stock: {}",
                    low.iter()
                        .map(|p| format!("{} ({})", p.name, p.stock))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }

        ui.add_space(8.0);
        ui.heading("Revenue trend");
        self.revenue_chart(ui, repo, &start, &end);

        ui.add_space(8.0);
        ui.heading("Recent activity");
        let recent = repo.list_transactions().unwrap_or_default();
        if recent.is_empty() {
            ui.label("No transactions yet.");
        } else {
            egui::Grid::new("recent_txns")
                .num_columns(3)
                .spacing([16.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    for t in recent.iter().take(10) {
                        ui.label(egui::RichText::new(&t.date).weak());
                        ui.label(&t.label);
                        let income = t.amount >= 0.0;
                        let color = if income {
                            egui::Color32::from_rgb(45, 170, 95)
                        } else {
                            egui::Color32::from_rgb(210, 90, 90)
                        };
                        let sign = if income { "+" } else { "-" };
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.colored_label(
                                    color,
                                    format!("{sign}{currency} {:.0}", t.amount.abs()),
                                );
                            },
                        );
                        ui.end_row();
                    }
                });
        }

        });
    }

    fn period_chips(&mut self, ui: &mut egui::Ui) {
        let opts: [Period; 6] = [
            Period::AllTime,
            Period::Today,
            Period::ThisWeek,
            Period::ThisMonth,
            Period::ThisQuarter,
            Period::ThisYear,
        ];
        ui.horizontal_wrapped(|ui| {
            for p in opts {
                let selected = self.period == p;
                if ui.selectable_label(selected, p.label()).clicked() {
                    self.period = p;
                }
            }
            let custom_selected = matches!(self.period, Period::Custom { .. });
            if ui.selectable_label(custom_selected, "Custom").clicked() {
                self.period = Period::Custom {
                    start: self.custom_start.clone(),
                    end: self.custom_end.clone(),
                };
            }
        });
        if matches!(self.period, Period::Custom { .. }) {
            ui.horizontal(|ui| {
                ui.label("From");
                if ui.text_edit_singleline(&mut self.custom_start).changed() {
                    self.period = Period::Custom {
                        start: self.custom_start.clone(),
                        end: self.custom_end.clone(),
                    };
                }
                ui.label("to");
                if ui.text_edit_singleline(&mut self.custom_end).changed() {
                    self.period = Period::Custom {
                        start: self.custom_start.clone(),
                        end: self.custom_end.clone(),
                    };
                }
                ui.weak("YYYY-MM-DD");
            });
        }
    }

    fn revenue_chart(&self, ui: &mut egui::Ui, repo: &Repository, start: &str, end: &str) {
        let by_day = repo.daily_revenue(start, end).unwrap_or_default();
        // Clamp to the span that actually has data so "All Time" (a 0000-9999
        // sentinel range) doesn't try to plot millions of days per frame.
        let (Some(lo), Some(hi)) = (by_day.keys().min().cloned(), by_day.keys().max().cloned())
        else {
            ui.weak("No data in this range.");
            return;
        };
        let cstart = if start > lo.as_str() { start } else { lo.as_str() };
        let cend = if end < hi.as_str() { end } else { hi.as_str() };
        let days = dates::days_inclusive(cstart, cend);
        if days.is_empty() {
            ui.weak("No data in this range.");
            return;
        }
        let values: Vec<f64> = days
            .iter()
            .map(|d| *by_day.get(&d.format("%Y-%m-%d").to_string()).unwrap_or(&0.0))
            .collect();
        let max = values.iter().cloned().fold(0.0f64, f64::max).max(1.0);
        let total: f64 = values.iter().sum();

        let desired = egui::vec2(ui.available_width(), 180.0);
        let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let bg = ui.visuals().extreme_bg_color;
        painter.rect_filled(rect, 4.0, bg);
        let line_color = egui::Color32::from_rgb(80, 170, 240);
        let axis_color = ui.visuals().weak_text_color();

        let pad = 8.0;
        let plot = egui::Rect::from_min_max(
            rect.min + egui::vec2(pad, pad),
            rect.max - egui::vec2(pad, pad),
        );
        painter.line_segment(
            [plot.left_bottom(), plot.right_bottom()],
            egui::Stroke::new(1.0, axis_color),
        );

        if values.len() == 1 {
            let p = egui::pos2(plot.center().x, plot.bottom() - plot.height() * (values[0] / max) as f32);
            painter.circle_filled(p, 3.0, line_color);
        } else {
            let step = plot.width() / (values.len().saturating_sub(1).max(1)) as f32;
            let points: Vec<egui::Pos2> = values
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let x = plot.left() + step * i as f32;
                    let y = plot.bottom() - plot.height() * (*v / max) as f32;
                    egui::pos2(x, y)
                })
                .collect();
            for w in points.windows(2) {
                painter.line_segment([w[0], w[1]], egui::Stroke::new(2.0, line_color));
            }
        }

        let currency = repo.currency();
        ui.horizontal(|ui| {
            ui.weak(format!("Total: {} {:.2}", currency, total));
            ui.add_space(12.0);
            ui.weak(format!("Peak day: {} {:.2}", currency, max));
            ui.add_space(12.0);
            ui.weak(format!("{} day(s)", days.len()));
        });
    }
}

/// A large emphasis card for the one or two numbers that matter most.
fn hero(ui: &mut egui::Ui, label: &str, value: &str, accent: egui::Color32) {
    use crate::ui::theme;
    let card = theme::card_fill(ui.visuals());
    let border = theme::border(ui.visuals());
    let muted = theme::text_muted(ui.visuals());
    egui::Frame::new()
        .fill(card)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::symmetric(20, 18))
        .show(ui, |ui| {
            ui.set_min_width(240.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label.to_uppercase()).size(12.0).color(muted));
                ui.add_space(6.0);
                ui.label(egui::RichText::new(value).size(34.0).strong().color(accent));
            });
        });
}

fn kpi(ui: &mut egui::Ui, label: &str, value: &str) {
    use crate::ui::theme;
    let card = theme::card_fill(ui.visuals());
    let border = theme::border(ui.visuals());
    let muted = theme::text_muted(ui.visuals());
    egui::Frame::new()
        .fill(card)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_min_width(160.0);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(label.to_uppercase())
                        .size(11.0)
                        .color(muted),
                );
                ui.add_space(4.0);
                ui.label(egui::RichText::new(value).size(22.0).strong());
            });
        });
}
