mod config;
mod duck_dns_client;
mod handlers;
mod system_metrics_task;
mod update_ip_task;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use anyhow::Result;
use axum::routing::get;
use axum::Router;
use duck_dns_client::client::{DuckDnsClient, DuckDnsConfig};
use handlers::{prometheus_handler, AppState};
use migrations::{MigrationRunner, MigrationRunnerConfiguration, PostgresSQLClientAdapter};
use system_metrics_task::PrometheusSystemInfoMetricsTask;
use tokio::net::TcpListener;
use tokio_postgres::NoTls;
use update_ip_task::{IpUpdateResultPostgresRepository, UpdateIpTask};

#[tokio::main]
async fn main() -> Result<()> {
    // required by Docker to exit the container gracefully by SIGTERM without waiting for SIGKILL
    signal_hook::flag::register_conditional_shutdown(
        signal_hook::consts::SIGTERM,
        0,
        Arc::new(AtomicBool::new(true)),
    )?;
    dotenv::read_env_file();
    env_logger::init();

    let registry = prometheus::Registry::new();

    let config = config::read_config_with_default()?;
    let client = DuckDnsClient::new(DuckDnsConfig {
        duck_dns_address: config.duck_dns_address,
        token: config.token,
    });

    let (postgres_client, connection) =
        tokio_postgres::connect(&config.database_uri, NoTls).await?;
    tokio::spawn(async move {
        if let Err(err) = connection.await {
            panic!(
                "Error occurred on connection to the postgres database {}",
                err
            );
        }
    });

    {
        let mut runner_config = MigrationRunnerConfiguration::default();
        runner_config.migrations_files_directory = PathBuf::from(config.migration_files_directory);
        let postgres_adapter = PostgresSQLClientAdapter::new(&postgres_client);
        let runner = MigrationRunner::new(runner_config, postgres_adapter);
        runner.run_migrations().await?;
    }

    let ip_update_result_repository = IpUpdateResultPostgresRepository::new(postgres_client);

    let process_info_metrics = PrometheusSystemInfoMetricsTask::new(&registry)?;
    let update_ip_task = UpdateIpTask::new(
        &registry,
        client,
        ip_update_result_repository,
        config.ip_update_interval,
        config.domains_to_update,
    )?;

    tokio::spawn(async move {
        update_ip_task.update_dns_record_task().await;
    });
    tokio::spawn(async move {
        process_info_metrics.update_system_info().await;
    });

    let app_state = Arc::new(AppState { registry });

    let app = Router::new()
        .route("/internal/status/prometheus", get(prometheus_handler))
        .with_state(app_state);

    let addr = format!("0.0.0.0:{}", config.server_port);
    let listener = TcpListener::bind(&addr).await?;

    axum::serve(listener, app).await?;
    Ok(())
}
