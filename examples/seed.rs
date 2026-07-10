//! Throwaway seed: fills a database with ~2 years of realistic gym activity so
//! the dashboard has something to show. Reuses the app's own repo, so the data
//! is inserted exactly the way the UI would insert it.
//!
//!   cargo run --example seed                       # seeds target/debug/roche.db
//!   cargo run --example seed -- path/to/roche.db   # seeds a specific file
//!
//! Re-running wipes the tables first, so it's idempotent.

use chrono::{Datelike, Local, Months, NaiveDate};

use roche_crm::core::db;
use roche_crm::core::models::{Expense, Member, Payment, Product};
use roche_crm::core::Repository;

/// Tiny deterministic PRNG (LCG) so a reseed reproduces the same gym — no rand
/// dependency for a throwaway script.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 16
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    /// True with probability `pct`/100.
    fn chance(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

const FIRST: [&str; 40] = [
    "Ali", "Ahmed", "Bilal", "Hassan", "Usman", "Fahad", "Kamran", "Imran", "Zain", "Hamza",
    "Sana", "Ayesha", "Fatima", "Maryam", "Hira", "Sara", "Noor", "Iqra", "Amna", "Rida",
    "Daniyal", "Talha", "Saad", "Umar", "Waleed", "Junaid", "Shahzeb", "Faizan", "Adnan", "Bilawal",
    "Mehak", "Zara", "Aiman", "Laiba", "Anza", "Kiran", "Nida", "Sundas", "Areeba", "Mahnoor",
];
const LAST: [&str; 20] = [
    "Khan", "Malik", "Sheikh", "Butt", "Chaudhry", "Raza", "Iqbal", "Hussain", "Awan", "Qureshi",
    "Ansari", "Farooq", "Siddiqui", "Baig", "Dar", "Gill", "Bhatti", "Nawaz", "Rehman", "Tariq",
];

fn ymd(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}
fn ym(d: NaiveDate) -> String {
    d.format("%Y-%m").to_string()
}

fn main() -> rusqlite::Result<()> {
    let path = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new("target/debug/roche.db").to_path_buf());

    let conn = db::open_db(&path)?;
    conn.execute_batch(
        "DELETE FROM sale_items; DELETE FROM sales; DELETE FROM payments;
         DELETE FROM expenses;   DELETE FROM products; DELETE FROM members;
         DELETE FROM recurring_expenses;",
    )?;
    let mut repo = Repository::new(conn);
    repo.set_setting("gym_name", "Roche Fitness")?;

    let mut rng = Rng(0x5eed_1234);
    let today = Local::now().date_naive();
    let this_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();

    // --- Members + their monthly dues ------------------------------------
    // Members trickle in over ~24 months (a growing gym), each on one of a few
    // fee plans. Most months are paid; a recent gap here and there leaves real
    // arrears so "Members due" isn't zero.
    let plans = [1000.0, 1500.0, 1500.0, 1500.0, 2000.0, 2500.0];
    let mut members = 0;
    for _ in 0..52 {
        let name = format!("{} {}", rng.pick(&FIRST), rng.pick(&LAST));
        let joined_ago = rng.below(24) as u32; // 0..24 months back
        // Clamp to today so a member joining this month never lands on a future
        // day (which would push their joining fee above real, current entries).
        let join = (this_month - Months::new(joined_ago)
            + chrono::Duration::days(rng.below(26) as i64))
        .min(today);
        // A few old members have quit.
        let active = !(joined_ago > 10 && rng.chance(15));
        let phone = format!("03{}{}", rng.below(2) + 1, 10_000_000 + rng.below(89_999_999));
        let id = repo.insert_member(&Member {
            id: 0,
            name,
            phone: Some(phone),
            join_date: ymd(join),
            active,
            notes: None,
        })?;
        members += 1;

        let fee = *rng.pick(&plans);

        // Joining fee: most paid it at signup; ~1 in 8 still owes it.
        if rng.chance(88) {
            repo.insert_payment(&Payment {
                id: 0,
                member_id: id,
                period_month: ym(join),
                amount: 500.0,
                date: ymd(join),
                note: None,
                category: "registration".into(),
            })?;
        }

        // Monthly dues from the join month up to (and often including) now.
        let mut m = NaiveDate::from_ymd_opt(join.year(), join.month(), 1).unwrap();
        while m <= this_month {
            let months_back = months_between(m, this_month);
            // Steady payers in the past; the current & previous month are more
            // likely to be outstanding (people pay a few days late).
            let pay_pct = if months_back == 0 {
                55
            } else if months_back == 1 {
                80
            } else {
                92
            };
            if active && rng.chance(pay_pct) {
                let day = 1 + rng.below(24) as i64;
                let paid_on = m + chrono::Duration::days(day);
                repo.insert_payment(&Payment {
                    id: 0,
                    member_id: id,
                    period_month: ym(m),
                    amount: fee,
                    date: ymd(paid_on.min(today)),
                    note: None,
                    category: "membership".into(),
                })?;
            }
            m = m + Months::new(1);
        }
    }

    // --- Merchandise: 100 SKUs ---------------------------------------------
    // Build a big pool of realistic names by combining bases with flavour/size
    // (supplements), colour/size (apparel), then shuffle and take 100 so the
    // catalog is a varied mix rather than all one category.
    let sup_bases = [
        ("Whey Protein", 6500.0), ("Mass Gainer", 7200.0), ("Pre-Workout", 3800.0),
        ("Creatine", 2500.0), ("BCAA", 3200.0), ("Glutamine", 2800.0), ("Fat Burner", 4200.0),
    ];
    let flavors = ["Chocolate", "Vanilla", "Strawberry", "Banana", "Mango", "Cookies & Cream", "Unflavored"];
    let sup_sizes: [(&str, f64); 4] = [("300g", 0.5), ("500g", 0.75), ("1kg", 1.0), ("2kg", 1.8)];
    let apparel = [
        ("T-Shirt", 1500.0), ("Tank Top", 1300.0), ("Shorts", 1800.0), ("Hoodie", 3500.0),
        ("Gym Gloves", 1200.0), ("Lifting Belt", 3500.0), ("Wrist Wraps", 900.0), ("Knee Sleeves", 2200.0),
    ];
    let colors = ["Black", "Grey", "Navy", "Red", "White"];
    let accessories = [
        ("Shaker Bottle", 700.0), ("Water Bottle 1L", 600.0), ("Towel", 900.0), ("Gym Bag", 2600.0),
        ("Resistance Band", 1100.0), ("Jump Rope", 800.0), ("Foam Roller", 2400.0), ("Protein Bar", 350.0),
    ];

    let mut pool: Vec<(String, f64)> = Vec::new();
    for (b, p) in sup_bases {
        for fl in flavors {
            for (sz, mult) in sup_sizes {
                pool.push((format!("{b} {fl} {sz}"), (p * mult / 100.0).round() * 100.0));
            }
        }
    }
    for (b, p) in apparel {
        for c in colors {
            pool.push((format!("{c} {b}"), p));
        }
    }
    for (b, p) in accessories {
        pool.push((b.to_string(), p));
    }
    // Fisher-Yates shuffle with the same deterministic rng.
    for i in (1..pool.len()).rev() {
        let j = rng.below(i as u64 + 1) as usize;
        pool.swap(i, j);
    }

    let mut products: Vec<(i64, String, f64, i64)> = Vec::new();
    for (name, price) in pool.into_iter().take(100) {
        // A handful open near-empty so the low-stock alert has something to say.
        let stock = if rng.chance(12) { rng.below(5) as i64 } else { 5 + rng.below(40) as i64 };
        let id = repo.insert_product(&Product {
            id: 0,
            name: name.clone(),
            price,
            stock,
            active: true,
        })?;
        products.push((id, name, price, stock));
    }

    // --- 800 sales spread across the last 12 months -----------------------
    for _ in 0..800 {
        let back = rng.below(12) as u32;
        let month = this_month - Months::new(back);
        let day = 1 + rng.below(27) as i64;
        let when = (month + chrono::Duration::days(day)).min(today);
        let n_items = 1 + rng.below(3) as usize;
        let mut items = Vec::new();
        for _ in 0..n_items {
            let p = rng.pick(&products);
            let (pid, price) = (p.0, p.2);
            let qty = 1 + rng.below(2) as i64;
            items.push((pid, qty, price));
        }
        repo.record_sale(&ymd(when), &items)?;
    }

    // Those sales drew stock down from small opening balances into the negatives.
    // Reset each product to a realistic on-hand count now (Gym Gloves & Protein
    // Bar stay deliberately low so the low-stock alert has something to say).
    for (id, name, price, stock) in products {
        repo.update_product(&Product { id, name, price, stock, active: true })?;
    }

    // --- Expenses: the recurring cost of running the place ----------------
    // Clamp every date to today so the current month's bills never land in the
    // future and float above real entries in the ledger.
    let cap = |d: NaiveDate| ymd(d.min(today));
    let mut m = this_month - Months::new(13);
    while m <= this_month {
        repo.insert_expense(&exp("Rent", 25000.0, &cap(m + chrono::Duration::days(4))))?;
        repo.insert_expense(&exp("Staff salaries", 30000.0, &cap(m + chrono::Duration::days(2))))?;
        repo.insert_expense(&exp("Electricity", 3500.0 + rng.below(3000) as f64, &cap(m + chrono::Duration::days(9))))?;
        repo.insert_expense(&exp("Water", 1200.0, &cap(m + chrono::Duration::days(9))))?;
        if rng.chance(25) {
            repo.insert_expense(&exp("Equipment repair", 4000.0 + rng.below(12000) as f64, &cap(m + chrono::Duration::days(15))))?;
        }
        m = m + Months::new(1);
    }

    // --- Recurring expense templates (the monthly bills) ------------------
    for (name, amount) in [
        ("Rent", 25000.0),
        ("Staff salaries", 30000.0),
        ("Electricity", 4500.0),
        ("Water", 1200.0),
        ("Internet", 3000.0),
    ] {
        repo.insert_recurring_expense(&roche_crm::core::models::RecurringExpense {
            id: 0,
            name: name.into(),
            amount,
        })?;
    }

    println!("Seeded {members} members, 100 products, 800 sales into {}", path.display());
    Ok(())
}

fn exp(name: &str, amount: f64, date: &str) -> Expense {
    Expense {
        id: 0,
        name: name.into(),
        amount,
        date: date.into(),
        note: None,
    }
}

/// Whole months from `a` to `b` (b >= a), counting year rollovers.
fn months_between(a: NaiveDate, b: NaiveDate) -> i64 {
    (b.year() - a.year()) as i64 * 12 + (b.month() as i64 - a.month() as i64)
}
