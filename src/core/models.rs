use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

impl MemberStatus {
    pub fn label(self) -> &'static str {
        match self {
            MemberStatus::Paid => "Paid",
            MemberStatus::Due => "Due",
            MemberStatus::Inactive => "Inactive",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Payment {
    pub id: i64,
    pub member_id: i64,
    /// `YYYY-MM`
    pub period_month: String,
    pub amount: f64,
    pub date: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub price: f64,
    pub stock: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sale {
    pub id: i64,
    pub date: String,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SaleItem {
    pub id: i64,
    pub sale_id: i64,
    pub product_id: Option<i64>,
    pub qty: i64,
    pub unit_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Expense {
    pub id: i64,
    pub amount: f64,
    pub date: String,
    pub note: Option<String>,
}
