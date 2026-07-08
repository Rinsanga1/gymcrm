use std::fs::File;
use std::path::PathBuf;

use eframe::egui;

use crate::core::models::{Expense, Member, Payment};
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
            .unwrap_or_else(|| "RocheCRM".into());
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
            ui.selectable_value(&mut selected, crate::ui::theme::Mode::Gray, "Gray");
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
            ui.label("Default monthly fee");
            ui.text_edit_singleline(&mut self.monthly_fee);
            ui.end_row();
            ui.label("Registration fee");
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
            if ui.add_enabled(valid, egui::Button::new("Save settings")).clicked() {
                let _ = repo.set_setting("gym_name", self.gym_name.trim());
                let _ = repo.set_setting("default_monthly_fee", self.monthly_fee.trim());
                let _ = repo.set_setting("registration_fee", self.registration_fee.trim());
                let _ = repo.set_setting("currency", self.currency.trim());
                self.status = Some("Saved.".into());
            }
            if let Some(s) = &self.status {
                ui.weak(s);
            }
        });

        ui.add_space(20.0);
        ui.heading("Data");
        ui.separator();
        ui.label(egui::RichText::new("Import CSV").strong());
        ui.weak("Round-trips with the exports below. Header row required; unknown columns ignored.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Import members…").clicked() {
                if let Some(path) = pick_csv() {
                    self.import_summary = Some(import_members_csv(repo, &path));
                }
            }
            if ui.button("Import payments…").clicked() {
                if let Some(path) = pick_csv() {
                    self.import_summary = Some(import_payments_csv(repo, &path));
                }
            }
            if ui.button("Import sales…").clicked() {
                if let Some(path) = pick_csv() {
                    self.import_summary = Some(import_sales_csv(repo, &path));
                }
            }
            if ui.button("Import expenses…").clicked() {
                if let Some(path) = pick_csv() {
                    self.import_summary = Some(import_expenses_csv(repo, &path));
                }
            }
        });
        if let Some(s) = &self.import_summary {
            ui.add_space(4.0);
            ui.label(s);
        }

        ui.add_space(16.0);
        ui.label(egui::RichText::new("Export CSV").strong());
        ui.horizontal_wrapped(|ui| {
            if ui.button("Export members").clicked() {
                self.export_summary = Some(pick_and_export("members.csv", |p| {
                    export_members(repo, p)
                }));
            }
            if ui.button("Export payments").clicked() {
                self.export_summary = Some(pick_and_export("payments.csv", |p| {
                    export_payments(repo, p)
                }));
            }
            if ui.button("Export sales").clicked() {
                self.export_summary = Some(pick_and_export("sales.csv", |p| {
                    export_sales(repo, p)
                }));
            }
            if ui.button("Export expenses").clicked() {
                self.export_summary = Some(pick_and_export("expenses.csv", |p| {
                    export_expenses(repo, p)
                }));
            }
        });
        if let Some(s) = &self.export_summary {
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
            if ui.button("Backup now").clicked() {
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
                        "Restore requested from {}. Restart the app to apply (the live DB is in use).",
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
            ui.weak(format!("{} backup file(s), newest first:", list.len()));
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

fn export_members(repo: &Repository, path: &PathBuf) -> std::io::Result<usize> {
    let rows = repo.list_members(false).map_err(io_err)?;
    write_csv(
        path,
        &["id", "name", "phone", "join_date", "active", "notes"],
        rows.into_iter().map(|m| {
            vec![
                m.id.to_string(),
                m.name,
                m.phone.unwrap_or_default(),
                m.join_date,
                if m.active { "1".into() } else { "0".into() },
                m.notes.unwrap_or_default(),
            ]
        }),
    )
}

fn export_payments(repo: &Repository, path: &PathBuf) -> std::io::Result<usize> {
    let members = repo.list_members(false).map_err(io_err)?;
    let mut all: Vec<Vec<String>> = Vec::new();
    for m in &members {
        for p in repo.payments_for_member(m.id).map_err(io_err)? {
            all.push(vec![
                p.id.to_string(),
                m.id.to_string(),
                m.name.clone(),
                p.period_month,
                format!("{}", p.amount),
                p.date,
                p.category,
                p.note.unwrap_or_default(),
            ]);
        }
    }
    write_csv(
        path,
        &["id", "member_id", "member_name", "period_month", "amount", "date", "category", "note"],
        all,
    )
}

fn export_sales(repo: &Repository, path: &PathBuf) -> std::io::Result<usize> {
    let sales = repo.list_sales().map_err(io_err)?;
    let mut all: Vec<Vec<String>> = Vec::new();
    for s in &sales {
        let items = repo.sale_items(s.id).map_err(io_err)?;
        if items.is_empty() {
            all.push(vec![
                s.id.to_string(),
                s.date.clone(),
                format!("{}", s.total),
                String::new(),
                String::new(),
                String::new(),
            ]);
        } else {
            for it in items {
                all.push(vec![
                    s.id.to_string(),
                    s.date.clone(),
                    format!("{}", s.total),
                    it.product_id.map(|i| i.to_string()).unwrap_or_default(),
                    it.qty.to_string(),
                    format!("{}", it.unit_price),
                ]);
            }
        }
    }
    write_csv(
        path,
        &["sale_id", "date", "sale_total", "product_id", "qty", "unit_price"],
        all,
    )
}

fn export_expenses(repo: &Repository, path: &PathBuf) -> std::io::Result<usize> {
    let rows = repo.list_expenses().map_err(io_err)?;
    write_csv(
        path,
        &["id", "name", "date", "amount", "note"],
        rows.into_iter().map(|e| {
            vec![
                e.id.to_string(),
                e.name,
                e.date,
                format!("{}", e.amount),
                e.note.unwrap_or_default(),
            ]
        }),
    )
}

fn io_err(e: rusqlite::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
}

fn pick_csv() -> Option<PathBuf> {
    rfd::FileDialog::new().add_filter("CSV", &["csv"]).pick_file()
}

/// Parse a CSV into (headers, rows), skipping malformed rows. Shared by the
/// importers so each only maps its own columns.
fn read_csv(path: &PathBuf) -> Result<(csv::StringRecord, Vec<csv::StringRecord>), String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {e}"))?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(file);
    let headers = rdr
        .headers()
        .map_err(|e| format!("Failed to read header: {e}"))?
        .clone();
    let rows = rdr.records().filter_map(Result::ok).collect();
    Ok((headers, rows))
}

fn import_expenses_csv(repo: &mut Repository, path: &PathBuf) -> String {
    let (headers, rows) = match read_csv(path) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let col = |name: &str| headers.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
    let Some(name_i) = col("name") else {
        return "CSV needs a 'name' column.".into();
    };
    let Some(amount_i) = col("amount") else {
        return "CSV needs an 'amount' column.".into();
    };
    let date_i = col("date");
    let note_i = col("note");
    let today = dates::today();
    let mut imported = 0u32;
    let mut skipped = 0u32;
    for rec in &rows {
        let name = rec.get(name_i).map(str::trim).unwrap_or("");
        let amount = rec.get(amount_i).and_then(|s| s.trim().parse::<f64>().ok());
        let (false, Some(amount)) = (name.is_empty(), amount) else {
            skipped += 1;
            continue;
        };
        let date = date_i
            .and_then(|i| rec.get(i))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| today.clone());
        let note = note_i
            .and_then(|i| rec.get(i))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let e = Expense {
            id: 0,
            name: name.to_string(),
            amount,
            date,
            note,
        };
        match repo.insert_expense(&e) {
            Ok(_) => imported += 1,
            Err(_) => skipped += 1,
        }
    }
    format!("Imported {} expense(s); skipped {}.", imported, skipped)
}

fn import_payments_csv(repo: &mut Repository, path: &PathBuf) -> String {
    let (headers, rows) = match read_csv(path) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let col = |name: &str| headers.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
    let member_id_i = col("member_id");
    let member_name_i = col("member_name");
    if member_id_i.is_none() && member_name_i.is_none() {
        return "CSV needs a 'member_id' or 'member_name' column.".into();
    }
    let Some(period_i) = col("period_month") else {
        return "CSV needs a 'period_month' column.".into();
    };
    let Some(amount_i) = col("amount") else {
        return "CSV needs an 'amount' column.".into();
    };
    let date_i = col("date");
    let category_i = col("category");
    let note_i = col("note");
    let members = repo.list_members(false).unwrap_or_default();
    let mut imported = 0u32;
    let mut skipped = 0u32;
    for rec in &rows {
        let csv_name = member_name_i
            .and_then(|i| rec.get(i))
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let by_id = member_id_i
            .and_then(|i| rec.get(i))
            .and_then(|s| s.trim().parse::<i64>().ok())
            .and_then(|id| members.iter().find(|m| m.id == id));
        let by_name =
            csv_name.and_then(|name| members.iter().find(|m| m.name.eq_ignore_ascii_case(name)));
        // An id from another database's id-space can collide with a different
        // member here, so trust an id match only when the name agrees (or there
        // is no name column); otherwise resolve by name.
        let member = match (by_id, csv_name) {
            (Some(m), Some(name)) if m.name.eq_ignore_ascii_case(name) => Some(m),
            (Some(m), None) => Some(m),
            _ => by_name.or(by_id),
        };
        let period = rec.get(period_i).map(str::trim).unwrap_or("");
        let amount = rec.get(amount_i).and_then(|s| s.trim().parse::<f64>().ok());
        let (Some(member), Some(amount)) = (member, amount) else {
            skipped += 1;
            continue;
        };
        if period.is_empty() {
            skipped += 1;
            continue;
        }
        let date = date_i
            .and_then(|i| rec.get(i))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{period}-15"));
        let category = category_i
            .and_then(|i| rec.get(i))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("membership")
            .to_string();
        let note = note_i
            .and_then(|i| rec.get(i))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let p = Payment {
            id: 0,
            member_id: member.id,
            period_month: period.to_string(),
            amount,
            date,
            note,
            category,
        };
        match repo.insert_payment(&p) {
            Ok(_) => imported += 1,
            Err(_) => skipped += 1,
        }
    }
    format!("Imported {} payment(s); skipped {}.", imported, skipped)
}

fn import_sales_csv(repo: &mut Repository, path: &PathBuf) -> String {
    let (headers, rows) = match read_csv(path) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let col = |name: &str| headers.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
    let Some(sale_id_i) = col("sale_id") else {
        return "CSV needs a 'sale_id' column.".into();
    };
    let Some(date_i) = col("date") else {
        return "CSV needs a 'date' column.".into();
    };
    let total_i = col("sale_total");
    let product_i = col("product_id");
    let qty_i = col("qty");
    let price_i = col("unit_price");

    // Sales span multiple rows (one per line item), so group by the source
    // sale_id, keeping first-seen order. product_id is preserved as-is: imports
    // don't touch current stock, since old product ids may not exist here.
    type Group = (String, Option<f64>, Vec<(Option<i64>, i64, f64)>);
    let mut order: Vec<String> = Vec::new();
    let mut grouped: std::collections::HashMap<String, Group> = std::collections::HashMap::new();
    for rec in &rows {
        let sid = rec.get(sale_id_i).map(str::trim).unwrap_or("").to_string();
        if sid.is_empty() {
            continue;
        }
        let date = rec.get(date_i).map(str::trim).unwrap_or("").to_string();
        let total = total_i
            .and_then(|i| rec.get(i))
            .and_then(|s| s.trim().parse::<f64>().ok());
        let entry = grouped.entry(sid.clone()).or_insert_with(|| {
            order.push(sid.clone());
            (date, total, Vec::new())
        });
        let qty = qty_i
            .and_then(|i| rec.get(i))
            .and_then(|s| s.trim().parse::<i64>().ok());
        let price = price_i
            .and_then(|i| rec.get(i))
            .and_then(|s| s.trim().parse::<f64>().ok());
        if let (Some(qty), Some(price)) = (qty, price) {
            let product_id = product_i
                .and_then(|i| rec.get(i))
                .and_then(|s| s.trim().parse::<i64>().ok());
            entry.2.push((product_id, qty, price));
        }
    }

    let mut imported = 0u32;
    let mut skipped = 0u32;
    for sid in &order {
        let (date, total, items) = &grouped[sid];
        if date.is_empty() {
            skipped += 1;
            continue;
        }
        let total = total.unwrap_or_else(|| {
            items.iter().map(|(_, q, p)| *q as f64 * *p).sum()
        });
        match repo.import_sale(date, total, items) {
            Ok(_) => imported += 1,
            Err(_) => skipped += 1,
        }
    }
    format!("Imported {} sale(s); skipped {}.", imported, skipped)
}

fn import_members_csv(repo: &mut Repository, path: &PathBuf) -> String {
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
    let name_idx = headers
        .iter()
        .position(|h| h.trim().eq_ignore_ascii_case("name"));
    let phone_idx = headers
        .iter()
        .position(|h| h.trim().eq_ignore_ascii_case("phone"));
    let join_idx = headers
        .iter()
        .position(|h| h.trim().eq_ignore_ascii_case("join_date"));
    let active_idx = headers
        .iter()
        .position(|h| h.trim().eq_ignore_ascii_case("active"));
    let notes_idx = headers
        .iter()
        .position(|h| h.trim().eq_ignore_ascii_case("notes"));
    let Some(name_idx) = name_idx else {
        return "CSV needs a 'Name' column.".into();
    };

    let today = dates::today();
    let tx = match repo.conn.transaction() {
        Ok(t) => t,
        Err(e) => return format!("Failed to start transaction: {}", e),
    };
    let mut imported = 0u32;
    let mut skipped = 0u32;
    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let name = record.get(name_idx).map(str::trim).unwrap_or("");
        if name.is_empty() {
            skipped += 1;
            continue;
        }
        let phone = phone_idx
            .and_then(|i| record.get(i))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let join_date = join_idx
            .and_then(|i| record.get(i))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| today.clone());
        let active = active_idx
            .and_then(|i| record.get(i))
            .map(str::trim)
            .map(|s| !matches!(s.to_ascii_lowercase().as_str(), "0" | "false" | "no" | "inactive"))
            .unwrap_or(true);
        let notes = notes_idx
            .and_then(|i| record.get(i))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let m = Member {
            id: 0,
            name: name.to_string(),
            phone,
            join_date,
            active,
            notes,
        };
        let r = tx.execute(
            "INSERT INTO members(name, phone, join_date, active, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![m.name, m.phone, m.join_date, m.active as i64, m.notes],
        );
        match r {
            Ok(_) => imported += 1,
            Err(_) => skipped += 1,
        }
    }
    if let Err(e) = tx.commit() {
        return format!("Commit failed: {}", e);
    }
    format!("Imported {} member(s); skipped {}.", imported, skipped)
}
