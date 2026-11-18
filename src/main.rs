use std::{path::PathBuf, process::ExitCode};

use skilj::local::{LocalConfig, LocalPostgres};
use skilj_shop::{demo::Demo, Error};

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Error> {
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()?;
    let local = if std::env::var_os("DATABASE_URL").is_none() {
        let mut config = LocalConfig::from_env()?;
        if std::env::var_os("SKILJ_LOCAL_DATA_DIR").is_none() {
            config.data_dir = PathBuf::from(".demo/postgres");
        }
        Some(LocalPostgres::start(config).await?)
    } else {
        None
    };
    let database_url = match &local {
        Some(postgres) => postgres.database_url().to_string(),
        None => std::env::var("DATABASE_URL")?,
    };
    let demo = Demo::start(&database_url, &PathBuf::from(".demo/keys.json")).await?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    println!("GraphQL: http://127.0.0.1:{port}/graphql");
    println!("Demo tokens expire after 8 hours; restart to get new tokens.");
    println!("export SELLER_TOKEN='{}'", demo.seller.token);
    println!("export OTHER_SELLER_TOKEN='{}'", demo.other_seller.token);
    println!("export FINANCE_TOKEN='{}'", demo.finance.token);
    println!("Run the queries in queries/; see README.md for the talk walkthrough.");
    let result = axum::serve(listener, demo.router.clone())
        .with_graceful_shutdown(shutdown_signal())
        .await;
    demo.shutdown().await;
    if let Some(postgres) = local {
        postgres.shutdown().await?;
    }
    result?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("could not install SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
