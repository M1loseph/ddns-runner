use crate::duck_dns_client::client::DuckDnsClient;
use crate::duck_dns_client::client::IPUpdateResult as IPUpdateResultAPI;
use crate::duck_dns_client::client::ServerAction as ServerActionAPI;
use anyhow::Result;
use log::{error, info};
use prometheus::{IntCounterVec, Registry};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::{Duration, SystemTime},
};
use tokio::time::sleep;

pub enum ServerAction {
    NoChange,
    Updated,
}

impl ServerAction {
    fn to_str(&self) -> &str {
        match self {
            ServerAction::NoChange => "NoChange",
            ServerAction::Updated => "Updated",
        }
    }

    fn from_str(string: &str) -> Option<ServerAction> {
        match string {
            "NoChange" => Some(ServerAction::NoChange),
            "Updated" => Some(ServerAction::Updated),
            _ => None,
        }
    }
}

impl From<ServerActionAPI> for ServerAction {
    fn from(value: ServerActionAPI) -> Self {
        match value {
            ServerActionAPI::Updated => ServerAction::Updated,
            ServerActionAPI::NoChange => ServerAction::NoChange,
        }
    }
}
pub struct IpUpdateResult {
    #[allow(dead_code)]
    pub id: Option<i64>,
    pub server_action: ServerAction,
    pub ipv4: Ipv4Addr,
    pub ipv6: Option<Ipv6Addr>,
    pub inserted_at: SystemTime,
}

impl From<IPUpdateResultAPI> for IpUpdateResult {
    fn from(value: IPUpdateResultAPI) -> Self {
        IpUpdateResult {
            id: None,
            server_action: ServerAction::from(value.server_action),
            ipv4: value.current_ip_v4,
            ipv6: value.current_ip_v6,
            inserted_at: SystemTime::now(),
        }
    }
}

impl From<tokio_postgres::Row> for IpUpdateResult {
    fn from(row: tokio_postgres::Row) -> Self {
        let ipv4: Ipv4Addr = match row.get("ipv4") {
            IpAddr::V4(v4) => v4,
            _ => panic!("Something else than IPV4 is stored in the IpUpdateResults table!"),
        };

        let ipv6: Option<Ipv6Addr>  = match row.get("ipv6") {
            Some(ip_address) => match ip_address {
                IpAddr::V6(v6) => Some(v6),
                _ => panic!("Something else than IPV6 is stored in the IpUpdateResults table!"),
            },
            None => None,
        };

        IpUpdateResult {
            id: Some(row.get("id")),
            server_action: ServerAction::from_str(row.get("server_action")).unwrap(),
            ipv4,
            ipv6,
            inserted_at: row.get("inserted_at"),
        }
    }
}

pub struct IpUpdateResultPostgresRepository {
    client: tokio_postgres::Client,
}

impl IpUpdateResultPostgresRepository {
    pub fn new(client: tokio_postgres::Client) -> Self {
        IpUpdateResultPostgresRepository { client }
    }
    
    async fn insert(&self, domain: &str, entity: IpUpdateResult) -> Result<IpUpdateResult> {
        self.client
            .execute(
                r#"
                    INSERT INTO "Domains"("domain_name")
                    VALUES ($1)
                    ON CONFLICT ("domain_name") DO NOTHING"#,
                &[&domain],
            )
            .await?;
        let row = self
            .client
            .query_one(
                r#"
                    INSERT INTO "IpUpdateResults"("server_action", "ipv4", "ipv6", "inserted_at", "domain_id")
                    VALUES ($1, $2, $3, $4, (SELECT "id" FROM "Domains" WHERE "domain_name" = $5))
                    RETURNING *"#,
                &[
                    &entity.server_action.to_str(),
                    &IpAddr::V4(entity.ipv4),
                    &entity.ipv6.map(|v6| IpAddr::V6(v6)),
                    &entity.inserted_at,
                    &domain,
                ],
            )
            .await?;
        Ok(IpUpdateResult::from(row))
    }
}

pub struct UpdateIpTask {
    duck_dns_client: DuckDnsClient,
    ip_results_counters: IntCounterVec,
    ip_update_result_repository: IpUpdateResultPostgresRepository,
    sleep_time: Duration,
    domains_to_update: Vec<String>,
}

impl UpdateIpTask {
    pub fn new(
        registry: &Registry,
        duck_dns_client: DuckDnsClient,
        ip_update_result_repository: IpUpdateResultPostgresRepository,
        sleep_time: Duration,
        domains_to_update: Vec<String>,
    ) -> Result<Self> {
        let ddns_request_counter = IntCounterVec::new(
            prometheus::Opts::new(
                "ddnsrunner_ddns_requests_total",
                "Number of successful updates in duck dns.",
            ),
            &["result"],
        )?;

        registry.register(Box::new(ddns_request_counter.clone()))?;

        Ok(UpdateIpTask {
            duck_dns_client,
            ip_results_counters: ddns_request_counter,
            ip_update_result_repository,
            sleep_time,
            domains_to_update,
        })
    }

    pub async fn update_dns_record_task(&self) {
        info!("Starting update_dns_record_task");
        loop {
            for domain_to_update in &self.domains_to_update {
                match self.duck_dns_client.update_ip(domain_to_update).await {
                    Ok(response) => {
                        info!("Got response from Duck DNS for domain {}: {:?}", domain_to_update, response);
                        let entity = IpUpdateResult::from(response);
                        let update_result = self
                            .ip_update_result_repository
                            .insert(&domain_to_update, entity)
                            .await;
                        if let Err(e) = update_result {
                            panic!("Error while saving the IP update result to the database: {}", e);
                        }
                        self.ip_results_counters
                            .with_label_values(&["success"])
                            .inc();
                    }
                    Err(e) => {
                        error!("Got error from Duck DNS: {:?}", e);
                        self.ip_results_counters
                            .with_label_values(&["failure"])
                            .inc();
                    }
                };
            }
            sleep(self.sleep_time).await;
        }
    }
}
