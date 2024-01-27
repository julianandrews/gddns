use std::collections::HashMap;

use clap::{AppSettings, Parser};
use serde::{de::Error, Deserialize, Deserializer};

pub fn load(config_file: &std::path::Path) -> anyhow::Result<Config> {
    let config = toml::from_str(&std::fs::read_to_string(config_file)?)?;
    Ok(config)
}

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None, setting=AppSettings::DeriveDisplayOrder)]
#[clap(global_setting(AppSettings::ArgsNegateSubcommands))]
pub struct Args {
    /// Path to IP cache directory
    #[clap(long, global(true))]
    pub cache_dir: Option<std::path::PathBuf>,

    /// Path to config file
    #[clap(long, default_value = "/etc/gddns/config.toml")]
    pub config_file: std::path::PathBuf,

    /// IP address override
    #[clap(long)]
    pub ip: Option<std::net::IpAddr>,

    #[clap(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand, Debug, Clone)]
#[clap(setting = AppSettings::ColoredHelp)]
pub enum Command {
    /// Launch as a long running daemon
    Daemon(DaemonArgs),

    /// Update a specific host providing arguments from the command line
    UpdateHost(HostArgs),

    /// Clear the IP cache for a host
    ClearCache(ClearCacheArgs),
}

#[derive(Parser, Debug, Clone)]
#[clap(setting = AppSettings::DeriveDisplayOrder)]
pub struct DaemonArgs {
    /// Path to config file
    #[clap(long, default_value = "/etc/gddns/config.toml")]
    pub config_file: std::path::PathBuf,

    /// Polling interval in seconds
    #[clap(short, long)]
    pub poll_interval: Option<u64>,
}

#[derive(Parser, Debug, Clone)]
#[clap(setting = AppSettings::DeriveDisplayOrder)]
pub struct HostArgs {
    /// Hostname to update
    #[clap()]
    pub hostname: String,

    #[clap(flatten)]
    pub client_config: ClientConfig,

    /// IP address override
    #[clap(long)]
    pub ip: Option<std::net::IpAddr>,
}

#[derive(Parser, Debug, Clone)]
#[clap(setting = AppSettings::DeriveDisplayOrder)]
pub struct ClearCacheArgs {
    /// Hostname to remove from the cache
    #[clap()]
    pub hostname: String,
}

#[derive(Parser, Debug, Clone)]
#[clap(group = clap::ArgGroup::new("auth").multiple(false))]
pub struct ClientConfig {
    /// URL for the dynamic DNS update API
    #[clap(short, long)]
    pub dyndns_url: String,

    /// Username for Dynamic DNS service
    #[clap(short, long)]
    #[clap(group = "auth", conflicts_with = "token", requires = "password")]
    pub username: Option<String>,

    /// Password or access key for Dynamic DNS service
    #[clap(short, long)]
    #[clap(conflicts_with = "token", requires = "username")]
    pub password: Option<String>,

    /// Token for Dynamic DNS service authentication
    #[clap(short, long)]
    #[clap(group = "auth", conflicts_with = "token", requires = "username")]
    pub token: Option<String>,

    /// Server error retry backoff time in minutes
    #[clap(long, default_value = "5")]
    pub server_backoff: u64,
}

// An enum for auth would work great for Serde, but Clap doesn't support that yet.
// See https://github.com/clap-rs/clap/issues/2621.
// For now, a custom deserializer seems like the cleanest way to solve this.
impl<'de> Deserialize<'de> for ClientConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        fn default_server_backoff() -> u64 {
            5
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case", deny_unknown_fields)]
        struct ClientConfigUnchecked {
            dyndns_url: String,
            username: Option<String>,
            password: Option<String>,
            token: Option<String>,
            #[serde(default = "default_server_backoff")]
            server_backoff: u64,
        }

        let config = ClientConfigUnchecked::deserialize(deserializer)?;
        match (&config.token, &config.username, &config.password) {
            (None, None, None) => Err(D::Error::custom("missing authentication")),
            (None, Some(_), None) => Err(D::Error::custom(
                "missing password for password authentication",
            )),
            (None, None, Some(_)) => Err(D::Error::custom(
                "missing username for password authentication",
            )),
            (Some(_), Some(_), _) | (Some(_), _, Some(_)) => Err(D::Error::custom(
                "multiple forms of authentication specified",
            )),
            _ => Ok(ClientConfig {
                dyndns_url: config.dyndns_url,
                username: config.username,
                password: config.password,
                token: config.token,
                server_backoff: config.server_backoff,
            }),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub cache_dir: Option<std::path::PathBuf>,
    pub daemon_poll_interval: Option<u64>,
    pub hosts: HashMap<String, ClientConfig>,
}
