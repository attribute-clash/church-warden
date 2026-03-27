use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use rusqlite::{OptionalExtension, params};

use crate::{
    db::Database,
    error::{AppError, AppResult},
    models::{Role, User},
};

pub struct AuthService<'a> {
    db: &'a Database,
}

impl<'a> AuthService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create_user(
        &self,
        username: &str,
        password: &str,
        qr_token: &str,
        role: Role,
    ) -> AppResult<i64> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::InvalidOperation(format!("password hashing failed: {e}")))?
            .to_string();

        self.db.conn().execute(
            "INSERT INTO users (username, password_hash, qr_token, role, is_active) VALUES (?1, ?2, ?3, ?4, 1)",
            params![username, password_hash, qr_token, role.as_str()],
        )?;

        Ok(self.db.conn().last_insert_rowid())
    }

    pub fn login_with_password(&self, username: &str, password: &str) -> AppResult<User> {
        let user = self.get_user_by_username(username)?;
        if !user.is_active {
            return Err(AppError::AuthenticationFailed);
        }

        let hash =
            PasswordHash::new(&user.password_hash).map_err(|_| AppError::AuthenticationFailed)?;
        if Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_err()
        {
            return Err(AppError::AuthenticationFailed);
        }

        Ok(user)
    }

    pub fn login_with_qr(&self, qr_token: &str) -> AppResult<User> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, username, password_hash, qr_token, role, is_active FROM users WHERE qr_token=?1",
        )?;

        let user = stmt
            .query_row(params![qr_token], map_user)
            .optional()?
            .ok_or(AppError::AuthenticationFailed)?;

        if !user.is_active {
            return Err(AppError::AuthenticationFailed);
        }
        Ok(user)
    }

    fn get_user_by_username(&self, username: &str) -> AppResult<User> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, username, password_hash, qr_token, role, is_active FROM users WHERE username=?1",
        )?;

        stmt.query_row(params![username], map_user)
            .optional()?
            .ok_or(AppError::AuthenticationFailed)
    }
}

fn map_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    let role_string: String = row.get(4)?;
    Ok(User {
        id: row.get(0)?,
        username: row.get(1)?,
        password_hash: row.get(2)?,
        qr_token: row.get(3)?,
        role: Role::from_db(&role_string).unwrap_or(Role::ShopWorker),
        is_active: row.get::<_, i64>(5)? == 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[test]
    fn password_login_works() {
        let db = Database::in_memory().unwrap();
        let auth = AuthService::new(&db);
        auth.create_user("alice", "secret", "QR:ALICE", Role::Admin)
            .unwrap();

        let user = auth.login_with_password("alice", "secret").unwrap();
        assert_eq!(user.username, "alice");
    }

    #[test]
    fn qr_login_works() {
        let db = Database::in_memory().unwrap();
        let auth = AuthService::new(&db);
        auth.create_user("bob", "secret", "QR:BOB", Role::Supervisor)
            .unwrap();

        let user = auth.login_with_qr("QR:BOB").unwrap();
        assert_eq!(user.role, Role::Supervisor);
    }
}
