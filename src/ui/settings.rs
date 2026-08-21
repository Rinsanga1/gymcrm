use std::fs::File;
use std::path::PathBuf;

use eframe::egui;

use crate::core::{backup, dates, db, Repository};

pub struct SettingsState {
    gym_name: String,
    monthly_fee: String,
    registration_fee: String,
    currency: String,
    loaded: bool,
    status: Option<String>,
    import_summary: Option<String>,
    export_summary: Option<String>,
    backup_summary: Option<String>,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            gym_name: String::new(),
            monthly_fee: String::new(),
            registration_fee: String::new(),
            currency: String::new(),
            loaded: false,
            status: None,
            import_summary: None,
            export_summary: None,
            backup_summary: None,
        }
    }
}

impl SettingsState {
    fn load(&mut self, repo: &Repository) {
        self.gym_name = repo
            .get_setting("gym_name")
            .ok()
            .flatten()
            .unwrap_or_else(|| "TenneCRM".into());
        self.monthly_fee = format!("{}", repo.default_monthly_fee());
        self.registration_fee = format!("{}", repo.registration_fee());
        self.currency = repo.currency();
        self.loaded = true;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, repo: &mut Repository) {
        if !self.loaded {
            self.load(repo);
        }

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        ui.heading("Settings");
        ui.add_space(6.0);
        ui.heading("Preferences");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Theme");
            let current = crate::ui::theme::Mode::from_str(
                &repo.get_setting("theme").ok().flatten().unwrap_or_default(),
            );
            let mut selected = current;
            ui.selectable_value(&mut selected, crate::ui::theme::Mode::Light, "Light");
            ui.selectable_value(&mut selected, crate::ui::theme::Mode::Dark, "Dark");
            if selected != current {
                let _ = repo.set_setting("theme", selected.as_str());
                crate::ui::theme::apply(ui.ctx(), selected);
            }
        });
        ui.add_space(8.0);

        egui::Grid::new("settings_form").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
            ui.label("Gym name");
            ui.text_edit_singleline(&mut self.gym_name);
            ui.end_row();
            ui.label("Monthly membership fee");
            ui.text_edit_singleline(&mut self.monthly_fee);
            ui.end_row();
            ui.label("Joining fee (one-time)");
            ui.text_edit_singleline(&mut self.registration_fee);
            ui.end_row();
            ui.label("Currency");
            egui::ComboBox::from_id_salt("currency")
                .selected_text(if self.currency.trim().is_empty() {
                    "Select".to_string()
                } else {
                    self.currency.clone()
                })
                .show_ui(ui, |ui| {
                    for c in ["Rs", "₹", "$", "€", "£"] {
                        ui.selectable_value(&mut self.currency, c.to_string(), c);
                    }
                });
            ui.end_row();
        });

        ui.horizontal(|ui| {
            let valid = !self.gym_name.trim().is_empty()
                && self.monthly_fee.parse::<f64>().is_ok()
                && self.registration_fee.parse::<f64>().is_ok()
                && !self.currency.trim().is_empty();
            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                let result = repo
                    .set_setting("gym_name", self.gym_name.trim())
                    .and_then(|_| repo.set_setting("default_monthly_fee", self.monthly_fee.trim()))
                    .and_then(|_| repo.set_setting("registration_fee", self.registration_fee.trim()))
                    .and_then(|_| repo.set_setting("currency", self.currency.trim()));
                self.status = Some(match result {
                    Ok(()) => "Saved.".into(),
                    Err(e) => format!("Save failed: {e}"),
                });
            }
            if let Some(s) = &self.status {
                ui.weak(s);
            }
        });

        ui.add_space(20.0);
        ui.heading("Data");
        ui.separator();
        ui.label(egui::RichText::new("Import & export").strong());
        ui.weak("One spreadsheet holds everything — members, payments, sales, and expenses. Export it to back up or share; import it to bring data in. The first row is the column names, and a 'Type' column marks each row (Member, Payment, Sale, or Expense).");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Export all data…").clicked() {
                self.export_summary =
                    Some(pick_and_export("tennecrm-data.csv", |p| export_all(repo, p)));
            }
            if ui.button("Import data…").clicked() {
                if let Some(path) = pick_csv() {
                    self.import_summary = Some(import_all_csv(repo, &path));
                }
            }
        });
        if let Some(s) = &self.export_summary {
            ui.add_space(4.0);
            ui.label(s);
        }
        if let Some(s) = &self.import_summary {
            ui.add_space(4.0);
            ui.label(s);
        }

        ui.add_space(16.0);
        ui.label(egui::RichText::new("Backups & restore").strong());
        let db_path = db::db_path();
        ui.weak(format!(
            "Stored in: {}",
            backup::backups_dir(&db_path).display()
        ));
        ui.horizontal_wrapped(|ui| {
            if ui.button("Back up now").clicked() {
                let _ = repo.checkpoint();
                self.backup_summary = Some(match backup::backup_now(&db_path) {
                    Ok(p) => format!("Saved: {}", p.display()),
                    Err(e) => format!("Failed: {}", e),
                });
            }
            if ui.button("Restore from file…").clicked() {
                if let Some(src) = rfd::FileDialog::new()
                    .add_filter("SQLite DB", &["db"])
                    .set_directory(backup::backups_dir(&db_path))
                    .pick_file()
                {
                    self.backup_summary = Some(format!(
                        "Restore ready from {}. Close and reopen the app to finish.",
                        src.display()
                    ));
                    let _ = std::fs::write(
                        db_path.with_extension("db.pending-restore"),
                        src.to_string_lossy().as_bytes(),
                    );
                }
            }
        });
        let list = backup::list_backups(&db_path).unwrap_or_default();
        if list.is_empty() {
            ui.weak("No backups yet.");
        } else {
            ui.weak(format!("{} backups, newest first:", list.len()));
            for p in &list {
                ui.label(p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
            }
        }
        if let Some(s) = &self.backup_summary {
            ui.add_space(4.0);
            ui.label(s);
        }
        });
    }
}

fn pick_and_export<F>(suggested: &str, write: F) -> String
where
    F: FnOnce(&PathBuf) -> std::io::Result<usize>,
{
    let Some(path) = rfd::FileDialog::new()
        .add_filter("CSV", &["csv"])
        .set_file_name(suggested)
        .save_file()
    else {
        return "Canceled.".into();
    };
    match write(&path) {
        Ok(n) => format!("Exported {} row(s) to {}", n, path.display()),
        Err(e) => format!("Failed: {}", e),
    }
}

fn write_csv(path: &PathBuf, headers: &[&str], rows: impl IntoIterator<Item = Vec<String>>) -> std::io::Result<usize> {
    let file = File::create(path)?;
    let mut wtr = csv::Writer::from_writer(file);
    wtr.write_record(headers)?;
    let mut n = 0usize;
    for r in rows {
        wtr.write_record(&r)?;
        n += 1;
    }
    wtr.flush()?;
    Ok(n)
}

/// Every record in the app as one flat CSV, one row per record, tagged by a
/// leading `Type` column. Shared columns cover all four kinds; a blank cell just
/// means "not applicable to this type" (e.g. a payment has no phone). Sales are
/// exported per-sale at their total — per-product line detail lives in the Shop
/// tab and is not round-tripped here.
fn export_all(repo: &Repository, path: &PathBuf) -> std::io::Result<usize> {
    let members = repo.list_members(false).map_err(io_err)?;
    let mut rows: Vec<Vec<String>> = Vec::new();

    for m in &members {
        rows.push(vec![
            "Member".into(),
            m.join_date.clone(),
            m.name.clone(),
            m.phone.clone().unwrap_or_default(),
            if m.active { "active".into() } else { "inactive".into() },
            String::new(),
            m.notes.clone().unwrap_or_default(),
        ]);
    }
    for m in &members {
        for p in repo.payments_for_member(m.id).map_err(io_err)? {
            rows.push(vec![
                "Payment".into(),
                p.date,
                m.name.clone(),
                String::new(),
                p.category,
                format!("{}", p.amount),
                p.note.unwrap_or_default(),
            ]);
        }
    }
    for s in repo.list_sales().map_err(io_err)? {
        rows.push(vec![
            "Sale".into(),
            s.date,
            String::new(),
            String::new(),
            "shop".into(),
            format!("{}", s.total),
            String::new(),
        ]);
    }
    for e in repo.list_expenses().map_err(io_err)? {
        rows.push(vec![
            "Expense".into(),
            e.date,
            e.name,
            String::new(),
            String::new(),
            format!("{}", e.amount),
            e.note.unwrap_or_default(),
        ]);
    }
    write_csv(
        path,
        &["Type", "Date", "Name", "Phone", "Category", "Amount", "Note"],
        rows,
    )
}

fn io_err(e: rusqlite::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
}

fn pick_csv() -> Option<PathBuf> {
    rfd::FileDialog::new().add_filter("CSV", &["csv"]).pick_file()
}

/// Read the unified export format back in. Members are inserted first (in a
/// separate pass) so payments can attach to them by name no matter the row
/// order in the file. The whole load runs in one transaction: if the file is
/// broken partway, nothing is committed. Unknown types and unparseable rows are
/// skipped and counted rather than aborting the import.
fn import_all_csv(repo: &mut Repository, path: &PathBuf) -> String {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return format!("Failed to open file: {}", e),
    };
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(file);
    let headers = match rdr.headers() {
        Ok(h) => h.clone(),
        Err(e) => return format!("Failed to read header: {}", e),
    };
    let col = |name: &str| headers.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
    let (Some(type_idx), Some(name_idx)) = (col("type"), col("name")) else {
        return "CSV needs 'Type' and 'Name' columns. Export a file first to see the format.".into();
    };
    let date_idx = col("date");
    let phone_idx = col("phone");
    let category_idx = col("category");
    let amount_idx = col("amount");
    let note_idx = col("note");

    // Read every row up front so member rows can be processed before the payment
    // rows that reference them, whatever order they appear in.
    let records: Vec<csv::StringRecord> = rdr.records().flatten().collect();
    let cell = |r: &csv::StringRecord, idx: Option<usize>| -> String {
        idx.and_then(|i| r.get(i)).map(str::trim).unwrap_or("").to_string()
    };
    let opt = |s: String| if s.is_empty() { None } else { Some(s) };

    // Seed a name -> id map with members that already exist, so payments in the
    // file can attach to them even when the member isn't re-imported.
    let existing = match repo.list_members(false) {
        Ok(v) => v,
        Err(e) => return format!("Failed to read members: {}", e),
    };
    let mut ids: std::collections::HashMap<String, i64> =
        existing.into_iter().map(|m| (m.name.to_lowercase(), m.id)).collect();

    let today = dates::today();
    let now = dates::now();
    let tx = match repo.conn.transaction() {
        Ok(t) => t,
        Err(e) => return format!("Failed to start transaction: {}", e),
    };
    let (mut members, mut payments, mut sales, mut expenses, mut skipped) = (0u32, 0u32, 0u32, 0u32, 0u32);
    let date_or_today = |r: &csv::StringRecord| {
        let d = cell(r, date_idx);
        if d.is_empty() { today.clone() } else { d }
    };

    for r in &records {
        if !cell(r, Some(type_idx)).eq_ignore_ascii_case("member") {
            continue;
        }
        let name = cell(r, Some(name_idx));
        if name.is_empty() {
            skipped += 1;
            continue;
        }
        let active = !cell(r, category_idx).eq_ignore_ascii_case("inactive");
        let res = tx.execute(
            "INSERT INTO members(name, phone, join_date, active, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![name, opt(cell(r, phone_idx)), date_or_today(r), active as i64, opt(cell(r, note_idx))],
        );
        match res {
            Ok(_) => {
                ids.insert(name.to_lowercase(), tx.last_insert_rowid());
                members += 1;
            }
            Err(_) => skipped += 1,
        }
    }

    for r in &records {
        let ty = cell(r, Some(type_idx));
        if ty.eq_ignore_ascii_case("member") {
            continue;
        }
        let amount = cell(r, amount_idx).parse::<f64>();
        if ty.eq_ignore_ascii_case("payment") {
            let (Some(&mid), Ok(amount)) = (ids.get(&cell(r, Some(name_idx)).to_lowercase()), amount) else {
                skipped += 1;
                continue;
            };
            let date = date_or_today(r);
            let period = if date.len() >= 7 { date[..7].to_string() } else { dates::current_month() };
            let category = { let c = cell(r, category_idx); if c.is_empty() { "membership".into() } else { c } };
            let res = tx.execute(
                "INSERT INTO payments(member_id, period_month, amount, date, note, category, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![mid, period, amount, date, opt(cell(r, note_idx)), category, now],
            );
            match res { Ok(_) => payments += 1, Err(_) => skipped += 1 }
        } else if ty.eq_ignore_ascii_case("sale") {
            let Ok(amount) = amount else { skipped += 1; continue; };
            let res = tx.execute(
                "INSERT INTO sales(date, total, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![date_or_today(r), amount, now],
            );
            match res { Ok(_) => sales += 1, Err(_) => skipped += 1 }
        } else if ty.eq_ignore_ascii_case("expense") {
            let Ok(amount) = amount else { skipped += 1; continue; };
            let note = opt(cell(r, note_idx));
            let name = cell(r, Some(name_idx));
            let ename = if !name.is_empty() { name } else { note.clone().unwrap_or_else(|| "Expense".into()) };
            let res = tx.execute(
                "INSERT INTO expenses(name, amount, date, note, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![ename, amount, date_or_today(r), note, now],
            );
            match res { Ok(_) => expenses += 1, Err(_) => skipped += 1 }
        } else {
            skipped += 1;
        }
    }

    if let Err(e) = tx.commit() {
        return format!("Commit failed: {}", e);
    }
    format!(
        "Imported {members} member(s), {payments} payment(s), {sales} sale(s), {expenses} expense(s); skipped {skipped}."
    )
}

#[cfg(test)]
mod tests {
    use super::{export_all, import_all_csv};
    use crate::core::db::open_memory;
    use crate::core::models::{Expense, Member, Payment};
    use crate::core::Repository;

    fn member(name: &str) -> Member {
        Member {
            id: 0,
            name: name.into(),
            phone: Some("98765".into()),
            join_date: "2026-01-01".into(),
            active: true,
            notes: Some("VIP".into()),
        }
    }

    #[test]
    fn unified_csv_round_trips_every_type() {
        let mut src = Repository::new(open_memory().unwrap());
        let mid = src.insert_member(&member("Priya Sharma")).unwrap();
        src.insert_payment(&Payment {
            id: 0,
            member_id: mid,
            period_month: "2026-08".into(),
            amount: 1500.0,
            date: "2026-08-15".into(),
            note: None,
            category: "membership".into(),
        })
        .unwrap();
        src.import_sale("2026-08-16", 200.0, &[]).unwrap();
        src.insert_expense(&Expense {
            id: 0,
            name: "Rent".into(),
            amount: 12000.0,
            date: "2026-08-10".into(),
            note: Some("August".into()),
        })
        .unwrap();

        let path = std::env::temp_dir().join("tenne_unified_roundtrip.csv");
        export_all(&src, &path).unwrap();

        let mut dst = Repository::new(open_memory().unwrap());
        import_all_csv(&mut dst, &path);
        let _ = std::fs::remove_file(&path);

        let members = dst.list_members(false).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "Priya Sharma");
        assert_eq!(members[0].phone.as_deref(), Some("98765"));
        let pays = dst.payments_for_member(members[0].id).unwrap();
        assert_eq!(pays.len(), 1);
        assert_eq!(pays[0].amount, 1500.0);
        assert_eq!(pays[0].category, "membership");
        assert_eq!(dst.list_sales().unwrap().len(), 1);
        let exps = dst.list_expenses().unwrap();
        assert_eq!(exps.len(), 1);
        assert_eq!(exps[0].name, "Rent");
        assert_eq!(exps[0].amount, 12000.0);
    }
}
