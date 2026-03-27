use rusqlite::{OptionalExtension, params};

use crate::{
    db::Database,
    error::{AppError, AppResult},
    models::InventoryReceiptItem,
};

pub struct WarehouseService<'a> {
    db: &'a Database,
}

impl<'a> WarehouseService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn register_receipt(&self, user_id: i64, items: &[InventoryReceiptItem]) -> AppResult<i64> {
        self.db.conn().execute(
            "INSERT INTO receipts (user_id) VALUES (?1)",
            params![user_id],
        )?;
        let receipt_id = self.db.conn().last_insert_rowid();

        for item in items {
            let product_id = self.upsert_product(item)?;
            self.db.conn().execute(
                "INSERT INTO receipt_items (receipt_id, product_id, qty, price) VALUES (?1, ?2, ?3, ?4)",
                params![receipt_id, product_id, item.qty, item.price],
            )?;

            self.db.conn().execute(
                "UPDATE products SET stock_qty=stock_qty+?1, price=?2 WHERE id=?3",
                params![item.qty, item.price, product_id],
            )?;
        }

        Ok(receipt_id)
    }

    pub fn get_stock_by_code(&self, code: &str) -> AppResult<i64> {
        let qty: i64 = self
            .db
            .conn()
            .query_row(
                "SELECT stock_qty FROM products WHERE code=?1",
                params![code],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(code.to_string()))?;
        Ok(qty)
    }

    fn upsert_product(&self, item: &InventoryReceiptItem) -> AppResult<i64> {
        let existing = self
            .db
            .conn()
            .query_row(
                "SELECT id FROM products WHERE code=?1",
                params![&item.product_code],
                |r| r.get::<_, i64>(0),
            )
            .optional()?;

        if let Some(id) = existing {
            self.db.conn().execute(
                "UPDATE products SET name=?1, price=?2, is_active=1 WHERE id=?3",
                params![item.product_name, item.price, id],
            )?;
            return Ok(id);
        }

        self.db.conn().execute(
            "INSERT INTO products (code, name, price, stock_qty, is_active) VALUES (?1, ?2, ?3, 0, 1)",
            params![item.product_code, item.product_name, item.price],
        )?;
        Ok(self.db.conn().last_insert_rowid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Database, models::Role, modules::auth::AuthService};

    #[test]
    fn receipt_increases_stock() {
        let db = Database::in_memory().unwrap();
        let auth = AuthService::new(&db);
        let user_id = auth
            .create_user("warehouse", "pw", "QR:WAREHOUSE", Role::WarehouseWorker)
            .unwrap();
        let warehouse = WarehouseService::new(&db);
        warehouse
            .register_receipt(
                user_id,
                &[InventoryReceiptItem {
                    product_code: "P001".to_string(),
                    product_name: "Свеча".to_string(),
                    qty: 10,
                    price: 50.0,
                }],
            )
            .unwrap();

        assert_eq!(warehouse.get_stock_by_code("P001").unwrap(), 10);
    }
}
