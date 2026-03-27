use rusqlite::{OptionalExtension, params};

use crate::{
    db::Database,
    error::{AppError, AppResult},
    models::Role,
};

pub struct AdminService<'a> {
    db: &'a Database,
}

impl<'a> AdminService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn set_role(&self, user_id: i64, role: Role) -> AppResult<()> {
        let affected = self.db.conn().execute(
            "UPDATE users SET role=?1 WHERE id=?2",
            params![role.as_str(), user_id],
        )?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("user {user_id}")));
        }
        Ok(())
    }

    pub fn set_user_active(&self, user_id: i64, is_active: bool) -> AppResult<()> {
        let affected = self.db.conn().execute(
            "UPDATE users SET is_active=?1 WHERE id=?2",
            params![if is_active { 1 } else { 0 }, user_id],
        )?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("user {user_id}")));
        }
        Ok(())
    }

    pub fn visible_sections(&self, user_id: i64) -> AppResult<Vec<String>> {
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

        let sections = match Role::from_db(&role).unwrap_or(Role::ShopWorker) {
            Role::ShopWorker => vec!["Лавка"],
            Role::WarehouseWorker => vec!["Лавка", "Склад"],
            Role::Supervisor => vec!["Лавка", "Склад"],
            Role::Admin => vec!["Лавка", "Склад", "Отчёты", "Администрирование"],
        };
        Ok(sections.into_iter().map(str::to_string).collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Database, models::Role, modules::auth::AuthService};

    use super::AdminService;

    #[test]
    fn role_visibility_for_admin() {
        let db = Database::in_memory().unwrap();
        let auth = AuthService::new(&db);
        let user_id = auth
            .create_user("admin", "password", "QR:ADMIN", Role::Admin)
            .unwrap();
        let admin = AdminService::new(&db);

        let sections = admin.visible_sections(user_id).unwrap();
        assert_eq!(sections.len(), 4);
    }

    #[test]
    fn can_disable_user() {
        let db = Database::in_memory().unwrap();
        let auth = AuthService::new(&db);
        let user_id = auth
            .create_user("employee", "password", "QR:EMP", Role::ShopWorker)
            .unwrap();
        let admin = AdminService::new(&db);
        admin.set_user_active(user_id, false).unwrap();

        assert!(auth.login_with_password("employee", "password").is_err());
    }
}
