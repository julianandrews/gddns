use std::net::IpAddr;

use anyhow::Result;

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug, Clone)]
pub struct Client {
    username: String,
    password: String,
    update_url: String,
}

impl Client {
    pub fn new(username: &str, password: &str, update_url: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
            update_url: update_url.to_string(),
        }
    }

    /// Updates the DNS for a host.
    pub async fn update(&self, hostname: &str, ip: IpAddr) -> Result<Response> {
        let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
        let response = client
            .get(&self.update_url)
            .basic_auth(&self.username, Some(&self.password))
            .query(&[("hostname", hostname), ("myip", &ip.to_string())])
            .send()
            .await?;
        let ddns_response: Response = response.error_for_status()?.text().await?.trim().parse()?;
        Ok(ddns_response)
    }
}

#[derive(Debug, Clone)]
pub enum Response {
    Good(IpAddr),
    NoChg(IpAddr),
    UserError(String),
    ServerError(String),
}

impl std::str::FromStr for Response {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let code = s.split(' ').next().unwrap_or("");
        let last_word = s.rsplit(' ').next().unwrap_or("");
        match code {
            "good" => {
                let ip = last_word
                    .parse()
                    .map_err(|_| Error::InvalidResponse(s.to_string()))?;
                Ok(Self::Good(ip))
            }
            "nochg" => {
                let ip = last_word
                    .parse()
                    .map_err(|_| Error::InvalidResponse(s.to_string()))?;
                Ok(Self::NoChg(ip))
            }
            "nohost" | "badauth" | "notfqdn" | "badagent" | "!donator" | "conflict" | "abuse" => {
                Ok(Self::UserError(s.to_string()))
            }
            "dnserr" | "911" => Ok(Self::ServerError(s.to_string())),
            _ => Err(Error::InvalidResponse(s.to_string())),
        }
    }
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Good(ip) => write!(f, "good {}", ip),
            Self::NoChg(ip) => write!(f, "nochg {}", ip),
            Self::UserError(s) => write!(f, "{}", s),
            Self::ServerError(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    InvalidResponse(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidResponse(s) => write!(f, "Invalid response from DDNS server:\n\n{}", s),
        }
    }
}

impl std::error::Error for Error {}
