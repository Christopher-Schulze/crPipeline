use backend::processing::{ai_client, ocr};
use serde_json::json;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::method;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

#[actix_rt::test]
async fn ai_client_retries_on_server_error() {
    let server = MockServer::start().await;
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    let _mock = Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            let n = c.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                ResponseTemplate::new(500)
            } else {
                ResponseTemplate::new(200).set_body_json(json!({"ok": true}))
            }
        })
        .mount_as_scoped(&server)
        .await;

    let res = ai_client::run_ai(&json!({"foo": "bar"}), &server.uri(), "", None)
        .await
        .unwrap();
    assert_eq!(res["ok"], true);
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[actix_rt::test]
async fn ocr_client_retries_on_server_error() {
    let server = MockServer::start().await;
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    let _mock = Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            let n = c.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                ResponseTemplate::new(500)
            } else {
                ResponseTemplate::new(200).set_body_string("ok")
            }
        })
        .mount_as_scoped(&server)
        .await;

    let text = ocr::run_external_ocr(&server.uri(), None, b"pdf".to_vec(), "test.pdf")
        .await
        .unwrap();
    assert_eq!(text, "ok");
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}
