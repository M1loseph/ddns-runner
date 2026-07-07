use log::{debug, error};
use prometheus::Result;
use prometheus::{Gauge, Opts, Registry};
use std::time::Duration;
use sysinfo::{get_current_pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::time::sleep;

pub struct PrometheusSystemInfoMetricsTask {
    memory_used: Gauge,
    cpu_used: Gauge,
}

impl PrometheusSystemInfoMetricsTask {
    pub fn new(registry: &Registry) -> Result<Self> {
        let memory_used = Gauge::with_opts(
            Opts::new(
                "ddns_runner_memory",
                "Memory associated with the process in bytes",
            )
            .const_label("type", "used"),
        )?;
        let cpu_used = Gauge::new("ddns_runner_cpu", "Memory used by the process in percents")?;

        registry.register(Box::new(memory_used.clone()))?;
        registry.register(Box::new(cpu_used.clone()))?;

        Ok(PrometheusSystemInfoMetricsTask {
            memory_used,
            cpu_used,
        })
    }

    pub async fn update_system_info(&self) {
        sysinfo::set_open_files_limit(0);
        let mut system_info = System::new_all();
        loop {
            sleep(Duration::from_secs(1)).await;
            let pid = match get_current_pid() {
                Ok(pid) => pid,
                Err(e) => {
                    error!("Error occurred when getting pid - {e}");
                    continue;
                }
            };

            let updated_processes = system_info.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[pid]),
                false,
                ProcessRefreshKind::default().with_cpu().with_memory(),
            );

            debug!("Updated {} processes information", updated_processes);

            let process = match system_info.process(pid) {
                Some(process) => process,
                None => {
                    error!("Unable to find process with pid {}", pid);
                    continue;
                }
            };
            debug!("Process ({pid}) info - cpu={},memory={}", process.cpu_usage(), process.memory());
            let memory_in_bytes = process.memory() as f64;
            let cpu_usage_percent = process.cpu_usage() as f64;
            self.memory_used.set(memory_in_bytes);
            self.cpu_used.set(cpu_usage_percent);
        }
    }
}
