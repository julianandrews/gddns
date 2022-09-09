use std::net::IpAddr;

use anyhow::Result;

static URL: &str = "https://domains.google.com/nic/update";

#[derive(Debug, Clone)]
pub struct Client {
    username: String,
    password: String,
}

impl Client {
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    /// Updates the DNS for a host.
    ///
    /// Returns Ok(true) if update is succesful, and Ok(false) if the DNS was already correct.
    pub fn update(&self, hostname: &str, ip: IpAddr) -> Result<bool> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(super::USER_AGENT)
            .build()?;
        let response = client
            .get(URL)
            .basic_auth(&self.username, Some(&self.password))
            .query(&[("hostname", hostname), ("myip", &ip.to_string())])
            .send()?;
        let ddns_response: Response = response.error_for_status()?.text()?.parse()?;
        match ddns_response {
            Response::Good(_) => Ok(true),
            Response::NoChg(_) => Ok(false),
            _ => Err(Error::Error(ddns_response))?,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Response {
    Good(IpAddr),
    NoChg(IpAddr),
    NoHost,
    BadAuth,
    NotFqdn,
    BadAgent,
    Abuse,
    Error,
    ConflictA,
    ConflictAAAA,
}

impl std::str::FromStr for Response {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut words = s.split(' ');
        let kind = words.next().unwrap_or("");
        let ip = words.next().unwrap_or("");
        match kind {
            "good" => Ok(Self::Good(ip.parse().map_err(|_| Error::InvalidResponse)?)),
            "nochg" => Ok(Self::NoChg(ip.parse().map_err(|_| Error::InvalidResponse)?)),
            "nohost" => Ok(Self::NoHost),
            "badauth" => Ok(Self::BadAuth),
            "notfqdn" => Ok(Self::NotFqdn),
            "badagent" => Ok(Self::BadAgent),
            "abuse" => Ok(Self::Abuse),
            "911" => Ok(Self::Error),
            "conflict A" => Ok(Self::ConflictA),
            "conflict AAAA" => Ok(Self::ConflictAAAA),
            _ => Err(Error::InvalidResponse),
        }
    }
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Good(ip) => write!(f, "IP updated ({})", ip),
            Self::NoChg(ip) => write!(f, "IP unchanged ({})", ip),
            Self::NoHost => write!(f, "Hostname not registered with account"),
            Self::BadAuth => write!(f, "Authentication failed"),
            Self::NotFqdn => write!(f, "Invalid hostname"),
            Self::BadAgent => write!(f, "User agent not set"),
            Self::Abuse => write!(f, "Request blocked by abuse policy"),
            Self::Error => write!(f, "Server error, wait 5 minutes and retry"),
            Self::ConflictA => write!(f, "Conflict with custom A resource record"),
            Self::ConflictAAAA => write!(f, "Conflict with custom AAAA resource record"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Error {
    InvalidResponse,
    Error(Response),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidResponse => write!(f, "Invalid response from DDNS server"),
            Self::Error(response) => write!(f, "{}", response),
        }
    }
}

impl std::error::Error for Error {}
