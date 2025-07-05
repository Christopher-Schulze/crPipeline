use lettre::message::{header, Mailbox, Message};
use lettre::{AsyncSmtpTransport, Tokio1Executor, transport::smtp::authentication::Credentials, AsyncTransport};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Serialize, Deserialize};

use std::env;
use tokio::sync::mpsc::{self, Sender};
use crate::config::AppConfig;

static MAILER: Lazy<Option<AsyncSmtpTransport<Tokio1Executor>>> = Lazy::new(|| {
    let server = env::var("SMTP_SERVER").ok()?;
    let port: u16 = env::var("SMTP_PORT").ok()?.parse().ok()?;
    let user = env::var("SMTP_USERNAME").ok()?;
    let pass = env::var("SMTP_PASSWORD").ok()?;
    let creds = Credentials::new(user, pass);
    Some(
        AsyncSmtpTransport::<Tokio1Executor>::relay(&server)
            .ok()?
            .port(port)
            .credentials(creds)
            .build(),
    )
});

#[derive(Serialize, Deserialize, Clone)]
struct EmailTask {
    to: String,
    subject: String,
    body: String,
}

static SENDER: OnceCell<Sender<EmailTask>> = OnceCell::new();

async fn deliver_email(to: &str, subject: &str, body: &str) -> anyhow::Result<()> {
    if let Ok(endpoint) = env::var("EMAIL_HTTP_ENDPOINT") {
        let client = reqwest::Client::new();
        let _ = client
            .post(&endpoint)
            .json(&serde_json::json!({"to": to, "subject": subject, "body": body}))
            .send()
            .await?;
        return Ok(());
    }
    if let Some(mailer) = MAILER.as_ref() {
        let from = env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@example.com".into());
        let email = Message::builder()
            .from(from.parse::<Mailbox>()?)
            .to(to.parse::<Mailbox>()?)
            .subject(subject)
            .header(header::ContentType::TEXT_PLAIN)
            .body(body.to_string())?;
        mailer.send(email).await?;
    } else {
        tracing::info!(to, subject, "email body: {}", body);
    }
    Ok(())
}

pub async fn enqueue_email(to: &str, subject: &str, body: &str) -> anyhow::Result<()> {
    let task = EmailTask { to: to.to_string(), subject: subject.to_string(), body: body.to_string() };
    if let Some(tx) = SENDER.get() {
        tx.send(task).await.map_err(|e| anyhow::anyhow!("send: {e}"))?;
        return Ok(());
    }
    if let Ok(redis_url) = env::var("REDIS_URL") {
        let payload = serde_json::to_string(&task)?;
        let client = redis::Client::open(redis_url)?;
        let mut conn = client.get_async_connection().await?;
        redis::cmd("LPUSH").arg("emails").arg(payload).query_async::<_, ()>(&mut conn).await?;
        return Ok(());
    }
    deliver_email(to, subject, body).await
}

pub fn start_email_worker(cfg: &AppConfig) {
    if cfg.email_queue_provider == "redis" {
        let redis_url = env::var("REDIS_URL").expect("REDIS_URL required for redis email queue");
        tokio::spawn(async move {
            let client = match redis::Client::open(redis_url) { Ok(c) => c, Err(e) => { tracing::error!(?e, "redis client"); return; } };
            let mut conn = match client.get_async_connection().await { Ok(c) => c, Err(e) => { tracing::error!(?e, "redis conn"); return; } };
            loop {
                let res: redis::RedisResult<Option<(String,String)>> = redis::cmd("BRPOP").arg("emails").arg(0).query_async(&mut conn).await;
                if let Ok(Some((_, payload))) = res {
                    if let Ok(task) = serde_json::from_str::<EmailTask>(&payload) {
                        if let Err(e) = deliver_email(&task.to, &task.subject, &task.body).await {
                            tracing::error!(?e, "email send");
                        }
                    }
                }
            }
        });
    } else {
        let (tx, mut rx) = mpsc::channel::<EmailTask>(cfg.email_queue_size);
        let _ = SENDER.set(tx);
        tokio::spawn(async move {
            while let Some(task) = rx.recv().await {
                if let Err(e) = deliver_email(&task.to, &task.subject, &task.body).await {
                    tracing::error!(?e, "email send");
                }
            }
        });
    }
}
