use church_warden::{Database, models::Role, modules::auth::AuthService};

#[test]
fn auth_integration_password_and_qr() {
    let db = Database::in_memory().unwrap();
    let auth = AuthService::new(&db);
    auth.create_user("u1", "pw1", "QR:U1", Role::ShopWorker)
        .unwrap();

    let u = auth.login_with_password("u1", "pw1").unwrap();
    assert_eq!(u.username, "u1");
    assert_eq!(auth.login_with_qr("QR:U1").unwrap().id, u.id);
}
