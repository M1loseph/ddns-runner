use std::env::{self, VarError};
use std::error::Error;
use std::fmt::Display;
use std::result::Result;
use std::time::Duration;

const DEFAULT_PORT: u16 = 3000;
const DEFAULT_INTERVAL: Duration = Duration::from_secs(60 * 5);

#[derive(Debug, PartialEq)]
pub enum ConfigReadingError {
    VariableNotSet { variable_name: String },
    VariableMalformed { variable_name: String },
}

impl ConfigReadingError {
    fn from(value: VarError, variable_name: String) -> Self {
        match value {
            VarError::NotPresent => ConfigReadingError::VariableNotSet { variable_name },
            VarError::NotUnicode(_) => ConfigReadingError::VariableMalformed { variable_name },
        }
    }
}

impl Error for ConfigReadingError {}

impl Display for ConfigReadingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigReadingError::VariableNotSet { variable_name } => {
                write!(f, "Env variable {} is not set", variable_name)
            }
            ConfigReadingError::VariableMalformed { variable_name } => {
                write!(
                    f,
                    "Env variable {} is malformed, can't read it",
                    variable_name
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct ApplicationConfig {
    pub domains_to_update: Vec<String>,
    pub duck_dns_address: String,
    pub token: String,
    pub server_port: u16,
    pub ip_update_interval: Duration,
    pub database_uri: String,
    pub migration_files_directory: String,
}

fn to_env_variable_name(name: &str) -> String {
    static ENV_VAR_PREFIX: &str = "DDNS_RUNNER";
    format!("{}_{}", ENV_VAR_PREFIX, name)
}

fn read_env_variable(name: &str) -> Result<String, ConfigReadingError> {
    let env_variable_name = to_env_variable_name(name);
    env::var(&env_variable_name).map_err(|e| ConfigReadingError::from(e, env_variable_name))
}

pub fn read_config_with_default() -> Result<ApplicationConfig, ConfigReadingError> {
    let domains_to_update = read_env_variable("DOMAINS_TO_UPDATE")?
        .split(';')
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    let token = read_env_variable("TOKEN")?;
    let server_port = read_env_variable("SERVER_PORT")
        .ok()
        .map(|port| port.parse::<u16>().ok())
        .flatten()
        .unwrap_or(DEFAULT_PORT);
    let ip_update_interval = read_env_variable("IP_UPDATE_INTERVAL_SEC")
        .ok()
        .map(|interval| interval.parse::<u64>().ok())
        .flatten()
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_INTERVAL);
    let duck_dns_address = read_env_variable("DUCK_DNS_ADDRESS")?;
    let database_uri = read_env_variable("POSTGRES_URI")?;
    let migration_files_directory = read_env_variable("MIGRATION_FILES_DIRECTORY")?;
    Ok(ApplicationConfig {
        domains_to_update,
        duck_dns_address,
        token,
        server_port,
        ip_update_interval,
        database_uri,
        migration_files_directory,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    // TODO: add this structure to tests using macro
    struct EnvVariableTest<'a> {
        #[allow(dead_code)]
        guard: MutexGuard<'a, i32>,
    }

    impl<'a> EnvVariableTest<'a> {
        fn new() -> Self {
            static ENV_VARIABLE_TEST_MUTEX: Mutex<i32> = Mutex::new(1);
            let guard = ENV_VARIABLE_TEST_MUTEX.lock().unwrap();
            EnvVariableTest { guard }
        }
    }

    impl<'a> Drop for EnvVariableTest<'a> {
        fn drop(&mut self) {
            for (env_var, _) in std::env::vars() {
                std::env::remove_var(env_var);
            }
        }
    }

    #[test]
    fn should_exit_with_error_when_domain_is_missing() {
        let _cleaner = EnvVariableTest::new();
        let error = read_config_with_default().unwrap_err();
        let expected_error = ConfigReadingError::VariableNotSet {
            variable_name: "DDNS_RUNNER_DOMAINS_TO_UPDATE".to_string(),
        };
        assert_eq!(error, expected_error);
    }

    #[test]
    fn should_exit_with_error_when_token_is_missing() {
        let _cleaner = EnvVariableTest::new();
        std::env::set_var("DDNS_RUNNER_DOMAINS_TO_UPDATE", "phdwebsite");
        let error = read_config_with_default().unwrap_err();
        let expected_error = ConfigReadingError::VariableNotSet {
            variable_name: "DDNS_RUNNER_TOKEN".to_string(),
        };
        assert_eq!(error, expected_error);
    }

    #[test]
    fn should_exit_with_error_when_address_is_missing() {
        let _cleaner = EnvVariableTest::new();
        std::env::set_var("DDNS_RUNNER_DOMAINS_TO_UPDATE", "phdwebsite");
        std::env::set_var("DDNS_RUNNER_TOKEN", "1234");
        let error = read_config_with_default().unwrap_err();
        let expected_error = ConfigReadingError::VariableNotSet {
            variable_name: "DDNS_RUNNER_DUCK_DNS_ADDRESS".to_string(),
        };
        assert_eq!(error, expected_error);
    }

    #[test]
    fn should_exit_with_error_when_database_uri_is_missing() {
        let _cleaner = EnvVariableTest::new();
        std::env::set_var("DDNS_RUNNER_DOMAINS_TO_UPDATE", "phdwebsite");
        std::env::set_var("DDNS_RUNNER_TOKEN", "1234");
        std::env::set_var("DDNS_RUNNER_DUCK_DNS_ADDRESS", "https://duckdns.org");
        let error = read_config_with_default().unwrap_err();
        let expected_error = ConfigReadingError::VariableNotSet {
            variable_name: "DDNS_RUNNER_POSTGRES_URI".to_string(),
        };
        assert_eq!(error, expected_error);
    }

    #[test]
    fn should_exit_with_error_when_migration_files_directory_is_missing() {
        let _cleaner = EnvVariableTest::new();
        std::env::set_var("DDNS_RUNNER_DOMAINS_TO_UPDATE", "phdwebsite");
        std::env::set_var("DDNS_RUNNER_TOKEN", "1234");
        std::env::set_var("DDNS_RUNNER_DUCK_DNS_ADDRESS", "https://duckdns.org");
        std::env::set_var("DDNS_RUNNER_POSTGRES_URI", "postgres://localhost:5432/postgres");
        let error = read_config_with_default().unwrap_err();
        let expected_error = ConfigReadingError::VariableNotSet {
            variable_name: "DDNS_RUNNER_MIGRATION_FILES_DIRECTORY".to_string(),
        };
        assert_eq!(error, expected_error);
    }

    #[test]
    fn should_read_config_and_apply_defaults() {
        let _cleaner = EnvVariableTest::new();
        std::env::set_var("DDNS_RUNNER_DOMAINS_TO_UPDATE", "phdwebsite;someotherdomain");
        std::env::set_var("DDNS_RUNNER_TOKEN", "1234");
        std::env::set_var("DDNS_RUNNER_DUCK_DNS_ADDRESS", "https://duckdns.org");
        std::env::set_var("DDNS_RUNNER_POSTGRES_URI", "postgres://localhost:5432/postgres");
        std::env::set_var("DDNS_RUNNER_MIGRATION_FILES_DIRECTORY", "./migrations");
        let result = read_config_with_default().unwrap();
        assert_eq!(result.domains_to_update, vec!["phdwebsite", "someotherdomain"]);
        assert_eq!(result.token, "1234");
        assert_eq!(result.duck_dns_address, "https://duckdns.org");
        assert_eq!(result.server_port, 3000);
        assert_eq!(result.ip_update_interval, Duration::from_secs(5 * 60));
    }

    #[test]
    fn should_read_config_and_override_defaults() {
        let _cleaner = EnvVariableTest::new();
        std::env::set_var("DDNS_RUNNER_DOMAINS_TO_UPDATE", "phdwebsite");
        std::env::set_var("DDNS_RUNNER_TOKEN", "1234");
        std::env::set_var("DDNS_RUNNER_DUCK_DNS_ADDRESS", "https://duckdns.org");
        std::env::set_var("DDNS_RUNNER_POSTGRES_URI", "postgres://localhost:5432/postgres");
        std::env::set_var("DDNS_RUNNER_IP_UPDATE_INTERVAL_SEC", "60");
        std::env::set_var("DDNS_RUNNER_SERVER_PORT", "4000");
        std::env::set_var("DDNS_RUNNER_MIGRATION_FILES_DIRECTORY", "./migrations");
        let result = read_config_with_default().unwrap();
        assert_eq!(result.server_port, 4000);
        assert_eq!(result.ip_update_interval, Duration::from_secs(60));
    }
}
