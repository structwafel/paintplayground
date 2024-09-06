use crate::db::UserPassword;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let argon2 = argon2::Argon2::default();

    let salt = SaltString::generate(&mut OsRng);

    let hashed = match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(h) => h,
        Err(_) => anyhow::bail!("failed to hash password"),
    };

    Ok(hashed.to_string())
}

pub fn verify_password(password: &str, password_data: &UserPassword) -> anyhow::Result<bool> {
    let argon2 = Argon2::default();

    let password_hash = PasswordHash::new(&password_data.password_hash)
        .map_err(|_| anyhow::anyhow!("failed to parse password hash"))?;

    Ok(argon2
        .verify_password(password.as_bytes(), &password_hash)
        .map_err(|_| anyhow::anyhow!("failed to verify password"))
        .is_ok())
}
