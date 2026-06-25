use super::db::open_memory;
use super::models::{Expense, Member, MemberStatus, Payment, Product};
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
    }
}

#[test]
fn seeded_settings_defaults() {
    let r = repo();
    assert_eq!(r.default_monthly_fee(), 1500.0);
    assert_eq!(r.currency(), "Rs");
    assert_eq!(r.get_setting("gym_name").unwrap().as_deref(), Some("My Gym"));
}

#[test]
fn unpaid_active_member_is_due() {
    let r = repo();
    let id = r.insert_member(&member("Ava", true)).unwrap();
    let m = r.get_member(id).unwrap().unwrap();
    assert_eq!(r.member_status(&m, "2026-06").unwrap(), MemberStatus::Due);
}

#[test]
fn paying_clears_due_even_when_partial() {
    let r = repo();
    let id = r.insert_member(&member("Ben", true)).unwrap();
    // half payment still flips to Paid (binary status)
    r.insert_payment(&payment(id, "2026-06", 750.0)).unwrap();
    let m = r.get_member(id).unwrap().unwrap();
    assert_eq!(r.member_status(&m, "2026-06").unwrap(), MemberStatus::Paid);
    // but a different month is still Due
    assert_eq!(r.member_status(&m, "2026-07").unwrap(), MemberStatus::Due);
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
        r.membership_income("2026-06-01", "2026-06-30").unwrap(),
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
    assert_eq!(r.member_status(&m, "2026-06").unwrap(), MemberStatus::Inactive);
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
        amount: 40000.0,
        date: "2026-06-01".into(),
        note: Some("Rent".into()),
    })
    .unwrap();
    r.insert_expense(&Expense {
        id: 0,
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
