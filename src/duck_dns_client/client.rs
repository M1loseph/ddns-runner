use super::errors::{DuckDnsMessageParsingError, StatusCodeError, UpdateIpError};
use log::{trace, warn};
use reqwest::StatusCode;
use std::{
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
    time::Duration,
};

pub struct DuckDnsConfig {
    pub duck_dns_address: String,
    pub token: String,
}

pub struct DuckDnsClient {
    config: DuckDnsConfig,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ServerAction {
    Updated,
    NoChange,
}

#[derive(Debug)]
pub struct IPUpdateResult {
    pub current_ip_v4: Ipv4Addr,
    pub current_ip_v6: Option<Ipv6Addr>,
    pub server_action: ServerAction,
}

impl DuckDnsClient {
    pub fn new(config: DuckDnsConfig) -> Self {
        DuckDnsClient { config }
    }

    pub async fn update_ip(&self, domain_to_update: &str) -> Result<IPUpdateResult, UpdateIpError> {
        let url = format!(
            "{}/update?domains={}&token={}&verbose=true",
            self.config.duck_dns_address, domain_to_update, self.config.token
        );
        trace!("Sending request to: {url}");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        let response = client.get(url).send().await?;
        if response.status() != StatusCode::OK {
            warn!("Received {} status from DuckDNS", response.status());
            let err = StatusCodeError::new(response.status().as_u16());
            return Err(UpdateIpError::from(err));
        }
        let response_text = response.text().await?;
        let parsed_response = IPUpdateResult::try_from(response_text)?;
        Ok(parsed_response)
    }
}

impl TryFrom<String> for IPUpdateResult {
    type Error = DuckDnsMessageParsingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(DuckDnsMessageParsingError::EmptyResponse);
        }
        let mut ip_v4: Option<Ipv4Addr> = None;
        let mut ip_v6: Option<Ipv6Addr> = None;
        let mut server_action: Option<ServerAction> = None;
        for (index, line) in value.lines().enumerate() {
            match index {
                0 => match line {
                    "OK" => continue,
                    "KO" => return Err(DuckDnsMessageParsingError::KoResponse),
                    _ => return Err(DuckDnsMessageParsingError::UnknownResponse),
                },
                1 => {
                    let parsed_ip_v4 = Ipv4Addr::from_str(line)
                        .map_err(|_| DuckDnsMessageParsingError::IncorrectIpV4)?;
                    ip_v4 = Some(parsed_ip_v4);
                }
                2 => {
                    if line.is_empty() {
                        continue;
                    }
                    let parsed_ip_v6 = Ipv6Addr::from_str(line)
                        .map_err(|_| DuckDnsMessageParsingError::IncorrectIpV6)?;
                    ip_v6 = Some(parsed_ip_v6);
                }
                3 => match line {
                    "UPDATED" => server_action = Some(ServerAction::Updated),
                    "NOCHANGE" => server_action = Some(ServerAction::NoChange),
                    "" => return Err(DuckDnsMessageParsingError::MissingServerAction),
                    _ => return Err(DuckDnsMessageParsingError::UnknownServerAction),
                },
                _ => {
                    return Err(DuckDnsMessageParsingError::TooManyLines);
                }
            }
        }
        if ip_v4.is_none() {
            return Err(DuckDnsMessageParsingError::IncorrectIpV4);
        }
        if server_action.is_none() {
            return Err(DuckDnsMessageParsingError::MissingServerAction);
        }
        Ok(IPUpdateResult {
            current_ip_v4: ip_v4.unwrap(),
            current_ip_v6: ip_v6,
            server_action: server_action.unwrap(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_duck_dns_response_without_ipv6() {
        let response = "OK\n\
        127.0.0.1\n\
        \n\
        NOCHANGE\
        "
        .to_string();
        let parsed_response = IPUpdateResult::try_from(response).unwrap();

        assert_eq!(parsed_response.current_ip_v4, Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(parsed_response.current_ip_v6, None);
        assert_eq!(parsed_response.server_action, ServerAction::NoChange);
    }

    #[test]
    fn should_parse_duck_dns_response_with_ipv6() {
        let response = "OK\n\
        127.0.0.1\n\
        2001:db8:3333:4444:5555:6666:7777:8888\n\
        NOCHANGE\
        "
        .to_string();
        let parsed_response = IPUpdateResult::try_from(response).unwrap();

        assert_eq!(parsed_response.current_ip_v4, Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(
            parsed_response.current_ip_v6,
            Ipv6Addr::from_str("2001:db8:3333:4444:5555:6666:7777:8888").ok()
        );
        assert_eq!(parsed_response.server_action, ServerAction::NoChange);
    }

    #[test]
    fn should_return_ko_error() {
        let response = "KO".to_string();
        let parsing_error = IPUpdateResult::try_from(response).unwrap_err();

        assert_eq!(parsing_error, DuckDnsMessageParsingError::KoResponse);
    }

    #[test]
    fn should_return_incorrect_ip_v4_error() {
        let response = "OK\n256.0.0.1".to_string();
        let parsing_error = IPUpdateResult::try_from(response).unwrap_err();

        assert_eq!(parsing_error, DuckDnsMessageParsingError::IncorrectIpV4);
    }

    #[test]
    fn should_return_incorrect_ip_v6_error() {
        let response = "OK\n255.0.0.0\ndonald the duck".to_string();
        let parsing_error = IPUpdateResult::try_from(response).unwrap_err();

        assert_eq!(parsing_error, DuckDnsMessageParsingError::IncorrectIpV6);
    }

    #[test]
    fn should_return_empty_server_action_error() {
        let response = "OK\n255.0.0.0\n2001:db8:3333:4444:5555:6666:7777:8888".to_string();
        let parsing_error = IPUpdateResult::try_from(response).unwrap_err();

        assert_eq!(
            parsing_error,
            DuckDnsMessageParsingError::MissingServerAction
        );
    }

    #[test]
    fn should_return_incorrect_server_action_error() {
        let response = "OK\n255.0.0.0\n2001:db8:3333:4444:5555:6666:7777:8888\ndonald".to_string();
        let parsing_error = IPUpdateResult::try_from(response).unwrap_err();

        assert_eq!(
            parsing_error,
            DuckDnsMessageParsingError::UnknownServerAction
        );
    }

    #[test]
    fn should_return_too_many_lines_error() {
        let response =
            "OK\n255.0.0.0\n2001:db8:3333:4444:5555:6666:7777:8888\nUPDATED\nhello world"
                .to_string();
        let parsing_error = IPUpdateResult::try_from(response).unwrap_err();

        assert_eq!(parsing_error, DuckDnsMessageParsingError::TooManyLines);
    }
}
