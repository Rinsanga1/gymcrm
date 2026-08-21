//! Throwaway production-readiness probe: seed a large gym and time the queries
//! the hot screens run, so we can see what breaks at scale before a real gym
//! does. Not a UI benchmark (egui isn't driven here) — it isolates the database
//! work each frame depends on.
//!
//!   cargo run --release --example stress                 # 2000 members, 10k sales
//!   cargo run --release --example stress -- 5000 40000   # bigger

use std::time::Instant;

use tenne_crm::core::{dates, db, Repository};

fn arg(n: usize, default: i64) -> i64 {
    std::env::args().nth(n).and_then(|s| s.parse().ok()).unwrap_or(default)
}

fn ym(months_back: i64) -> String {
    // Deterministic YYYY-MM, `months_back` before a fixed recent anchor.
    let (mut y, mut m) = (2026, 8i64);
    m -= months_back;
    while m <= 0 {
        m += 12;
        y -= 1;
    }
    format!("{y:04}-{m:02}")
}
fn ymd(months_back: i64, day: i64) -> String {
    format!("{}-{:02}", ym(months_back), (day % 27) + 1)
}

fn seed(conn: &rusqlite::Connection, members: i64, sales: i64, expenses: i64) -> rusqlite::Result<()> {
    conn.execute_batch(
        "DELETE FROM sale_items; DELETE FROM sales; DELETE FROM payments;
         DELETE FROM expenses;   DELETE FROM products; DELETE FROM members;",
    )?;
    conn.execute_batch("BEGIN")?;

    let mut payments = 0i64;
    {
        let mut ins_m = conn.prepare(
            "INSERT INTO members(name, phone, join_date, active, notes) VALUES (?1,?2,?3,1,NULL)",
        )?;
        let mut ins_p = conn.prepare(
            "INSERT INTO payments(member_id, period_month, amount, date, note, category, created_at)
             VALUES (?1,?2,?3,?4,NULL,?5,?6)",
        )?;
        for i in 0..members {
            let joined_ago = i % 24; // spread joins over two years
            ins_m.execute(rusqlite::params![
                format!("Member {i}"),
                format!("03{:09}", i),
                ymd(joined_ago, i % 26),
            ])?;
            let mid = conn.last_insert_rowid();
            // registration + one membership payment per month since joining,
            // skipping the last 2 months for ~everyone so arrears is non-trivial.
            ins_p.execute(rusqlite::params![mid, ym(joined_ago), 500.0, ymd(joined_ago, 1), "registration", "2026-08-01 00:00:00"])?;
            payments += 1;
            let mut mb = joined_ago;
            while mb >= 2 {
                ins_p.execute(rusqlite::params![mid, ym(mb), 1500.0, ymd(mb, 10), "membership", "2026-08-01 00:00:00"])?;
                payments += 1;
                mb -= 1;
            }
        }

        // 100 products
        let mut ins_prod = conn.prepare("INSERT INTO products(name, price, stock, active) VALUES (?1,?2,?3,1)")?;
        for i in 0..100 {
            ins_prod.execute(rusqlite::params![format!("Product {i}"), 500.0 + (i % 50) as f64 * 100.0, 20])?;
        }

        // sales, each with 1-3 items
        let mut ins_s = conn.prepare("INSERT INTO sales(date, total, created_at) VALUES (?1,?2,?3)")?;
        let mut ins_si = conn.prepare("INSERT INTO sale_items(sale_id, product_id, qty, unit_price) VALUES (?1,?2,?3,?4)")?;
        for i in 0..sales {
            ins_s.execute(rusqlite::params![ymd(i % 12, i % 27), 1500.0, "2026-08-01 00:00:00"])?;
            let sid = conn.last_insert_rowid();
            let n = (i % 3) + 1;
            for k in 0..n {
                let pid = ((i + k) % 100) + 1;
                ins_si.execute(rusqlite::params![sid, pid, 1i64, 500.0])?;
            }
        }

        // expenses
        let mut ins_e = conn.prepare("INSERT INTO expenses(name, amount, date, note, created_at) VALUES (?1,?2,?3,NULL,?4)")?;
        for i in 0..expenses {
            ins_e.execute(rusqlite::params![format!("Expense {i}"), 1000.0, ymd(i % 14, i % 27), "2026-08-01 00:00:00"])?;
        }
    }
    conn.execute_batch("COMMIT")?;
    println!("Seeded: {members} members, {payments} payments, {sales} sales, {expenses} expenses\n");
    Ok(())
}

fn time<F: Fn()>(label: &str, f: F) {
    // Warm once, then take the best of a few runs (steady-state frame cost).
    f();
    let mut best = std::time::Duration::MAX;
    for _ in 0..5 {
        let t = Instant::now();
        f();
        best = best.min(t.elapsed());
    }
    let ms = best.as_secs_f64() * 1000.0;
    let flag = if ms > 16.0 { "  <-- OVER ONE FRAME (16ms)" } else { "" };
    println!("  {label:52} {ms:8.2} ms{flag}");
}

fn main() -> rusqlite::Result<()> {
    let members = arg(1, 2000);
    let sales = arg(2, 10000);
    let expenses = arg(3, 5000);

    let path = std::env::temp_dir().join("tenne_stress.db");
    let _ = std::fs::remove_file(&path);
    let conn = db::open_db(&path)?;
    seed(&conn, members, sales, expenses)?;

    let repo = Repository::new(conn);
    let month = dates::current_month();
    let (start, end) = ("2026-08-01".to_string(), "2026-08-31".to_string());

    println!("Per-query timings (best of 5, steady-state):\n");
    println!("DASHBOARD (runs every repaint while the Dashboard tab is open):");
    time("category_income x2 + merch + expenses (KPIs)", || {
        repo.category_income("membership", &start, &end).unwrap();
        repo.category_income("registration", &start, &end).unwrap();
        repo.merch_income(&start, &end).unwrap();
        repo.total_expenses(&start, &end).unwrap();
    });
    time("due_members_with_arrears(month)", || {
        repo.due_members_with_arrears(&month).unwrap();
    });
    time("low_stock_products(5)", || {
        repo.low_stock_products(5).unwrap();
    });
    time("monthly_revenue(12)", || {
        repo.monthly_revenue(12).unwrap();
    });
    time("recent_transactions(10)  [NEW, dashboard]", || {
        repo.recent_transactions(10).unwrap();
    });
    time("list_transactions(None, None)  [OLD, for comparison]", || {
        repo.list_transactions(None, None).unwrap();
    });

    // Full simulated dashboard frame.
    let t = Instant::now();
    for _ in 0..5 {
        repo.count_members(false).unwrap();
        repo.transaction_years().unwrap();
        repo.category_income("membership", &start, &end).unwrap();
        repo.category_income("registration", &start, &end).unwrap();
        repo.merch_income(&start, &end).unwrap();
        repo.merch_units(&start, &end).unwrap();
        repo.total_expenses(&start, &end).unwrap();
        repo.count_members(true).unwrap();
        repo.due_members_with_arrears(&month).unwrap();
        repo.low_stock_products(5).unwrap();
        repo.monthly_revenue(12).unwrap();
        repo.recent_transactions(10).unwrap();
    }
    println!("\n  ONE FULL DASHBOARD FRAME (all queries above): {:.2} ms",
        t.elapsed().as_secs_f64() * 1000.0 / 5.0);

    println!("\nOTHER HOT SCREENS:");
    time("Transactions tab: list_transactions(month)", || {
        repo.list_transactions(Some(&start), Some(&end)).unwrap();
    });
    time("Members tab: list_members(false)", || {
        repo.list_members(false).unwrap();
    });
    time("Shop tab: list_sales()", || {
        repo.list_sales().unwrap();
    });

    Ok(())
}
