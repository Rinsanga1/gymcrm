use rusqlite::{params, Connection, Row};

use super::dates;
use super::models::{Expense, Member, Payment, Product, Sale, SaleItem, Txn, TxnKind};

/// Thin data-access layer over the SQLite connection. UI-free and testable.
pub struct Repository {
    pub conn: Connection,
}

fn member_from_row(r: &Row) -> rusqlite::Result<Member> {
    Ok(Member {
        id: r.get("id")?,
        name: r.get("name")?,
        phone: r.get("phone")?,
        join_date: r.get("join_date")?,
        active: r.get::<_, i64>("active")? != 0,
        notes: r.get("notes")?,
    })
}

fn payment_from_row(r: &Row) -> rusqlite::Result<Payment> {
    Ok(Payment {
        id: r.get("id")?,
        member_id: r.get("member_id")?,
        period_month: r.get("period_month")?,
        amount: r.get("amount")?,
        date: r.get("date")?,
        note: r.get("note")?,
        category: r.get("category")?,
    })
}

fn product_from_row(r: &Row) -> rusqlite::Result<Product> {
    Ok(Product {
        id: r.get("id")?,
        name: r.get("name")?,
        price: r.get("price")?,
        stock: r.get("stock")?,
        active: r.get::<_, i64>("active")? != 0,
    })
}

fn expense_from_row(r: &Row) -> rusqlite::Result<Expense> {
    Ok(Expense {
        id: r.get("id")?,
        name: r.get("name")?,
        amount: r.get("amount")?,
        date: r.get("date")?,
        note: r.get("note")?,
    })
}

impl Repository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    // ---- Settings -------------------------------------------------------

    pub fn get_setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get::<_, String>(0)
            })
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn default_monthly_fee(&self) -> f64 {
        self.get_setting("default_monthly_fee")
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1500.0)
    }

    pub fn registration_fee(&self) -> f64 {
        self.get_setting("registration_fee")
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or(500.0)
    }

    /// Count of members still owing the one-time registration fee — i.e. with no
    /// payment of category `registration` on record.
    pub fn unpaid_registration_count(&self, active_only: bool) -> rusqlite::Result<i64> {
        let sql = if active_only {
            "SELECT COUNT(*) FROM members m WHERE m.active=1
             AND NOT EXISTS (SELECT 1 FROM payments p
                             WHERE p.member_id=m.id AND p.category='registration')"
        } else {
            "SELECT COUNT(*) FROM members m
             WHERE NOT EXISTS (SELECT 1 FROM payments p
                               WHERE p.member_id=m.id AND p.category='registration')"
        };
        self.conn.query_row(sql, [], |r| r.get(0))
    }

    /// True if the member has recorded their one-time registration payment.
    pub fn has_registration_payment(&self, member_id: i64) -> rusqlite::Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM payments WHERE member_id=?1 AND category='registration'",
            [member_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Members who still owe the one-time registration fee, name-sorted.
    pub fn members_missing_registration(&self, active_only: bool) -> rusqlite::Result<Vec<Member>> {
        let sql = if active_only {
            "SELECT * FROM members m WHERE m.active=1
             AND NOT EXISTS (SELECT 1 FROM payments p
                             WHERE p.member_id=m.id AND p.category='registration')
             ORDER BY m.name COLLATE NOCASE"
        } else {
            "SELECT * FROM members m
             WHERE NOT EXISTS (SELECT 1 FROM payments p
                               WHERE p.member_id=m.id AND p.category='registration')
             ORDER BY m.name COLLATE NOCASE"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], member_from_row)?;
        rows.collect()
    }

    /// Force a full WAL checkpoint so the main DB file contains all committed
    /// data — important before copying the file for a backup.
    pub fn checkpoint(&self) -> rusqlite::Result<()> {
        self.conn
            .pragma_update(None, "wal_checkpoint", "FULL")
    }

    pub fn currency(&self) -> String {
        self.get_setting("currency")
            .ok()
            .flatten()
            .unwrap_or_else(|| "Rs".to_string())
    }

    // ---- Members --------------------------------------------------------

    pub fn insert_member(&self, m: &Member) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO members(name, phone, join_date, active, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                m.name,
                m.phone,
                m.join_date,
                m.active as i64,
                m.notes,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_member(&self, m: &Member) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE members SET name=?1, phone=?2, join_date=?3, active=?4, notes=?5
             WHERE id=?6",
            params![
                m.name,
                m.phone,
                m.join_date,
                m.active as i64,
                m.notes,
                m.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_member(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM members WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn set_member_active(&self, id: i64, active: bool) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE members SET active=?1 WHERE id=?2",
            params![active as i64, id],
        )?;
        Ok(())
    }

    pub fn get_member(&self, id: i64) -> rusqlite::Result<Option<Member>> {
        self.conn
            .query_row("SELECT * FROM members WHERE id=?1", [id], member_from_row)
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
    }

    /// List members. `active_only` restricts to the active roster.
    pub fn list_members(&self, active_only: bool) -> rusqlite::Result<Vec<Member>> {
        let sql = if active_only {
            "SELECT * FROM members WHERE active=1 ORDER BY name COLLATE NOCASE"
        } else {
            "SELECT * FROM members ORDER BY name COLLATE NOCASE"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], member_from_row)?;
        rows.collect()
    }

    /// Search by name or phone (DB-level, indexed prefix/substring match).
    pub fn search_members(&self, query: &str, active_only: bool) -> rusqlite::Result<Vec<Member>> {
        let like = format!("%{}%", query);
        let sql = if active_only {
            "SELECT * FROM members WHERE active=1 AND (name LIKE ?1 OR phone LIKE ?1)
             ORDER BY name COLLATE NOCASE"
        } else {
            "SELECT * FROM members WHERE (name LIKE ?1 OR phone LIKE ?1)
             ORDER BY name COLLATE NOCASE"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([like], member_from_row)?;
        rows.collect()
    }

    pub fn count_members(&self, active_only: bool) -> rusqlite::Result<i64> {
        let sql = if active_only {
            "SELECT COUNT(*) FROM members WHERE active=1"
        } else {
            "SELECT COUNT(*) FROM members"
        };
        self.conn.query_row(sql, [], |r| r.get(0))
    }

    // ---- Payments / dues -----------------------------------------------

    pub fn insert_payment(&self, p: &Payment) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO payments(member_id, period_month, amount, date, note, category, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                p.member_id,
                p.period_month,
                p.amount,
                p.date,
                p.note,
                p.category,
                dates::now()
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Record the one-time registration fee as a payment, but only if this
    /// member doesn't already have one (idempotent across re-saves).
    pub fn ensure_registration_payment(
        &self,
        member_id: i64,
        amount: f64,
        date: &str,
        month: &str,
    ) -> rusqlite::Result<()> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM payments WHERE member_id=?1 AND category='registration'",
            [member_id],
            |r| r.get(0),
        )?;
        if n == 0 {
            self.insert_payment(&Payment {
                id: 0,
                member_id,
                period_month: month.to_string(),
                amount,
                date: date.to_string(),
                note: None,
                category: "registration".to_string(),
            })?;
        }
        Ok(())
    }

    /// Reverse of `ensure_registration_payment`: drop the member's registration
    /// payment so un-collecting the joining fee also removes its transaction.
    pub fn remove_registration_payment(&self, member_id: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM payments WHERE member_id=?1 AND category='registration'",
            [member_id],
        )?;
        Ok(())
    }

    pub fn update_payment(&self, p: &Payment) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE payments SET period_month=?1, amount=?2, date=?3, note=?4 WHERE id=?5",
            params![p.period_month, p.amount, p.date, p.note, p.id],
        )?;
        Ok(())
    }

    pub fn delete_payment(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM payments WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn get_payment(&self, id: i64) -> rusqlite::Result<Option<Payment>> {
        self.conn
            .query_row("SELECT * FROM payments WHERE id=?1", [id], payment_from_row)
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
    }

    /// Every money movement (payments, sales, expenses) newest-first, for the
    /// Transactions history view. Expense amounts are negated (money out).
    pub fn list_transactions(&self) -> rusqlite::Result<Vec<Txn>> {
        // Each entry is (created_at, Txn); created_at breaks same-day ties so
        // rows across the three sources order by when they were recorded.
        let mut out: Vec<(String, Txn)> = Vec::new();

        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.date, p.amount, p.category, p.note, m.name,
                    COALESCE(p.created_at, p.date)
             FROM payments p LEFT JOIN members m ON m.id = p.member_id
             WHERE p.amount <> 0",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, String>(6)?,
            ))
        })?;
        for row in rows {
            let (id, date, amount, category, note, name, created_at) = row?;
            let cat = match category.as_str() {
                "registration" => "Registration",
                _ => "Membership",
            };
            let label = match &name {
                Some(n) => format!("{cat} · {n}"),
                None => cat.to_string(),
            };
            out.push((
                created_at,
                Txn {
                    kind: TxnKind::Payment,
                    id,
                    date,
                    amount,
                    label,
                    detail: note,
                },
            ));
        }

        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.date, s.total, COALESCE(s.created_at, s.date),
                    group_concat(si.qty || '× ' || COALESCE(p.name, '(removed)'), ' · ')
             FROM sales s
             LEFT JOIN sale_items si ON si.sale_id = s.id
             LEFT JOIN products p ON p.id = si.product_id
             GROUP BY s.id, s.date, s.total, s.created_at",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?;
        for row in rows {
            let (id, date, total, created_at, items) = row?;
            out.push((
                created_at,
                Txn {
                    kind: TxnKind::Sale,
                    id,
                    date,
                    amount: total,
                    label: "Merchandise sale".to_string(),
                    detail: items.filter(|s| !s.trim().is_empty()),
                },
            ));
        }

        let mut stmt = self.conn.prepare(
            "SELECT id, date, amount, note, name, COALESCE(created_at, date) FROM expenses",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f64>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })?;
        for row in rows {
            let (id, date, amount, note, name, created_at) = row?;
            let label = if name.trim().is_empty() {
                "Expense".to_string()
            } else {
                name
            };
            out.push((
                created_at,
                Txn {
                    kind: TxnKind::Expense,
                    id,
                    date,
                    amount: -amount,
                    label,
                    detail: note,
                },
            ));
        }

        out.sort_by(|a, b| {
            b.1.date
                .cmp(&a.1.date)
                .then(b.0.cmp(&a.0))
                .then(b.1.id.cmp(&a.1.id))
        });
        Ok(out.into_iter().map(|(_, t)| t).collect())
    }

    pub fn payments_for_member(&self, member_id: i64) -> rusqlite::Result<Vec<Payment>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM payments WHERE member_id=?1 ORDER BY date DESC")?;
        let rows = stmt.query_map([member_id], payment_from_row)?;
        rows.collect()
    }

    /// True if the member has at least one payment recorded for `month` (YYYY-MM).
    pub fn is_paid(&self, member_id: i64, month: &str) -> rusqlite::Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM payments
             WHERE member_id=?1 AND period_month=?2 AND category != 'registration'",
            params![member_id, month],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Set of member ids with at least one payment in `month` (one query;
    /// used to compute per-row status without N queries).
    pub fn paid_member_ids(&self, month: &str) -> rusqlite::Result<std::collections::HashSet<i64>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT member_id FROM payments
                 WHERE period_month=?1 AND category != 'registration'",
            )?;
        let rows = stmt.query_map([month], |r| r.get::<_, i64>(0))?;
        let mut set = std::collections::HashSet::new();
        for id in rows {
            set.insert(id?);
        }
        Ok(set)
    }

    /// How many membership-months the member is behind as of `current_month`,
    /// counting from their `join_month` (both `YYYY-MM`), and the money that
    /// represents at the current monthly fee. Registration payments don't count.
    pub fn membership_arrears(
        &self,
        member_id: i64,
        join_month: &str,
        current_month: &str,
    ) -> rusqlite::Result<(i64, f64)> {
        let paid: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT period_month) FROM payments
             WHERE member_id=?1 AND category != 'registration'
               AND period_month >= ?2 AND period_month <= ?3",
            params![member_id, join_month, current_month],
            |r| r.get(0),
        )?;
        let expected = super::dates::month_diff(join_month, current_month).max(0) + 1;
        let behind = (expected - paid).max(0);
        Ok((behind, behind as f64 * self.default_monthly_fee()))
    }

    /// Every active member's outstanding membership-months as of `current_month`,
    /// counted from each member's own join month. Only members who owe at least
    /// one month are included, keyed by member id: (months behind, money owed).
    /// One query for all payments, then per-member counting in Rust.
    pub fn arrears_all(
        &self,
        current_month: &str,
    ) -> rusqlite::Result<std::collections::HashMap<i64, (i64, f64)>> {
        let mut paid: std::collections::HashMap<i64, Vec<String>> =
            std::collections::HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT member_id, period_month FROM payments
             WHERE category != 'registration' AND period_month <= ?1",
        )?;
        for row in stmt.query_map([current_month], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })? {
            let (id, m) = row?;
            paid.entry(id).or_default().push(m);
        }
        let fee = self.default_monthly_fee();
        let mut out = std::collections::HashMap::new();
        for m in self.list_members(true)? {
            let join_month = &m.join_date[..7];
            let paid_count = paid
                .get(&m.id)
                .map(|ms| ms.iter().filter(|pm| pm.as_str() >= join_month).count() as i64)
                .unwrap_or(0);
            let expected = super::dates::month_diff(join_month, current_month).max(0) + 1;
            let behind = (expected - paid_count).max(0);
            if behind > 0 {
                out.insert(m.id, (behind, behind as f64 * fee));
            }
        }
        Ok(out)
    }

    /// Active members who owe at least one month as of `current_month`, each with
    /// (months behind, money owed), most-behind first. Drives the Dashboard due list.
    pub fn due_members_with_arrears(
        &self,
        current_month: &str,
    ) -> rusqlite::Result<Vec<(Member, i64, f64)>> {
        let arrears = self.arrears_all(current_month)?;
        let mut out: Vec<(Member, i64, f64)> = self
            .list_members(true)?
            .into_iter()
            .filter_map(|m| arrears.get(&m.id).map(|&(b, o)| (m, b, o)))
            .collect();
        out.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| a.0.name.to_lowercase().cmp(&b.0.name.to_lowercase()))
        });
        Ok(out)
    }

    /// Active members with no payment recorded for `month`.
    pub fn due_members(&self, month: &str) -> rusqlite::Result<Vec<Member>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM members
             WHERE active=1
               AND id NOT IN (SELECT member_id FROM payments
                              WHERE period_month=?1 AND category != 'registration')
             ORDER BY name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([month], member_from_row)?;
        rows.collect()
    }

    /// Sum of payments of one `category` ('membership'|'registration')
    /// whose `date` falls in [start, end] (inclusive ISO date strings).
    pub fn category_income(
        &self,
        category: &str,
        start: &str,
        end: &str,
    ) -> rusqlite::Result<f64> {
        let v: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(amount),0) FROM payments
             WHERE category=?1 AND date >= ?2 AND date <= ?3",
            params![category, start, end],
            |r| r.get(0),
        )?;
        Ok(v)
    }

    // ---- Products -------------------------------------------------------

    pub fn insert_product(&self, p: &Product) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO products(name, price, stock, active) VALUES (?1, ?2, ?3, ?4)",
            params![p.name, p.price, p.stock, p.active as i64],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_product(&self, p: &Product) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE products SET name=?1, price=?2, stock=?3, active=?4 WHERE id=?5",
            params![p.name, p.price, p.stock, p.active as i64, p.id],
        )?;
        Ok(())
    }

    pub fn delete_product(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM products WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn list_products(&self) -> rusqlite::Result<Vec<Product>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM products ORDER BY name COLLATE NOCASE")?;
        let rows = stmt.query_map([], product_from_row)?;
        rows.collect()
    }

    pub fn low_stock_products(&self, threshold: i64) -> rusqlite::Result<Vec<Product>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM products WHERE active=1 AND stock <= ?1 ORDER BY stock ASC",
        )?;
        let rows = stmt.query_map([threshold], product_from_row)?;
        rows.collect()
    }

    // ---- Sales (anonymous, with line items) -----------------------------

    /// Record a sale and its items in one transaction, decrementing stock.
    pub fn record_sale(
        &mut self,
        date: &str,
        items: &[(i64, i64, f64)], // (product_id, qty, unit_price)
    ) -> rusqlite::Result<i64> {
        let total: f64 = items.iter().map(|(_, q, p)| *q as f64 * *p).sum();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO sales(date, total, created_at) VALUES (?1, ?2, ?3)",
            params![date, total, dates::now()],
        )?;
        let sale_id = tx.last_insert_rowid();
        for (product_id, qty, unit_price) in items {
            tx.execute(
                "INSERT INTO sale_items(sale_id, product_id, qty, unit_price)
                 VALUES (?1, ?2, ?3, ?4)",
                params![sale_id, product_id, qty, unit_price],
            )?;
            tx.execute(
                "UPDATE products SET stock = stock - ?1 WHERE id = ?2",
                params![qty, product_id],
            )?;
        }
        tx.commit()?;
        Ok(sale_id)
    }

    /// Insert a sale exactly as given (CSV import). Unlike `record_sale`, this
    /// preserves the recorded total and does NOT adjust product stock, since
    /// imported product ids may not exist in this database.
    pub fn import_sale(
        &mut self,
        date: &str,
        total: f64,
        items: &[(Option<i64>, i64, f64)], // (product_id, qty, unit_price)
    ) -> rusqlite::Result<i64> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO sales(date, total, created_at) VALUES (?1, ?2, ?3)",
            params![date, total, dates::now()],
        )?;
        let sale_id = tx.last_insert_rowid();
        for (product_id, qty, unit_price) in items {
            tx.execute(
                "INSERT INTO sale_items(sale_id, product_id, qty, unit_price)
                 VALUES (?1, ?2, ?3, ?4)",
                params![sale_id, product_id, qty, unit_price],
            )?;
        }
        tx.commit()?;
        Ok(sale_id)
    }

    pub fn list_sales(&self) -> rusqlite::Result<Vec<Sale>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM sales ORDER BY date DESC, id DESC")?;
        let rows = stmt.query_map([], |r| {
            Ok(Sale {
                id: r.get("id")?,
                date: r.get("date")?,
                total: r.get("total")?,
            })
        })?;
        rows.collect()
    }

    pub fn list_sales_between(&self, start: &str, end: &str) -> rusqlite::Result<Vec<Sale>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM sales WHERE date BETWEEN ?1 AND ?2 ORDER BY date DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![start, end], |r| {
            Ok(Sale {
                id: r.get("id")?,
                date: r.get("date")?,
                total: r.get("total")?,
            })
        })?;
        rows.collect()
    }

    pub fn sale_items(&self, sale_id: i64) -> rusqlite::Result<Vec<SaleItem>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM sale_items WHERE sale_id=?1")?;
        let rows = stmt.query_map([sale_id], |r| {
            Ok(SaleItem {
                id: r.get("id")?,
                sale_id: r.get("sale_id")?,
                product_id: r.get("product_id")?,
                qty: r.get("qty")?,
                unit_price: r.get("unit_price")?,
            })
        })?;
        rows.collect()
    }

    /// Delete a sale and restore stock for each of its line items.
    pub fn delete_sale(&mut self, id: i64) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt =
                tx.prepare("SELECT product_id, qty FROM sale_items WHERE sale_id=?1")?;
            let rows = stmt
                .query_map([id], |r| {
                    Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, i64>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for (product_id, qty) in rows {
                if let Some(pid) = product_id {
                    tx.execute(
                        "UPDATE products SET stock = stock + ?1 WHERE id = ?2",
                        params![qty, pid],
                    )?;
                }
            }
        }
        tx.execute("DELETE FROM sales WHERE id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    /// Replace a sale's line items, adjusting stock by the net difference.
    /// Simple implementation: restore old stock, then record new items.
    pub fn update_sale(
        &mut self,
        sale_id: i64,
        date: &str,
        items: &[(i64, i64, f64)],
    ) -> rusqlite::Result<()> {
        let total: f64 = items.iter().map(|(_, q, p)| *q as f64 * *p).sum();
        let tx = self.conn.transaction()?;
        {
            let mut stmt =
                tx.prepare("SELECT product_id, qty FROM sale_items WHERE sale_id=?1")?;
            let rows = stmt
                .query_map([sale_id], |r| {
                    Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, i64>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for (product_id, qty) in rows {
                if let Some(pid) = product_id {
                    tx.execute(
                        "UPDATE products SET stock = stock + ?1 WHERE id = ?2",
                        params![qty, pid],
                    )?;
                }
            }
        }
        tx.execute("DELETE FROM sale_items WHERE sale_id=?1", [sale_id])?;
        tx.execute(
            "UPDATE sales SET date=?1, total=?2 WHERE id=?3",
            params![date, total, sale_id],
        )?;
        for (product_id, qty, unit_price) in items {
            tx.execute(
                "INSERT INTO sale_items(sale_id, product_id, qty, unit_price)
                 VALUES (?1, ?2, ?3, ?4)",
                params![sale_id, product_id, qty, unit_price],
            )?;
            tx.execute(
                "UPDATE products SET stock = stock - ?1 WHERE id = ?2",
                params![qty, product_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn merch_income(&self, start: &str, end: &str) -> rusqlite::Result<f64> {
        let v: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total),0) FROM sales WHERE date >= ?1 AND date <= ?2",
            params![start, end],
            |r| r.get(0),
        )?;
        Ok(v)
    }

    /// Daily total revenue (payments + sales) in [start, end]. Missing days
    /// are omitted; caller can zero-fill against `dates::days_inclusive`.
    pub fn daily_revenue(
        &self,
        start: &str,
        end: &str,
    ) -> rusqlite::Result<std::collections::HashMap<String, f64>> {
        let mut out: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT date, SUM(amount) FROM payments WHERE date >= ?1 AND date <= ?2 GROUP BY date",
        )?;
        for row in stmt.query_map(params![start, end], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        })? {
            let (d, v) = row?;
            *out.entry(d).or_default() += v;
        }
        let mut stmt = self.conn.prepare(
            "SELECT date, SUM(total) FROM sales WHERE date >= ?1 AND date <= ?2 GROUP BY date",
        )?;
        for row in stmt.query_map(params![start, end], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        })? {
            let (d, v) = row?;
            *out.entry(d).or_default() += v;
        }
        Ok(out)
    }

    pub fn merch_units(&self, start: &str, end: &str) -> rusqlite::Result<i64> {
        let v: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(si.qty),0) FROM sale_items si
             JOIN sales s ON s.id = si.sale_id
             WHERE s.date >= ?1 AND s.date <= ?2",
            params![start, end],
            |r| r.get(0),
        )?;
        Ok(v)
    }

    // ---- Expenses -------------------------------------------------------

    pub fn insert_expense(&self, e: &Expense) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO expenses(name, amount, date, note, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![e.name, e.amount, e.date, e.note, dates::now()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_expense(&self, e: &Expense) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE expenses SET name=?1, amount=?2, date=?3, note=?4 WHERE id=?5",
            params![e.name, e.amount, e.date, e.note, e.id],
        )?;
        Ok(())
    }

    pub fn delete_expense(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM expenses WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn get_expense(&self, id: i64) -> rusqlite::Result<Option<Expense>> {
        self.conn
            .query_row("SELECT * FROM expenses WHERE id=?1", [id], expense_from_row)
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
    }

    pub fn list_expenses(&self) -> rusqlite::Result<Vec<Expense>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM expenses ORDER BY date DESC, id DESC")?;
        let rows = stmt.query_map([], expense_from_row)?;
        rows.collect()
    }

    pub fn total_expenses(&self, start: &str, end: &str) -> rusqlite::Result<f64> {
        let v: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(amount),0) FROM expenses WHERE date >= ?1 AND date <= ?2",
            params![start, end],
            |r| r.get(0),
        )?;
        Ok(v)
    }
}
