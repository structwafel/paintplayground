use crate::types::*;
use sqlx::sqlite::SqlitePool;

#[derive(Debug, Clone)]
pub struct DB {
    pub pool: SqlitePool,
}

impl DB {
    pub async fn new() -> anyhow::Result<Self> {
        let pool = connect_db().await?;

        Ok(Self { pool })
    }

    pub async fn get_user_by_username(
        &self,
        username: &str,
    ) -> anyhow::Result<Option<super::User>> {
        let user = query_as!(
            super::User,
            "SELECT * FROM users WHERE username = $1",
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_password_by_user_id(
        &self,
        user_id: &str,
    ) -> anyhow::Result<Option<super::UserPassword>> {
        let user_password = query_as!(
            super::UserPassword,
            "SELECT * FROM passwords WHERE user_id = $1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user_password)
    }

    pub async fn create_user(
        &self,
        user: &super::User,
        user_password: &super::UserPassword,
    ) -> anyhow::Result<()> {
        let mut transaction = self.pool.begin().await?;

        query!(
            "INSERT INTO users (id, username, xlocation, ylocation) VALUES ($1, $2, $3, $4)",
            user.id,
            user.username,
            user.xlocation,
            user.ylocation
        )
        .execute(&mut *transaction)
        .await?;

        query!(
            "INSERT INTO passwords (user_id, password_hash, password_salt) VALUES ($1, $2, $3)",
            user_password.user_id,
            user_password.password_hash,
            user_password.password_salt
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(())
    }

    pub async fn add_token(&self, user_id: &str, token: &str) -> anyhow::Result<()> {
        query!(
            "INSERT INTO jwt_tokens (user_id, token) VALUES ($1, $2)",
            user_id,
            token
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

pub async fn connect_db() -> anyhow::Result<SqlitePool> {
    let pool = SqlitePool::connect(&std::env::var("DATABASE_URL")?).await?;

    Ok(pool)
}
