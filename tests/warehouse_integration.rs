use church_warden::{
    Database,
    models::{InventoryReceiptItem, Role},
    modules::{auth::AuthService, warehouse::WarehouseService},
};

#[test]
fn warehouse_integration_register_receipt() {
    let db = Database::in_memory().unwrap();
    let auth = AuthService::new(&db);
    let uid = auth
        .create_user("w", "pw", "QR:W", Role::WarehouseWorker)
        .unwrap();
    let warehouse = WarehouseService::new(&db);

    warehouse
        .register_receipt(
            uid,
            &[InventoryReceiptItem {
                product_code: "W1".into(),
                product_name: "Товар".into(),
                qty: 9,
                price: 12.5,
            }],
        )
        .unwrap();

    assert_eq!(warehouse.get_stock_by_code("W1").unwrap(), 9);
}
