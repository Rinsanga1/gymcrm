#[derive(Debug, Clone)]
pub struct Member {
    pub id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub join_date: String,
    pub active: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStatus {
    Paid,
    Due,
    Inactive,
}

#[derive(Debug, Clone)]
pub struct Payment {
    pub id: i64,
    pub member_id: i64,
    /// `YYYY-MM`
    pub period_month: String,
    pub amount: f64,
    pub date: String,
    pub note: Option<String>,
    /// `membership` | `registration`
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub price: f64,
    pub stock: i64,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct Sale {
    pub id: i64,
    pub date: String,
    pub total: f64,
}

#[derive(Debug, Clone)]
pub struct SaleItem {
    pub id: i64,
    pub sale_id: i64,
    pub product_id: Option<i64>,
    pub qty: i64,
    pub unit_price: f64,
}

#[derive(Debug, Clone)]
pub struct Expense {
    pub id: i64,
    pub name: String,
    pub amount: f64,
    pub date: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxnKind {
    Payment,
    Sale,
    Expense,
}

/// A unified money movement for the Transactions history view.
#[derive(Debug, Clone)]
pub struct Txn {
    pub kind: TxnKind,
    pub id: i64,
    pub date: String,
    /// Positive = money in, negative = money out.
    pub amount: f64,
    pub label: String,
    pub detail: Option<String>,
}
