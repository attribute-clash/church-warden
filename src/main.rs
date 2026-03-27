use church_warden::{
    Database,
    models::{InventoryReceiptItem, Role, SaleItem},
    modules::{
        auth::AuthService, reports::ReportsService, shop::ShopService, warehouse::WarehouseService,
    },
};

fn main() {
    let db = Database::from_path("church_warden.db").expect("db init");
    let auth = AuthService::new(&db);

    let admin_id = auth
        .create_user("admin", "admin123", "QR:ADMIN", Role::Admin)
        .unwrap_or_else(|_| {
            db.conn()
                .query_row("SELECT id FROM users WHERE username='admin'", [], |r| {
                    r.get(0)
                })
                .expect("admin exists")
        });

    let warehouse = WarehouseService::new(&db);
    let _ = warehouse.register_receipt(
        admin_id,
        &[InventoryReceiptItem {
            product_code: "DEMO-001".into(),
            product_name: "Демо товар".into(),
            qty: 1,
            price: 10.0,
        }],
    );

    let product_id: i64 = db
        .conn()
        .query_row("SELECT id FROM products WHERE code='DEMO-001'", [], |r| {
            r.get(0)
        })
        .expect("product exists");

    let shop = ShopService::new(&db);
    let _ = shop.register_sale(admin_id, "2026-03-27", &[SaleItem { product_id, qty: 1 }]);

    let report = ReportsService::new(&db)
        .turnover_report()
        .expect("build report");
    println!(
        "Система инициализирована. Записей в отчете: {}",
        report.len()
    );
}
