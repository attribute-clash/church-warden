use church_warden::{
    Database,
    models::{InventoryReceiptItem, Role, SaleItem},
    modules::{
        auth::AuthService, reports::ReportsService, shop::ShopService, warehouse::WarehouseService,
    },
};

#[test]
fn reports_integration_turnover_and_close_day() {
    let db = Database::in_memory().unwrap();
    let auth = AuthService::new(&db);
    let uid = auth
        .create_user("sup", "pw", "QR:SUP", Role::Supervisor)
        .unwrap();
    let warehouse = WarehouseService::new(&db);
    warehouse
        .register_receipt(
            uid,
            &[InventoryReceiptItem {
                product_code: "R1".into(),
                product_name: "Крестик".into(),
                qty: 11,
                price: 7.0,
            }],
        )
        .unwrap();

    let product_id: i64 = db
        .conn()
        .query_row("SELECT id FROM products WHERE code='R1'", [], |r| r.get(0))
        .unwrap();
    ShopService::new(&db)
        .register_sale(uid, "2026-03-27", &[SaleItem { product_id, qty: 2 }])
        .unwrap();

    let reports = ReportsService::new(&db);
    let rows = reports.turnover_report().unwrap();
    assert_eq!(rows[0].closing_qty, 9);

    reports.close_day(uid, "2026-03-27").unwrap();
    let result =
        ShopService::new(&db).register_sale(uid, "2026-03-27", &[SaleItem { product_id, qty: 1 }]);
    assert!(result.is_err());
}
