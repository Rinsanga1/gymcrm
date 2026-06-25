use rusqlite::{params, Connection, Row};

use super::models::{Expense, Member, MemberStatus, Payment, Product, Sale, SaleItem};

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
            params![m.name, m.phone, m.join_date, m.active as i64, m.notes],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_member(&self, m: &Member) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE members SET name=?1, phone=?2, join_date=?3, active=?4, notes=?5
             WHERE id=?6",
            params![m.name, m.phone, m.join_date, m.active as i64, m.notes, m.id],
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
            "INSERT INTO payments(member_id, period_month, amount, date, note)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![p.member_id, p.period_month, p.amount, p.date, p.note],
        )?;
        Ok(self.conn.last_insert_rowid())
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
            "SELECT COUNT(*) FROM payments WHERE member_id=?1 AND period_month=?2",
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
            .prepare("SELECT DISTINCT member_id FROM payments WHERE period_month=?1")?;
        let rows = stmt.query_map([month], |r| r.get::<_, i64>(0))?;
        let mut set = std::collections::HashSet::new();
        for id in rows {
            set.insert(id?);
        }
        Ok(set)
    }

    /// Active members with no payment recorded for `month`.
    pub fn due_members(&self, month: &str) -> rusqlite::Result<Vec<Member>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM members
             WHERE active=1
               AND id NOT IN (SELECT member_id FROM payments WHERE period_month=?1)
             ORDER BY name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([month], member_from_row)?;
        rows.collect()
    }

    pub fn member_status(&self, m: &Member, month: &str) -> rusqlite::Result<MemberStatus> {
        if !m.active {
            return Ok(MemberStatus::Inactive);
        }
        Ok(if self.is_paid(m.id, month)? {
            MemberStatus::Paid
        } else {
            MemberStatus::Due
        })
    }

    /// Sum of membership payments whose `date` falls in [start, end] (inclusive,
    /// ISO date strings).
    pub fn membership_income(&self, start: &str, end: &str) -> rusqlite::Result<f64> {
        let v: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(amount),0) FROM payments WHERE date >= ?1 AND date <= ?2",
            params![start, end],
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
            "INSERT INTO sales(date, total) VALUES (?1, ?2)",
            params![date, total],
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
            "INSERT INTO expenses(amount, date, note) VALUES (?1, ?2, ?3)",
            params![e.amount, e.date, e.note],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_expense(&self, e: &Expense) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE expenses SET amount=?1, date=?2, note=?3 WHERE id=?4",
            params![e.amount, e.date, e.note, e.id],
        )?;
        Ok(())
    }

    pub fn delete_expense(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM expenses WHERE id=?1", [id])?;
        Ok(())
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
