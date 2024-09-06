#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub xlocation: i64,
    pub ylocation: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserPassword {
    pub user_id: String,
    pub password_hash: String,
    pub password_salt: String,
}
