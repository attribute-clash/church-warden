use rusqlite::{OptionalExtension, params};

use crate::{
    db::Database,
    error::{AppError, AppResult},
    models::Role,
};

pub struct ReportsService<'a> {
    db: &'a Database,
}

#[derive(Debug)]
pub struct TurnoverRow {
    pub product_code: String,
    pub incoming_qty: i64,
    pub outgoing_qty: i64,
    pub closing_qty: i64,
}

impl<'a> ReportsService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn close_day(&self, user_id: i64, day_key: &str) -> AppResult<()> {
        let role = self.user_role(user_id)?;
        if !matches!(role, Role::Supervisor | Role::Admin) {
            return Err(AppError::AuthorizationDenied);
        }

        self.db.conn().execute(
            "INSERT OR IGNORE INTO day_closures (day_key, closed_by) VALUES (?1, ?2)",
            params![day_key, user_id],
        )?;
        Ok(())
    }

    pub fn turnover_report(&self) -> AppResult<Vec<TurnoverRow>> {
        let mut stmt = self.db.conn().prepare(
            "
            SELECT p.code,
                   COALESCE((SELECT SUM(ri.qty) FROM receipt_items ri WHERE ri.product_id = p.id), 0) AS incoming,
                   COALESCE((SELECT SUM(si.qty) FROM sale_items si WHERE si.product_id = p.id), 0) AS outgoing,
                   p.stock_qty AS closing
            FROM products p
            ORDER BY p.code
            ",
        )?;

        let rows = stmt
            .query_map([], |row| {
                Ok(TurnoverRow {
                    product_code: row.get(0)?,
                    incoming_qty: row.get(1)?,
                    outgoing_qty: row.get(2)?,
                    closing_qty: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    fn user_role(&self, user_id: i64) -> AppResult<Role> {
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
        modules::{auth::AuthService, shop::ShopService, warehouse::WarehouseService},
    };

    use super::ReportsService;

    #[test]
    fn supervisor_can_close_day() {
        let db = Database::in_memory().unwrap();
        let auth = AuthService::new(&db);
        let user_id = auth
            .create_user("sup", "pw", "QR:SUP", Role::Supervisor)
            .unwrap();
        let reports = ReportsService::new(&db);

        reports.close_day(user_id, "2026-03-27").unwrap();

        let closed: String = db
            .conn()
            .query_row(
                "SELECT day_key FROM day_closures WHERE day_key='2026-03-27'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(closed, "2026-03-27");
    }

    #[test]
    fn turnover_report_has_values() {
        let db = Database::in_memory().unwrap();
        let auth = AuthService::new(&db);
        let user_id = auth
            .create_user("admin", "pw", "QR:ADMIN", Role::Admin)
            .unwrap();
        let warehouse = WarehouseService::new(&db);
        warehouse
            .register_receipt(
                user_id,
                &[InventoryReceiptItem {
                    product_code: "P100".into(),
                    product_name: "Икона".into(),
                    qty: 20,
                    price: 200.0,
                }],
            )
            .unwrap();
        let product_id: i64 = db
            .conn()
            .query_row("SELECT id FROM products WHERE code='P100'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let shop = ShopService::new(&db);
        shop.register_sale(user_id, "2026-03-27", &[SaleItem { product_id, qty: 3 }])
            .unwrap();

        let report = ReportsService::new(&db).turnover_report().unwrap();
        assert_eq!(report[0].incoming_qty, 20);
        assert_eq!(report[0].outgoing_qty, 3);
        assert_eq!(report[0].closing_qty, 17);
    }
}
