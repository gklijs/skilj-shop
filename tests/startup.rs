use std::{net::TcpListener, process::Command};

#[test]
fn occupied_http_port_fails_before_embedded_postgres_setup() -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let address = listener.local_addr()?;
    let output = Command::new(env!("CARGO_BIN_EXE_skilj-shop"))
        .env("PORT", address.port().to_string())
        .env_remove("DATABASE_URL")
        // Fail before touching a real database if startup tries to configure one first.
        .env("SKILJ_LOCAL_PORT", "invalid")
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains(&format!("could not bind HTTP listener at {address}")),
        "{stderr}"
    );
    Ok(())
}
