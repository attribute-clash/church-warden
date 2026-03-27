use church_warden::{
    Database,
    models::{InventoryReceiptItem, Role, SaleItem},
    modules::{auth::AuthService, shop::ShopService, warehouse::WarehouseService},
};

#[test]
fn shop_integration_sale_flow() {
    let db = Database::in_memory().unwrap();
    let auth = AuthService::new(&db);
    let uid = auth
        .create_user("c", "pw", "QR:C", Role::ShopWorker)
        .unwrap();
    let warehouse = WarehouseService::new(&db);
    warehouse
        .register_receipt(
            uid,
            &[InventoryReceiptItem {
                product_code: "S1".into(),
                product_name: "Свеча большая".into(),
                qty: 4,
                price: 30.0,
            }],
        )
        .unwrap();

    let product_id: i64 = db
        .conn()
        .query_row("SELECT id FROM products WHERE code='S1'", [], |r| r.get(0))
        .unwrap();
    let shop = ShopService::new(&db);
    shop.register_sale(uid, "2026-03-27", &[SaleItem { product_id, qty: 1 }])
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
