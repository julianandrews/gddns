mod ddns;
mod ip_cache;

use std::collections::HashMap;
use std::net::IpAddr;

use clap::Parser;

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

fn main() {
    let args = Args::parse();
    let ip = match get_ip() {
        Ok(ip) => ip,
        Err(e) => {
            eprintln!("Failed to get IP address: {}", e);
            std::process::exit(1);
        }
    };
    let ip_cache = ip_cache::IpCache::new(args.cache_dir);
    if !args.force {
        let old_ip = match ip_cache.get(&args.domain) {
            Ok(ip) => ip,
            Err(e) => {
                eprintln!("Failed to load IP cache: {}", e);
                std::process::exit(1);
            }
        };
        if old_ip == Some(ip) {
            println!("IP already up to date ({})", ip);
            std::process::exit(0);
        }
    }

    println!("Updating IP to {}", ip);
    let client = ddns::Client::new(args.username, args.password);
    match client.update(&args.domain, ip) {
        Err(e) => {
            eprintln!("Failed to update DNS: {}", e);
            std::process::exit(1);
        }
        Ok(false) => println!("Warning: IP unchanged"),
        _ => println!("IP updated"),
    }
    if let Err(e) = ip_cache.put(&args.domain, ip) {
        eprintln!("Failed to update cache: {}", e);
        std::process::exit(1);
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Domain to update
    #[clap()]
    domain: String,

    /// Google Domains Dynamic DNS credentials Username
    #[clap(short, long, value_name = "USERNAME")]
    username: String,

    /// Google Domains Dynamic DNS credentials Password
    #[clap(short, long, value_name = "PASSWORD")]
    password: String,

    /// Path to directory to use for IP cache
    #[clap(value_name = "FILE", default_value = "/var/cache/google-ddns")]
    cache_dir: std::path::PathBuf,

    /// Update DDNS even if cached value matches current address
    #[clap(short, long)]
    force: bool,
}

// fn update_domain(domain: &str, ip: IpAddr

fn get_ip() -> Result<IpAddr, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;
    let mut response = client.get("https://httpbin.org/ip").send()?;
    response = response.error_for_status()?;
    let data = response.json::<HashMap<String, String>>()?;
    let ip = data
        .get("origin")
        .ok_or(GoogleDdnsError::InvalidIpResponse)?
        .parse()?;

    Ok(ip)
}

#[derive(Debug)]
enum GoogleDdnsError {
    InvalidIpResponse,
}

impl std::fmt::Display for GoogleDdnsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIpResponse => write!(f, "Invalid response for public IP address."),
        }
    }
}

impl std::error::Error for GoogleDdnsError {}
