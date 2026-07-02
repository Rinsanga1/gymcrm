use super::db::open_memory;
use super::models::{Expense, Member, Payment, Product};
use super::repo::Repository;

fn repo() -> Repository {
    Repository::new(open_memory().unwrap())
}

fn member(name: &str, active: bool) -> Member {
    Member {
        id: 0,
        name: name.to_string(),
        phone: None,
        join_date: "2026-01-01".into(),
        active,
        notes: None,
        registration_fee_paid: false,
    }
}

fn payment(member_id: i64, month: &str, amount: f64) -> Payment {
    Payment {
        id: 0,
        member_id,
        period_month: month.into(),
        amount,
        date: format!("{month}-15"),
        note: None,
        category: "membership".into(),
    }
}

#[test]
fn income_split_by_category() {
    let r = repo();
    let id = r.insert_member(&member("Ria", true)).unwrap();
    r.insert_payment(&payment(id, "2026-03", 1500.0)).unwrap(); // membership
    r.ensure_registration_payment(id, 500.0, "2026-03-15", "2026-03").unwrap();
    // Idempotent: a second call must not double-count.
    r.ensure_registration_payment(id, 500.0, "2026-03-15", "2026-03").unwrap();

    let (s, e) = ("2026-03-01", "2026-03-31");
    assert_eq!(r.category_income("membership", s, e).unwrap(), 1500.0);
    assert_eq!(r.category_income("registration", s, e).unwrap(), 500.0);
}

#[test]
fn days_inclusive_handles_max_without_panic() {
    // All Time ends at NaiveDate::MAX (9999-12-31); the iterator must terminate
    // instead of overflowing the date add (the old dashboard "All Time" crash).
    let d = crate::core::dates::days_inclusive("9999-12-29", "9999-12-31");
    assert_eq!(d.len(), 3);
}

#[test]
fn seeded_settings_defaults() {
    let r = repo();
    assert_eq!(r.default_monthly_fee(), 1500.0);
    assert_eq!(r.registration_fee(), 500.0);
    assert_eq!(r.currency(), "Rs");
    assert_eq!(r.get_setting("gym_name").unwrap().as_deref(), Some("My Gym"));
}

#[test]
fn member_flags_round_trip_and_registration_count() {
    let r = repo();
    let mut paid = member("Nia", true);
    paid.registration_fee_paid = true;
    let pid = r.insert_member(&paid).unwrap();
    let unpaid_id = r.insert_member(&member("Omar", true)).unwrap();

    let loaded = r.get_member(pid).unwrap().unwrap();
    assert!(loaded.registration_fee_paid);

    // Only Omar still owes the registration fee.
    assert_eq!(r.unpaid_registration_count(true).unwrap(), 1);

    // Clearing it via update is reflected in the count.
    let mut omar = r.get_member(unpaid_id).unwrap().unwrap();
    omar.registration_fee_paid = true;
    r.update_member(&omar).unwrap();
    assert_eq!(r.unpaid_registration_count(true).unwrap(), 0);
}

#[test]
fn unpaid_active_member_is_due() {
    let r = repo();
    let id = r.insert_member(&member("Ava", true)).unwrap();
    let m = r.get_member(id).unwrap().unwrap();
    assert!(m.active && !r.is_paid(id, "2026-06").unwrap());
}

#[test]
fn registration_only_payment_leaves_member_due() {
    // Paying the one-time registration fee must not satisfy the monthly
    // membership, otherwise the member shows "Paid" and the Record-payment
    // button never appears.
    let r = repo();
    let id = r.insert_member(&member("Gita", true)).unwrap();
    r.ensure_registration_payment(id, 500.0, "2026-06-15", "2026-06")
        .unwrap();
    assert!(!r.is_paid(id, "2026-06").unwrap());
    assert!(!r.paid_member_ids("2026-06").unwrap().contains(&id));
    let due_ids: Vec<i64> = r.due_members("2026-06").unwrap().iter().map(|m| m.id).collect();
    assert!(due_ids.contains(&id));
}

#[test]
fn paying_clears_due_even_when_partial() {
    let r = repo();
    let id = r.insert_member(&member("Ben", true)).unwrap();
    // half payment still flips to Paid (binary status)
    r.insert_payment(&payment(id, "2026-06", 750.0)).unwrap();
    assert!(r.is_paid(id, "2026-06").unwrap());
    // but a different month is still Due
    assert!(!r.is_paid(id, "2026-07").unwrap());
}

#[test]
fn multiple_payments_in_one_month_allowed() {
    let r = repo();
    let id = r.insert_member(&member("Cara", true)).unwrap();
    r.insert_payment(&payment(id, "2026-06", 750.0)).unwrap();
    r.insert_payment(&payment(id, "2026-06", 750.0)).unwrap();
    assert!(r.is_paid(id, "2026-06").unwrap());
    assert_eq!(r.payments_for_member(id).unwrap().len(), 2);
    // both count toward income
    assert_eq!(
        r.category_income("membership", "2026-06-01", "2026-06-30").unwrap(),
        1500.0
    );
}

#[test]
fn inactive_member_excluded_from_dues_and_status() {
    let r = repo();
    let active = r.insert_member(&member("Dan", true)).unwrap();
    let quit = r.insert_member(&member("Eli", false)).unwrap();

    let due = r.due_members("2026-06").unwrap();
    let due_ids: Vec<i64> = due.iter().map(|m| m.id).collect();
    assert!(due_ids.contains(&active));
    assert!(!due_ids.contains(&quit), "inactive must not appear in dues");

    let m = r.get_member(quit).unwrap().unwrap();
    assert!(!m.active);
}

#[test]
fn paid_member_drops_out_of_dues() {
    let r = repo();
    let id = r.insert_member(&member("Fay", true)).unwrap();
    assert_eq!(r.due_members("2026-06").unwrap().len(), 1);
    r.insert_payment(&payment(id, "2026-06", 1500.0)).unwrap();
    assert_eq!(r.due_members("2026-06").unwrap().len(), 0);
}

#[test]
fn search_matches_name_and_phone() {
    let r = repo();
    let mut m = member("Gita Sharma", true);
    m.phone = Some("9841001122".into());
    r.insert_member(&m).unwrap();
    r.insert_member(&member("Hari", true)).unwrap();

    assert_eq!(r.search_members("gita", true).unwrap().len(), 1);
    assert_eq!(r.search_members("9841", true).unwrap().len(), 1);
    assert_eq!(r.search_members("zzz", true).unwrap().len(), 0);
}

#[test]
fn sale_decrements_stock_and_totals() {
    let mut r = repo();
    let pid = r
        .insert_product(&Product {
            id: 0,
            name: "Protein".into(),
            price: 2000.0,
            stock: 10,
            active: true,
        })
        .unwrap();
    r.record_sale("2026-06-10", &[(pid, 3, 2000.0)]).unwrap();

    let p = r.list_products().unwrap().into_iter().next().unwrap();
    assert_eq!(p.stock, 7);
    assert_eq!(r.merch_income("2026-06-01", "2026-06-30").unwrap(), 6000.0);
    assert_eq!(r.merch_units("2026-06-01", "2026-06-30").unwrap(), 3);
}

#[test]
fn low_stock_filter() {
    let r = repo();
    r.insert_product(&Product {
        id: 0,
        name: "Glucose".into(),
        price: 100.0,
        stock: 2,
        active: true,
    })
    .unwrap();
    r.insert_product(&Product {
        id: 0,
        name: "Shirt".into(),
        price: 800.0,
        stock: 50,
        active: true,
    })
    .unwrap();
    let low = r.low_stock_products(5).unwrap();
    assert_eq!(low.len(), 1);
    assert_eq!(low[0].name, "Glucose");
}

#[test]
fn expenses_sum_in_range() {
    let r = repo();
    r.insert_expense(&Expense {
        id: 0,
        name: "Rent".into(),
        amount: 40000.0,
        date: "2026-06-01".into(),
        note: Some("Rent".into()),
    })
    .unwrap();
    r.insert_expense(&Expense {
        id: 0,
        name: "Misc".into(),
        amount: 5000.0,
        date: "2026-07-01".into(),
        note: None,
    })
    .unwrap();
    assert_eq!(r.total_expenses("2026-06-01", "2026-06-30").unwrap(), 40000.0);
}

#[test]
fn delete_member_cascades_payments() {
    let r = repo();
    let id = r.insert_member(&member("Ila", true)).unwrap();
    r.insert_payment(&payment(id, "2026-06", 1500.0)).unwrap();
    r.delete_member(id).unwrap();
    assert_eq!(r.payments_for_member(id).unwrap().len(), 0);
}

#[test]
fn month_diff_counts_whole_months() {
    use crate::core::dates::month_diff;
    assert_eq!(month_diff("2026-01", "2026-01"), 0);
    assert_eq!(month_diff("2026-01", "2026-04"), 3);
    assert_eq!(month_diff("2025-11", "2026-02"), 3);
    assert_eq!(month_diff("2026-04", "2026-01"), -3);
    assert_eq!(month_diff("bad", "2026-01"), 0);
}

#[test]
fn membership_arrears_counts_unpaid_months() {
    // Joined Jan, it's now April: 4 months expected (Jan..Apr inclusive).
    let r = repo();
    let mut ava = member("Ava", true);
    ava.join_date = "2026-01-10".into();
    let id = r.insert_member(&ava).unwrap();

    // No payments yet -> 4 months behind, 4 * 1500.
    assert_eq!(r.membership_arrears(id, "2026-01", "2026-04").unwrap(), (4, 6000.0));

    // Pay Jan and Feb -> 2 months behind.
    r.insert_payment(&payment(id, "2026-01", 1500.0)).unwrap();
    r.insert_payment(&payment(id, "2026-02", 1500.0)).unwrap();
    assert_eq!(r.membership_arrears(id, "2026-01", "2026-04").unwrap(), (2, 3000.0));

    // A registration payment must not reduce arrears.
    r.ensure_registration_payment(id, 500.0, "2026-03-01", "2026-03").unwrap();
    assert_eq!(r.membership_arrears(id, "2026-01", "2026-04").unwrap(), (2, 3000.0));

    // Fully paid through April -> zero behind.
    r.insert_payment(&payment(id, "2026-03", 1500.0)).unwrap();
    r.insert_payment(&payment(id, "2026-04", 1500.0)).unwrap();
    assert_eq!(r.membership_arrears(id, "2026-01", "2026-04").unwrap(), (0, 0.0));
}
