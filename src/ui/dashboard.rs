use eframe::egui;

use crate::core::dates::{self, Period};
use crate::core::models::Payment;
use crate::core::Repository;

struct PaymentDialog {
    member_id: i64,
    member_name: String,
    amount: String,
    month: String,
}

pub struct DashboardState {
    period: Period,
    custom_start: String,
    custom_end: String,
    pay_dialog: Option<PaymentDialog>,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self {
            period: Period::ThisMonth,
            custom_start: dates::months_ago(1),
            custom_end: dates::today(),
            pay_dialog: None,
        }
    }
}

impl DashboardState {
    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) {
        ui.heading("Dashboard");
        ui.add_space(4.0);
        self.period_chips(ui);
        ui.separator();

        let (start, end) = self.period.range();
        let currency = repo.currency();

        let membership = repo.membership_income(&start, &end).unwrap_or(0.0);
        let merch = repo.merch_income(&start, &end).unwrap_or(0.0);
        let merch_units = repo.merch_units(&start, &end).unwrap_or(0);
        let expenses = repo.total_expenses(&start, &end).unwrap_or(0.0);
        let total_income = membership + merch;
        let net = total_income - expenses;

        let total_members = repo.count_members(false).unwrap_or(0);
        let active_members = repo.count_members(true).unwrap_or(0);
        let dues = repo.due_members(&dates::current_month()).unwrap_or_default();
        let due_count = dues.len();

        ui.add_space(8.0);
        egui::Grid::new("kpis").num_columns(4).spacing([12.0, 12.0]).show(ui, |ui| {
            kpi(ui, "Total income", &format!("{} {:.2}", currency, total_income));
            kpi(ui, "Membership", &format!("{} {:.2}", currency, membership));
            kpi(ui, "Merchandise", &format!("{} {:.2} ({} units)", currency, merch, merch_units));
            kpi(ui, "Expenses", &format!("{} {:.2}", currency, expenses));
            ui.end_row();
            kpi(ui, "Net earnings", &format!("{} {:.2}", currency, net));
            kpi(ui, "Members (total)", &format!("{}", total_members));
            kpi(ui, "Active members", &format!("{}", active_members));
            kpi(ui, "Pending dues", &format!("{}", due_count));
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
        ui.heading(format!("Due this month ({})", due_count));
        if dues.is_empty() {
            ui.label("No members currently due. ✓");
        } else {
            let fee = repo.default_monthly_fee();
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for m in &dues {
                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::new(format!("Record payment → {}", m.name)).small())
                            .clicked()
                        {
                            self.pay_dialog = Some(PaymentDialog {
                                member_id: m.id,
                                member_name: m.name.clone(),
                                amount: format!("{}", fee),
                                month: dates::current_month(),
                            });
                        }
                        if let Some(p) = &m.phone {
                            ui.weak(p);
                        }
                    });
                }
            });
        }

        self.draw_payment_dialog(ui.ctx(), repo);
    }

    fn draw_payment_dialog(&mut self, ctx: &egui::Context, repo: &mut Repository) {
        let mut close = false;
        if let Some(d) = &mut self.pay_dialog {
            egui::Window::new(format!("Record payment — {}", d.member_name))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("pay_form").num_columns(2).show(ui, |ui| {
                        ui.label("Month (YYYY-MM)");
                        ui.text_edit_singleline(&mut d.month);
                        ui.end_row();
                        ui.label("Amount");
                        ui.text_edit_singleline(&mut d.amount);
                        ui.end_row();
                    });
                    let valid =
                        !d.month.trim().is_empty() && d.amount.parse::<f64>().is_ok();
                    ui.horizontal(|ui| {
                        if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                            let p = Payment {
                                id: 0,
                                member_id: d.member_id,
                                period_month: d.month.trim().to_string(),
                                amount: d.amount.parse().unwrap_or(0.0),
                                date: dates::today(),
                                note: None,
                            };
                            let _ = repo.insert_payment(&p);
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });
        }
        if close {
            self.pay_dialog = None;
        }
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
        let days = dates::days_inclusive(start, end);
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

fn kpi(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(170.0);
        ui.vertical(|ui| {
            ui.weak(label);
            ui.label(egui::RichText::new(value).heading());
        });
    });
}
