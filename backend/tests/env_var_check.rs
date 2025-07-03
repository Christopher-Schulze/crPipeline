use std::process::Command;

#[test]
fn missing_env_vars_cause_error() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_backend"));
    cmd.env_remove("DATABASE_URL");
    cmd.env("JWT_SECRET", "secret");
    cmd.env("S3_BUCKET", "uploads");
    let output = cmd.output().expect("run backend binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DATABASE_URL"));
}
#[test]
fn missing_jwt_secret_causes_error() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_backend"));
    cmd.env("DATABASE_URL", "postgres://user:pass@localhost/db");
    cmd.env_remove("JWT_SECRET");
    cmd.env("S3_BUCKET", "uploads");
    let output = cmd.output().expect("run backend binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("JWT_SECRET"));
}

#[test]
fn short_jwt_secret_causes_error() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_backend"));
    cmd.env("DATABASE_URL", "postgres://user:pass@localhost/db");
    cmd.env("JWT_SECRET", "short_secret");
    cmd.env("S3_BUCKET", "uploads");
    let output = cmd.output().expect("run backend binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("JWT_SECRET"));
}

#[test]
fn missing_s3_bucket_causes_error() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_backend"));
    cmd.env("DATABASE_URL", "postgres://user:pass@localhost/db");
    cmd.env("JWT_SECRET", "secret");
    cmd.env_remove("S3_BUCKET");
    let output = cmd.output().expect("run backend binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("S3_BUCKET"));
}
