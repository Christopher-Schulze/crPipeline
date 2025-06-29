use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use backend::handlers::document::{cleanup_s3_object, S3Deleter};

#[derive(Clone, Default)]
struct MockS3 {
    calls: Arc<AtomicUsize>,
    fail: bool,
}

#[async_trait]
impl S3Deleter for MockS3 {
    async fn delete_object(&self, _bucket: &str, _key: &str) -> Result<(), Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err(anyhow!("fail"))
        } else {
            Ok(())
        }
    }
}

#[actix_rt::test]
async fn cleanup_invokes_delete() {
    let mock = MockS3::default();
    cleanup_s3_object(&mock, "b", "k").await;
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
}

#[actix_rt::test]
async fn cleanup_logs_error() {
    let mock = MockS3 { fail: true, ..Default::default() };
    cleanup_s3_object(&mock, "b", "k").await;
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
}
