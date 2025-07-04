use lettre::message::{header, Mailbox, Message};
use lettre::{AsyncSmtpTransport, Tokio1Executor, transport::smtp::authentication::Credentials, AsyncTransport};
use once_cell::sync::Lazy;
use std::env;

static MAILER: Lazy<Option<AsyncSmtpTransport<Tokio1Executor>>> = Lazy::new(|| {
    let server = env::var("SMTP_SERVER").ok()?;
    let port: u16 = env::var("SMTP_PORT").ok()?.parse().ok()?;
    let user = env::var("SMTP_USERNAME").ok()?;
    let pass = env::var("SMTP_PASSWORD").ok()?;
    let creds = Credentials::new(user, pass);
    Some(AsyncSmtpTransport::<Tokio1Executor>::relay(&server).ok()?
        .port(port)
        .credentials(creds)
        .build())
});

pub async fn send_email(to: &str, subject: &str, body: &str) -> anyhow::Result<()> {
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
        // Mailer not configured; log only
        tracing::info!(to, subject, "email body: {}", body);
    }
    Ok(())
}

use crate::utils::log_action;
use sqlx::PgPool;
use uuid::Uuid;
use tokio::time::{sleep, Duration};

/// Send an email with simple retry logic. If all attempts fail an audit log entry is created.
pub async fn send_email_retry(
    pool: &PgPool,
    org_id: Uuid,
    user_id: Uuid,
    to: &str,
    subject: &str,
    body: &str,
    max_retries: u32,
) -> anyhow::Result<()> {
    let mut attempt = 0;
    loop {
        match send_email(to, subject, body).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                attempt += 1;
                log::warn!("Email send attempt {} failed for {}: {:?}", attempt, to, e);
                if attempt >= max_retries {
                    log::error!("Giving up sending email to {} after {} attempts", to, attempt);
                    log_action(pool, org_id, user_id, &format!("email_failure:{}:{}", to, subject)).await;
                    return Err(e);
                }
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
