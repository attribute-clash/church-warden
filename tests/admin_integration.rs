use church_warden::{
    Database,
    models::Role,
    modules::{admin::AdminService, auth::AuthService},
};

#[test]
fn admin_integration_can_change_role() {
    let db = Database::in_memory().unwrap();
    let auth = AuthService::new(&db);
    let uid = auth
        .create_user("u2", "pw", "QR:U2", Role::ShopWorker)
        .unwrap();
    let admin = AdminService::new(&db);

    admin.set_role(uid, Role::Supervisor).unwrap();
    let sections = admin.visible_sections(uid).unwrap();
    assert!(sections.iter().any(|s| s == "Склад"));
}
