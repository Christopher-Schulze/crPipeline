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
