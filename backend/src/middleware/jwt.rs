use jsonwebtoken::{encode, Header, EncodingKey, decode, Validation, DecodingKey};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::env;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub org: Uuid,
    pub role: String,
    pub exp: usize,
}

pub fn create_jwt(user_id: Uuid, org_id: Uuid, role: &str) -> String {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".into());
    let exp = chrono::Utc::now() + chrono::Duration::hours(24);
    let claims = Claims {
        sub: user_id,
        org: org_id,
        role: role.to_string(),
        exp: exp.timestamp() as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).unwrap()
}

pub fn verify_jwt(token: &str) -> Option<Claims> {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".into());
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_ref()), &Validation::default())
        .map(|d| d.claims)
        .ok()
}
