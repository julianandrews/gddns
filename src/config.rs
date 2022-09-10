use std::collections::HashMap;

use clap::{AppSettings, Parser};
use serde::Deserialize;

pub fn load(config_file: &std::path::Path) -> anyhow::Result<Config> {
    let config = toml::from_str(&std::fs::read_to_string(config_file)?)?;
    Ok(config)
}

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
#[clap(global_setting(AppSettings::ArgsNegateSubcommands))]
pub struct Args {
    /// Path to IP cache directory
    #[clap(long, global(true))]
    pub cache_dir: Option<std::path::PathBuf>,

    /// Path to config file
    #[clap(long, default_value = "/etc/gddns/config.toml")]
    pub config_file: std::path::PathBuf,

    #[clap(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand, Debug, Clone)]
#[clap(setting = AppSettings::ColoredHelp)]
pub enum Command {
    /// Update a specific host providing arguments from the command line
    UpdateHost(HostArgs),

    /// Clear the IP cache for a host
    ClearCache(ClearCacheArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct HostArgs {
    /// Hostname to update
    #[clap()]
    pub hostname: String,

    #[clap(flatten)]
    pub client_info: ClientInfo,
}

#[derive(Parser, Debug, Clone)]
pub struct ClearCacheArgs {
    /// Hostname to remove from the cache
    #[clap()]
    pub hostname: String,
}

#[derive(Deserialize, Parser, Debug, Clone)]
pub struct ClientInfo {
    /// Username for Dynamic DNS service
    #[clap(short, long)]
    pub username: String,

    /// Password or access key for Dynamic DNS service
    #[clap(short, long)]
    pub password: String,

    /// URL for the dynamic DNS update API
    #[clap(short, long)]
    pub dyndns_url: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub cache_dir: Option<std::path::PathBuf>,
    pub hosts: HashMap<String, ClientInfo>,
}
