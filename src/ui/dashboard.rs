use eframe::egui;

use crate::core::dates::{self, MonthFilter};
use crate::core::Repository;
use crate::ui::theme;

/// A click on the dashboard that the app turns into navigation. The dashboard
/// stays ignorant of `View` and other tabs' internals — it just names intent.
pub enum DashNav {
    Members,
    MembersDue,
    LowStock,
    Transactions,
}

pub struct DashboardState {
    filter: MonthFilter,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self {
            filter: MonthFilter::current(),
        }
    }
}

impl DashboardState {
    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) -> Option<DashNav> {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| self.body(ui, repo))
            .inner
    }

    fn body(&mut self, ui: &mut egui::Ui, repo: &mut Repository) -> Option<DashNav> {
        let mut nav = None;

        ui.label(egui::RichText::new("Dashboard").size(24.0).strong());
        ui.add_space(6.0);

        // Brand-new install: a wall of zeros helps no one. Point them at the
        // first real step instead.
        let total_members = repo.count_members(false).unwrap_or(0);
        if total_members == 0 {
            return self.onboarding(ui);
        }

        let years =
            crate::ui::year_options(repo.transaction_years().unwrap_or_default(), self.filter.year);
        crate::ui::year_month_filter(ui, "dash_filter", &mut self.filter, &years);
        ui.separator();

        let (start, end) = self.filter.range();
        let currency = repo.currency();

        let membership = repo.category_income("membership", &start, &end).unwrap_or(0.0);
        let registration = repo.category_income("registration", &start, &end).unwrap_or(0.0);
        let merch = repo.merch_income(&start, &end).unwrap_or(0.0);
        let merch_units = repo.merch_units(&start, &end).unwrap_or(0);
        let expenses = repo.total_expenses(&start, &end).unwrap_or(0.0);
        let total_income = membership + registration + merch;
        let net = total_income - expenses;

        let active_members = repo.count_members(true).unwrap_or(0);
        // Due = owes any month since joining, most-behind first.
        let due_count = repo
            .due_members_with_arrears(&dates::current_month())
            .map(|d| d.len())
            .unwrap_or(0);

        // The two numbers a gym owner opens the app to see.
        ui.add_space(8.0);
        let net_color = if net >= 0.0 { theme::POSITIVE } else { theme::NEGATIVE };
        let due_color = if due_count > 0 { theme::WARNING } else { theme::POSITIVE };
        let net_sub = format!(
            "{} in \u{00b7} {} out",
            money(&currency, total_income),
            money(&currency, expenses)
        );
        let due_sub = if due_count > 0 {
            format!("of {} active", active_members)
        } else {
            "All paid up".to_string()
        };
        ui.horizontal_wrapped(|ui| {
            hero(ui, "Net earnings", &money(&currency, net), &net_sub, net_color, false);
            ui.add_space(10.0);
            if hero(ui, "Members due", &due_count.to_string(), &due_sub, due_color, true).clicked() {
                nav = Some(DashNav::MembersDue);
            }
        });

        ui.add_space(12.0);
        // The headline is the two heroes above; the per-category figures are a
        // breakdown you open when you want it, not seven numbers shouting at once.
        let kpis = [
            ("Monthly fees", money(&currency, membership)),
            ("Joining fees", money(&currency, registration)),
            ("Merchandise", format!("{} ({} units)", money(&currency, merch), merch_units)),
            ("Expenses", money(&currency, expenses)),
            ("Members (total)", total_members.to_string()),
            ("Active members", active_members.to_string()),
        ];
        egui::CollapsingHeader::new("Breakdown")
            .default_open(false)
            .show(ui, |ui| {
                // Fewer columns on narrow windows so the cards never clip off the edge.
                let cols = if ui.available_width() < 800.0 { 2 } else { 3 };
                egui::Grid::new("kpis").num_columns(cols).spacing([12.0, 12.0]).show(ui, |ui| {
                    for (i, (label, value)) in kpis.iter().enumerate() {
                        kpi(ui, label, value);
                        if (i + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
            });

        ui.add_space(12.0);
        let low = repo.low_stock_products(5).unwrap_or_default();
        if !low.is_empty() {
            let text = format!(
                "Low stock: {}",
                low.iter()
                    .map(|p| format!("{} ({})", p.name, p.stock))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let r = ui.add(
                egui::Label::new(egui::RichText::new(text).color(theme::WARNING))
                    .sense(egui::Sense::click()),
            );
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if r.clicked() {
                nav = Some(DashNav::LowStock);
            }
        }

        ui.add_space(12.0);
        section(ui, "Monthly revenue");
        self.revenue_chart(ui, repo);

        ui.add_space(12.0);
        section(ui, "Recent activity");
        let recent = repo.list_transactions(None, None).unwrap_or_default();
        if recent.is_empty() {
            ui.weak("No transactions yet.");
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
                        let color = if income { theme::POSITIVE } else { theme::NEGATIVE };
                        let sign = if income { "+" } else { "-" };
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.colored_label(
                                    color,
                                    format!("{sign}{}", money(&currency, t.amount.abs())),
                                );
                            },
                        );
                        ui.end_row();
                    }
                });
            ui.add_space(6.0);
            if ui.link("View all transactions \u{2192}").clicked() {
                nav = Some(DashNav::Transactions);
            }
        }

        nav
    }

    fn onboarding(&self, ui: &mut egui::Ui) -> Option<DashNav> {
        let mut nav = None;
        ui.add_space(24.0);
        let card = theme::card_fill(ui.visuals());
        let border = theme::border(ui.visuals());
        let muted = theme::text_muted(ui.visuals());
        egui::Frame::new()
            .fill(card)
            .stroke(egui::Stroke::new(1.0, border))
            .corner_radius(12.0)
            .inner_margin(egui::Margin::symmetric(24, 22))
            .show(ui, |ui| {
                ui.set_min_width(360.0);
                ui.label(egui::RichText::new("Welcome to RocheCRM").size(18.0).strong());
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "Add your members to start tracking dues, income, and stock.",
                    )
                    .color(muted),
                );
                ui.add_space(14.0);
                if ui.button("Add your first member").clicked() {
                    nav = Some(DashNav::Members);
                }
            });
        nav
    }

    fn revenue_chart(&self, ui: &mut egui::Ui, repo: &Repository) {
        let data = repo.monthly_revenue(12).unwrap_or_default();
        let values: Vec<f64> = data.iter().map(|(_, v)| *v).collect();
        if values.iter().sum::<f64>() <= 0.0 {
            ui.weak("No revenue yet.");
            return;
        }
        let currency = repo.currency();
        let max = values.iter().cloned().fold(0.0f64, f64::max).max(1.0);
        let n = values.len();

        let desired = egui::vec2(ui.available_width(), 180.0);
        let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
        let bar_color = egui::Color32::from_rgb(80, 170, 240);
        let axis_color = ui.visuals().weak_text_color();

        let pad = 8.0;
        let label_h = 16.0; // room for month labels under the axis
        let plot = egui::Rect::from_min_max(
            rect.min + egui::vec2(pad, pad),
            rect.max - egui::vec2(pad, pad + label_h),
        );
        painter.line_segment(
            [plot.left_bottom(), plot.right_bottom()],
            egui::Stroke::new(1.0, axis_color),
        );

        let slot = plot.width() / n as f32;
        let bar_w = (slot * 0.6).min(28.0);
        let hovered = resp.hover_pos().map(|p| {
            (((p.x - plot.left()) / slot).floor() as i64).clamp(0, n as i64 - 1) as usize
        });

        for i in 0..n {
            let cx = plot.left() + slot * (i as f32 + 0.5);
            let h = plot.height() * (values[i] / max) as f32;
            let br = egui::Rect::from_min_max(
                egui::pos2(cx - bar_w / 2.0, plot.bottom() - h),
                egui::pos2(cx + bar_w / 2.0, plot.bottom()),
            );
            let c = if hovered == Some(i) { bar_color } else { bar_color.gamma_multiply(0.8) };
            painter.rect_filled(br, 2.0, c);
            let m = data[i].0[5..7].parse::<usize>().unwrap_or(1).clamp(1, 12);
            painter.text(
                egui::pos2(cx, plot.bottom() + 3.0),
                egui::Align2::CENTER_TOP,
                MON[m - 1],
                egui::FontId::proportional(10.0),
                axis_color,
            );
        }

        if let Some(i) = hovered {
            resp.on_hover_ui(|ui| {
                ui.strong(dates::pretty_month(&data[i].0));
                ui.label(money(&currency, values[i]));
            });
        }

        let this = *values.last().unwrap();
        let prev = if n >= 2 { values[n - 2] } else { 0.0 };
        ui.horizontal(|ui| {
            ui.weak(format!("This month: {}", money(&currency, this)));
            if prev > 0.0 {
                ui.add_space(12.0);
                let up = this >= prev;
                let pct = (this - prev) / prev * 100.0;
                let color = if up { theme::POSITIVE } else { theme::NEGATIVE };
                let sign = if up { "+" } else { "" };
                ui.colored_label(color, format!("{sign}{:.0}% vs last month", pct));
            }
        });
    }
}

const MON: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Money with a currency prefix and two decimals — the one place we decide how
/// money reads on this screen, so every figure matches.
fn money(currency: &str, amount: f64) -> String {
    format!("{} {:.2}", currency, amount)
}

/// A muted section label, quieter than the page title so hierarchy reads.
fn section(ui: &mut egui::Ui, text: &str) {
    let muted = theme::text_muted(ui.visuals());
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .size(12.0)
            .strong()
            .color(muted),
    );
    ui.add_space(4.0);
}

/// A large emphasis card for the one or two numbers that matter most. Returns
/// its click response so a caller can make the number drill through.
fn hero(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    sub: &str,
    accent: egui::Color32,
    clickable: bool,
) -> egui::Response {
    let card = theme::card_fill(ui.visuals());
    let border = theme::border(ui.visuals());
    let muted = theme::text_muted(ui.visuals());
    let inner = egui::Frame::new()
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
                if !sub.is_empty() {
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(sub).size(12.0).color(muted));
                }
            });
        });
    if !clickable {
        return inner.response;
    }
    let resp = inner.response.interact(egui::Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

fn kpi(ui: &mut egui::Ui, label: &str, value: &str) {
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
