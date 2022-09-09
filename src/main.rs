mod config;
mod ddns;
mod ip_cache;

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use config::Command;
use ip_cache::IpCache;

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);
static DEFAULT_CACHE_DIR: &str = concat!("/var/cache/", env!("CARGO_PKG_NAME"));

fn main() -> Result<()> {
    let args = config::Args::parse();
    match args.command {
        None => update_from_config(args.config_file, args.cache_dir),
        Some(Command::UpdateHost(comm_args)) => {
            let ip = get_public_ip().context("Failed to get public IP")?;
            update_host(&comm_args.hostname, &comm_args.auth, ip, args.cache_dir)
        }
        Some(Command::ClearCache(comm_args)) => clear_cache(&comm_args.hostname, args.cache_dir),
    }
}

fn update_from_config(config_file: PathBuf, cache_dir: Option<PathBuf>) -> Result<()> {
    let config = config::load(&config_file).context("Failed to load config")?;
    let cache_dir = cache_dir
        .or(config.cache_dir)
        .unwrap_or(PathBuf::from(DEFAULT_CACHE_DIR));
    let ip = get_public_ip().context("Failed to get public IP")?;
    for (hostname, auth) in &config.hosts {
        update_host(hostname, auth, ip, Some(&cache_dir))?;
    }
    Ok(())
}

fn update_host<P: Into<PathBuf>>(
    hostname: &str,
    auth: &config::Auth,
    ip: IpAddr,
    cache_dir: Option<P>,
) -> Result<()> {
    let cache = match cache_dir {
        Some(dir) => IpCache::new(dir),
        None => IpCache::new(DEFAULT_CACHE_DIR),
    };
    let old_ip = cache
        .get(hostname)
        .context(format!("Failed to load cache for {}", hostname))?;
    if old_ip == Some(ip) {
        println!("IP for {} already up to date ({})", hostname, ip);
        return Ok(());
    }
    println!("Updating IP for {} to {}", hostname, ip);
    let client = ddns::Client::new(&auth.username, &auth.password);
    match !client
        .update(hostname, ip)
        .with_context(|| format!("Failed to update DNS for {}", hostname))?
    {
        true => println!("IP for {} updated", hostname),
        false => println!("Warning: IP for {} unchanged", hostname),
    }
    cache
        .put(hostname, ip)
        .with_context(|| format!("Failed to update cache for {}", hostname))?;
    Ok(())
}

fn clear_cache(hostname: &str, cache_dir: Option<PathBuf>) -> Result<()> {
    let cache_dir = cache_dir.unwrap_or(PathBuf::from(DEFAULT_CACHE_DIR));
    let cache = IpCache::new(cache_dir);
    cache
        .clear(hostname)
        .with_context(|| format!("Failed to clear cache for {}", hostname))?;
    Ok(())
}

fn get_public_ip() -> Result<IpAddr> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;
    let mut response = client.get("https://httpbin.org/ip").send()?;
    response = response.error_for_status()?;
    let data: HashMap<String, String> = response.json::<HashMap<String, String>>()?;
    let ip = data
        .get("origin")
        .ok_or(anyhow::anyhow!("Invalid data in response"))?
        .parse()?;

    Ok(ip)
}
