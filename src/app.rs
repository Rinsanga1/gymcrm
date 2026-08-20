use eframe::egui;

use crate::core::{backup, db, Repository};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Members,
    Merchandise,
    Expenses,
    Transactions,
    Settings,
}

impl View {
    const ALL: [View; 6] = [
        View::Dashboard,
        View::Members,
        View::Merchandise,
        View::Expenses,
        View::Transactions,
        View::Settings,
    ];

    fn label(self) -> &'static str {
        match self {
            View::Dashboard => "Dashboard",
            View::Members => "Members",
            View::Merchandise => "Shop",
            View::Expenses => "Expenses",
            View::Transactions => "Transactions",
            View::Settings => "Settings",
        }
    }
}

pub struct App {
    pub repo: Repository,
    pub view: View,
    pub gym_name: String,
    members: crate::ui::members::MembersState,
    merchandise: crate::ui::merchandise::MerchandiseState,
    expenses: crate::ui::expenses::ExpensesState,
    transactions: crate::ui::transactions::TransactionsState,
    dashboard: crate::ui::dashboard::DashboardState,
    settings: crate::ui::settings::SettingsState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let path = db::db_path();

        // Apply a pending restore (queued from a previous run's Settings dialog).
        let pending = path.with_extension("db.pending-restore");
        if let Ok(src_str) = std::fs::read_to_string(&pending) {
            let src = std::path::PathBuf::from(src_str.trim());
            if src.is_file() {
                let _ = backup::restore(&path, &src);
            }
            let _ = std::fs::remove_file(&pending);
        }

        let conn = db::open_db(&path).expect("failed to open database");
        let repo = Repository::new(conn);
        let gym_name = repo
            .get_setting("gym_name")
            .ok()
            .flatten()
            .unwrap_or_else(|| "RocheCRM".to_string());

        let mode = crate::ui::theme::Mode::from_str(
            &repo.get_setting("theme").ok().flatten().unwrap_or_default(),
        );
        crate::ui::theme::apply(&cc.egui_ctx, mode);

        Self {
            repo,
            view: View::Dashboard,
            gym_name,
            members: crate::ui::members::MembersState::default(),
            merchandise: crate::ui::merchandise::MerchandiseState::default(),
            expenses: crate::ui::expenses::ExpensesState::default(),
            transactions: crate::ui::transactions::TransactionsState::default(),
            dashboard: crate::ui::dashboard::DashboardState::default(),
            settings: crate::ui::settings::SettingsState::default(),
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        use crate::ui::theme;
        let muted = theme::text_muted(ui.visuals());
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("RocheCRM").size(16.0).strong());
        });
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(&self.gym_name).size(12.0).color(muted));
        });
        ui.add_space(16.0);

        for v in View::ALL {
            let selected = self.view == v;
            if nav_item(ui, v.label(), selected).clicked() {
                self.view = v;
                // Re-query the DB on every navigation so changes made in
                // other views (e.g. CSV import in Settings) are visible.
                self.members.invalidate();
                self.merchandise.invalidate();
                self.expenses.invalidate();
                self.transactions.invalidate();
                self.gym_name = self
                    .repo
                    .get_setting("gym_name")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "RocheCRM".into());
            }
            ui.add_space(2.0);
        }
    }
}

/// A full-width sidebar navigation row with a selection highlight.
fn nav_item(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    use crate::ui::theme;
    let desired = egui::vec2(ui.available_width(), 32.0);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());
    let text_color = ui.visuals().text_color();
    let muted = theme::text_muted(ui.visuals());
    let (bg, fg) = if selected {
        (theme::nav_selected_fill(ui.ctx()), text_color)
    } else if resp.hovered() {
        (theme::nav_hover_fill(ui.ctx()), text_color)
    } else {
        (egui::Color32::TRANSPARENT, muted)
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, egui::CornerRadius::same(7), bg);
    painter.text(
        rect.left_center() + egui::vec2(12.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.5),
        fg,
    );
    resp
}

impl eframe::App for App {
    fn on_exit(&mut self) {
        // Best-effort: checkpoint WAL, then snapshot to backups/. Old backups
        // beyond KEEP_LAST are pruned inside backup_now.
        let _ = self.repo.checkpoint();
        let path = db::db_path();
        let _ = backup::backup_now(&path);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("sidebar")
            .resizable(false)
            .exact_size(210.0)
            .frame(
                egui::Frame::new()
                    .fill(crate::ui::theme::sidebar_fill(ui.ctx()))
                    .inner_margin(egui::Margin {
                        left: 10,
                        right: 10,
                        top: 0,
                        bottom: 8,
                    }),
            )
            .show_inside(ui, |ui| {
                self.sidebar(ui);
            });

        let repo = &mut self.repo;
        let members = &mut self.members;
        let merchandise = &mut self.merchandise;
        let expenses = &mut self.expenses;
        let transactions = &mut self.transactions;
        let dashboard = &mut self.dashboard;
        let settings = &mut self.settings;
        let view = self.view;
        let nav = egui::CentralPanel::default()
            .show_inside(ui, |ui| match view {
                View::Dashboard => dashboard.show(ui, repo).map(Nav::Dash),
                View::Members => {
                    members.show(ui, repo);
                    None
                }
                View::Merchandise => {
                    merchandise.show(ui, repo);
                    None
                }
                View::Expenses => {
                    expenses.show(ui, repo);
                    None
                }
                View::Transactions => transactions.show(ui, repo).map(Nav::Txn),
                View::Settings => {
                    settings.show(ui, repo);
                    None
                }
            })
            .inner;

        match nav {
            Some(Nav::Dash(d)) => {
                use crate::ui::dashboard::DashNav;
                match d {
                    DashNav::Members => self.view = View::Members,
                    DashNav::MembersDue => {
                        self.view = View::Members;
                        self.members.focus_due();
                    }
                    DashNav::LowStock => self.view = View::Merchandise,
                    DashNav::Transactions => self.view = View::Transactions,
                }
            }
            Some(Nav::Txn(t)) => {
                use crate::ui::transactions::TxnNav;
                match t {
                    TxnNav::Member(id) => {
                        self.view = View::Members;
                        self.members.focus_member(id);
                    }
                    TxnNav::Sale(id) => {
                        self.view = View::Merchandise;
                        self.merchandise.focus_sale(id);
                    }
                    TxnNav::Expense(id) => {
                        self.view = View::Expenses;
                        self.expenses.focus_expense(id);
                    }
                }
            }
            None => {}
        }
    }
}

/// A navigation intent bubbled up from a tab: either the dashboard's drill-downs
/// or a Transactions row tapped to open its source record.
enum Nav {
    Dash(crate::ui::dashboard::DashNav),
    Txn(crate::ui::transactions::TxnNav),
}
