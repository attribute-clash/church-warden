use rusqlite::{OptionalExtension, params};

use crate::{
    db::Database,
    error::{AppError, AppResult},
    models::{Role, SaleItem},
};

pub struct ShopService<'a> {
    db: &'a Database,
}

impl<'a> ShopService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn register_sale(&self, user_id: i64, day_key: &str, items: &[SaleItem]) -> AppResult<i64> {
        self.ensure_day_open(day_key)?;
        let role = self.get_user_role(user_id)?;
        if !matches!(role, Role::ShopWorker | Role::Supervisor | Role::Admin) {
            return Err(AppError::AuthorizationDenied);
        }

        let mut total = 0.0;
        for item in items {
            let (price, stock): (f64, i64) = self
                .db
                .conn()
                .query_row(
                    "SELECT price, stock_qty FROM products WHERE id=?1 AND is_active=1",
                    params![item.product_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(format!("product {}", item.product_id)))?;

            if stock < item.qty {
                return Err(AppError::InvalidOperation("insufficient stock".to_string()));
            }
            total += price * item.qty as f64;
        }

        self.db.conn().execute(
            "INSERT INTO sales (user_id, day_key, total) VALUES (?1, ?2, ?3)",
            params![user_id, day_key, total],
        )?;
        let sale_id = self.db.conn().last_insert_rowid();

        for item in items {
            let price: f64 = self.db.conn().query_row(
                "SELECT price FROM products WHERE id=?1",
                params![item.product_id],
                |r| r.get(0),
            )?;
            self.db.conn().execute(
                "INSERT INTO sale_items (sale_id, product_id, qty, price) VALUES (?1, ?2, ?3, ?4)",
                params![sale_id, item.product_id, item.qty, price],
            )?;
            self.db.conn().execute(
                "UPDATE products SET stock_qty=stock_qty-?1 WHERE id=?2",
                params![item.qty, item.product_id],
            )?;
        }
        Ok(sale_id)
    }

    fn ensure_day_open(&self, day_key: &str) -> AppResult<()> {
        let exists = self
            .db
            .conn()
            .query_row(
                "SELECT day_key FROM day_closures WHERE day_key=?1",
                params![day_key],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        if exists.is_some() {
            return Err(AppError::InvalidOperation(format!(
                "day {day_key} is closed"
            )));
        }
        Ok(())
    }

    fn get_user_role(&self, user_id: i64) -> AppResult<Role> {
        let role: String = self
            .db
            .conn()
            .query_row(
                "SELECT role FROM users WHERE id=?1",
                params![user_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("user {user_id}")))?;
        Ok(Role::from_db(&role).unwrap_or(Role::ShopWorker))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Database,
        models::{InventoryReceiptItem, Role, SaleItem},
        modules::{auth::AuthService, warehouse::WarehouseService},
    };

    use super::ShopService;

    #[test]
    fn sale_reduces_stock() {
        let db = Database::in_memory().unwrap();
        let auth = AuthService::new(&db);
        let user_id = auth
            .create_user("cashier", "pw", "QR:CASH", Role::ShopWorker)
            .unwrap();
        let warehouse = WarehouseService::new(&db);
        warehouse
            .register_receipt(
                user_id,
                &[InventoryReceiptItem {
                    product_code: "P002".into(),
                    product_name: "Лампада".into(),
                    qty: 5,
                    price: 100.0,
                }],
            )
            .unwrap();

        let product_id: i64 = db
            .conn()
            .query_row("SELECT id FROM products WHERE code='P002'", [], |r| {
                r.get(0)
            })
            .unwrap();

        let shop = ShopService::new(&db);
        shop.register_sale(user_id, "2026-03-27", &[SaleItem { product_id, qty: 2 }])
            .unwrap();

        let stock: i64 = db
            .conn()
            .query_row(
                "SELECT stock_qty FROM products WHERE id=?1",
                [product_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stock, 3);
    }
}
