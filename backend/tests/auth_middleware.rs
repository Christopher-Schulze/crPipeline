use actix_web::{test, FromRequest};
use backend::middleware::auth::AuthUser;
use backend::middleware::jwt::create_jwt;
use uuid::Uuid;

#[actix_rt::test]
async fn extractor_from_header() {
    std::env::set_var("JWT_SECRET", "secret");
    let user_id = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let token = create_jwt(user_id, org_id, "user").unwrap();
    let req = test::TestRequest::default()
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_http_request();
    let auth = AuthUser::from_request(&req, &mut actix_web::dev::Payload::None).await.unwrap();
    assert_eq!(auth.user_id, user_id);
    assert_eq!(auth.org_id, org_id);
}

#[actix_rt::test]
async fn extractor_from_cookie() {
    std::env::set_var("JWT_SECRET", "secret");
    let user_id = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let token = create_jwt(user_id, org_id, "user").unwrap();
    let req = test::TestRequest::default()
        .cookie(actix_web::cookie::Cookie::new("token", token))
        .to_http_request();
    let auth = AuthUser::from_request(&req, &mut actix_web::dev::Payload::None).await.unwrap();
    assert_eq!(auth.user_id, user_id);
    assert_eq!(auth.org_id, org_id);
}
