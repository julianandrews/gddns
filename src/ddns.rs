use std::net::IpAddr;

static URL: &str = "https://domains.google.com/nic/update";

#[derive(Debug)]
pub struct Client {
    username: String,
    password: String,
}

impl Client {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    /// Update the DNS for `domain` to `ip`.
    ///
    /// Returns Ok(true) if update is succesful, and Ok(false) if the DNS was already correct.
    pub fn update(&self, domain: &str, ip: IpAddr) -> Result<bool, Box<dyn std::error::Error>> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(super::USER_AGENT)
            .build()?;
        let response = client
            .get(URL)
            .basic_auth(&self.username, Some(&self.password))
            .query(&[("hostname", domain), ("myip", &ip.to_string())])
            .send()?;
        let ddns_response: Response = response.error_for_status()?.text()?.parse()?;
        match ddns_response {
            Response::Good(_) => Ok(true),
            Response::NoChg(_) => Ok(false),
            _ => Err(Box::new(Error::Error(ddns_response))),
        }
    }
}

#[derive(Debug)]
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
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut words = s.split(' ');
        let kind = words.next().unwrap_or("");
        let ip = words.next().unwrap_or("");
        match kind {
            "good" => Ok(Self::Good(ip.parse()?)),
            "nochg" => Ok(Self::NoChg(ip.parse()?)),
            "nohost" => Ok(Self::NoHost),
            "badauth" => Ok(Self::BadAuth),
            "notfqdn" => Ok(Self::NotFqdn),
            "badagent" => Ok(Self::BadAgent),
            "abuse" => Ok(Self::Abuse),
            "911" => Ok(Self::Error),
            "conflict A" => Ok(Self::ConflictA),
            "conflict AAAA" => Ok(Self::ConflictAAAA),
            _ => Err(Box::new(Error::InvalidResponse)),
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

#[derive(Debug)]
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
