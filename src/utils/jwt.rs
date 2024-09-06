use axum_extra::headers::Cookie;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub user_id: String,
    pub exp: usize,
}

pub async fn validate_jwt(cookie: &Cookie) -> Result<Claims, axum::http::StatusCode> {
    let token = cookie
        .get("jwt")
        .ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
    let decoding_key = DecodingKey::from_secret("your_secret_key".as_ref());
    let validation = Validation::new(Algorithm::HS256);

    decode::<Claims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)
}
