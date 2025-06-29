use lettre::message::{header, Mailbox, Message};
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport,
    Tokio1Executor,
};
use once_cell::sync::Lazy;
use std::env;

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
