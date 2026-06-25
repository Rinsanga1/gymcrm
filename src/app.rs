use eframe::egui;

use crate::core::{backup, db, Repository};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Members,
    Merchandise,
    Expenses,
    Settings,
}

impl View {
    const ALL: [View; 5] = [
        View::Dashboard,
        View::Members,
        View::Merchandise,
        View::Expenses,
        View::Settings,
    ];

    fn label(self) -> &'static str {
        match self {
            View::Dashboard => "Dashboard",
            View::Members => "Members",
            View::Merchandise => "Merchandise",
            View::Expenses => "Expenses",
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
    dashboard: crate::ui::dashboard::DashboardState,
    settings: crate::ui::settings::SettingsState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

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

        Self {
            repo,
            view: View::Dashboard,
            gym_name,
            members: crate::ui::members::MembersState::default(),
            merchandise: crate::ui::merchandise::MerchandiseState::default(),
            expenses: crate::ui::expenses::ExpensesState::default(),
            dashboard: crate::ui::dashboard::DashboardState::default(),
            settings: crate::ui::settings::SettingsState::default(),
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.heading("RocheCRM");
        ui.label(egui::RichText::new(&self.gym_name).weak());
        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        for v in View::ALL {
            let selected = self.view == v;
            if ui
                .selectable_label(selected, egui::RichText::new(v.label()).size(16.0))
                .clicked()
            {
                self.view = v;
                // Re-query the DB on every navigation so changes made in
                // other views (e.g. CSV import in Settings) are visible.
                self.members.invalidate();
                self.merchandise.invalidate();
                self.expenses.invalidate();
            }
            ui.add_space(4.0);
        }
    }
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
            .exact_size(200.0)
            .show_inside(ui, |ui| {
                self.sidebar(ui);
            });

        let repo = &mut self.repo;
        let members = &mut self.members;
        let merchandise = &mut self.merchandise;
        let expenses = &mut self.expenses;
        let dashboard = &mut self.dashboard;
        let settings = &mut self.settings;
        let view = self.view;
        egui::CentralPanel::default().show_inside(ui, |ui| match view {
            View::Dashboard => dashboard.show(ui, repo),
            View::Members => members.show(ui, repo),
            View::Merchandise => merchandise.show(ui, repo),
            View::Expenses => expenses.show(ui, repo),
            View::Settings => settings.show(ui, repo),
        });
    }
}
