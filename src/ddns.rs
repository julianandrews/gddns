use std::net::IpAddr;

use anyhow::anyhow;

use crate::config::ClientConfig;

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug, Clone)]
pub struct Client {
    auth: Auth,
    update_url: String,
}

#[derive(Debug, Clone)]
enum Auth {
    Password(PasswordAuth),
    Token(String),
}

#[derive(Debug, Clone)]
struct PasswordAuth {
    username: String,
    password: String,
}

impl Client {
    /// Updates the DNS for a host.
    pub async fn update(&self, hostname: &str, ip: IpAddr) -> DdnsResult {
        let client = match reqwest::Client::builder().user_agent(USER_AGENT).build() {
            Ok(client) => client,
            Err(e) => return DdnsResult::FatalError("requesterror".to_string(), e.to_string()),
        };
        let mut request = client
            .get(&self.update_url)
            .query(&[("hostname", hostname), ("myip", &ip.to_string())]);
        request = match &self.auth {
            Auth::Password(auth) => request.basic_auth(&auth.username, Some(&auth.password)),
            Auth::Token(token) => request.header("Authorization", format!("Token {}", token)),
        };

        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => return DdnsResult::FatalError("requesterror".to_string(), e.to_string()),
        };
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_else(|_| "".to_string());
            if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return DdnsResult::RetryableError("retryable".to_string(), text);
            } else {
                return DdnsResult::FatalError("clienterror".to_string(), text);
            }
        }
        let text = match response.text().await {
            Ok(text) => text,
            Err(e) => return DdnsResult::FatalError("requesterror".to_string(), e.to_string()),
        };
        // deSEC doesn't return the IP address with "good" and "nochg" responses. Add it in.
        let text = match text.as_str() {
            "good" | "nochg" => format!("{} {}", text, ip),
            _ => text,
        };
        text.parse::<DdnsResult>()
            .unwrap_or_else(|e| DdnsResult::FatalError("parseerror".to_string(), e.to_string()))
    }
}

impl std::convert::From<&ClientConfig> for Client {
    fn from(config: &ClientConfig) -> Self {
        let auth = match &config.username {
            Some(username) => Auth::Password(PasswordAuth {
                username: username.to_string(),
                password: config.password.clone().unwrap(),
            }),
            None => Auth::Token(config.token.clone().unwrap()),
        };
        Client {
            auth,
            update_url: config.dyndns_url.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DdnsResult {
    Good(IpAddr),
    NoChg(IpAddr),
    FatalError(String, String),
    RetryableError(String, String),
}

impl std::str::FromStr for DdnsResult {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (code, rest) = s
            .split_once(' ')
            .ok_or_else(|| anyhow!("Invalid response from DDNS server: {}", s))?;
        match code {
            "good" => Ok(Self::Good(rest.parse()?)),
            "nochg" => Ok(Self::NoChg(rest.parse()?)),
            "nohost" | "badauth" | "notfqdn" | "badagent" | "!donator" | "conflict" | "abuse"
            | "clienterror" => Ok(Self::FatalError(code.to_string(), rest.to_string())),
            "dnserr" | "911" | "retryable" => {
                Ok(Self::RetryableError(code.to_string(), rest.to_string()))
            }
            _ => Err(anyhow!("Invalid response from DDNS server: {}.", s)),
        }
    }
}

impl std::fmt::Display for DdnsResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Good(ip) => write!(f, "good {}", ip),
            Self::NoChg(ip) => write!(f, "nochg {}", ip),
            Self::FatalError(code, s) => write!(f, "{} {}", code, s),
            Self::RetryableError(code, s) => write!(f, "{} {}", code, s),
        }
    }
}
