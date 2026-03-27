use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    ShopWorker,
    WarehouseWorker,
    Supervisor,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::ShopWorker => "shop_worker",
            Role::WarehouseWorker => "warehouse_worker",
            Role::Supervisor => "supervisor",
            Role::Admin => "admin",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "shop_worker" => Some(Role::ShopWorker),
            "warehouse_worker" => Some(Role::WarehouseWorker),
            "supervisor" => Some(Role::Supervisor),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub qr_token: String,
    pub role: Role,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct Product {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub price: f64,
    pub stock_qty: i64,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct SaleItem {
    pub product_id: i64,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct InventoryReceiptItem {
    pub product_code: String,
    pub product_name: String,
    pub qty: i64,
    pub price: f64,
}
