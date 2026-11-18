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
        println!("Starting embedded PostgreSQL. The first run downloads the database binaries.");
        Some(LocalPostgres::start(config).await.inspect_err(|_| {
            eprintln!(
                "See the embedded PostgreSQL troubleshooting and DATABASE_URL fallback in README.md."
            );
        })?)
    } else {
        None
    };
    let database_url = match &local {
        Some(postgres) => postgres.database_url().to_string(),
        None => std::env::var("DATABASE_URL")?,
    };
    let demo = Demo::start(&database_url, &PathBuf::from(".demo/keys.json")).await?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let address = listener.local_addr()?;
    println!("Storefront is ready at http://{address}/");
    println!("GraphQL is ready at http://{address}/graphql");
    println!("Copy these exports into another terminal, then run the seller query in the README walkthrough.");
    println!("export GRAPHQL_URL='http://{address}/graphql'");
    println!("export SELLER_TOKEN='{}'", demo.seller.token);
    println!("export OTHER_SELLER_TOKEN='{}'", demo.other_seller.token);
    println!("export FINANCE_TOKEN='{}'", demo.finance.token);
    println!("Demo tokens expire after 8 hours. Copy the new tokens after each restart.");
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
