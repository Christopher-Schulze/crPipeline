use jsonwebtoken::{encode, Header, EncodingKey, decode, Validation, DecodingKey};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::{env, process};
use once_cell::sync::Lazy;

static JWT_SECRET: Lazy<String> = Lazy::new(|| {
    env::var("JWT_SECRET").unwrap_or_else(|_| {
        eprintln!("JWT_SECRET environment variable not set");
        process::exit(1);
    })
});

/// Ensure the JWT secret is loaded at startup.
pub fn init_jwt_secret() {
    Lazy::force(&JWT_SECRET);
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub org: Uuid,
    pub role: String,
    pub exp: usize,
}

pub fn create_jwt(user_id: Uuid, org_id: Uuid, role: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = chrono::Utc::now() + chrono::Duration::hours(24);
    let claims = Claims {
        sub: user_id,
        org: org_id,
        role: role.to_string(),
        exp: exp.timestamp() as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET.as_bytes()))
}

pub fn verify_jwt(token: &str) -> Option<Claims> {
    decode::<Claims>(token, &DecodingKey::from_secret(JWT_SECRET.as_bytes()), &Validation::default())
        .map(|d| d.claims)
        .ok()
}
