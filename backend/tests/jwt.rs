use backend::middleware::jwt::{create_jwt, verify_jwt};
use uuid::Uuid;

#[test]
fn round_trip_jwt() {
    std::env::set_var("JWT_SECRET", "testsecret");
    let user_id = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let token = create_jwt(user_id, org_id, "user");
    let claims = verify_jwt(&token).expect("valid token");
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.org, org_id);
}
