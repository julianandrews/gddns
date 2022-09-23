mod config;
mod ddns;
mod response_cache;

use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use config::Command;
use response_cache::ResponseCache;

static DEFAULT_CACHE_DIR: &str = concat!("/var/cache/", env!("CARGO_PKG_NAME"));

#[tokio::main]
async fn main() -> Result<()> {
    let args = config::Args::parse();
    match args.command {
        None => update_from_config(args.config_file, args.cache_dir, args.ip).await,
        Some(Command::UpdateHost(comm_args)) => {
            let ip = match comm_args.ip {
                Some(ip) => ip,
                None => public_ip::addr().await.context("Failed to get public IP")?,
            };
            update_host(
                &comm_args.hostname,
                &comm_args.client_config,
                ip,
                args.cache_dir,
            )
            .await
        }
        Some(Command::ClearCache(comm_args)) => clear_cache(&comm_args.hostname, args.cache_dir),
    }
}

async fn update_from_config(
    config_file: PathBuf,
    cache_dir: Option<PathBuf>,
    ip: Option<IpAddr>,
) -> Result<()> {
    let config = config::load(&config_file).context("Failed to load config")?;
    let cache_dir = cache_dir
        .or(config.cache_dir)
        .unwrap_or(PathBuf::from(DEFAULT_CACHE_DIR));
    let ip = match ip {
        Some(ip) => ip,
        None => public_ip::addr().await.context("Failed to get public IP")?,
    };
    let mut update_failed = false;
    for (hostname, client_config) in &config.hosts {
        if let Err(error) = update_host(hostname, client_config, ip, Some(&cache_dir)).await {
            update_failed = true;
            eprintln!("Failed to update {}:\n  {}", hostname, error);
        }
    }
    if update_failed {
        std::process::exit(1);
    }
    Ok(())
}

async fn update_host<P: Into<PathBuf>>(
    hostname: &str,
    client_config: &config::ClientConfig,
    ip: IpAddr,
    cache_dir: Option<P>,
) -> Result<()> {
    let cache = match cache_dir {
        Some(dir) => ResponseCache::new(dir),
        None => ResponseCache::new(DEFAULT_CACHE_DIR),
    };
    let cache_entry = cache
        .get(hostname)
        .context(format!("Failed to load cache for {}", hostname))?;
    let old_ip = match cache_entry {
        Some((ddns::Response::Good(ip), _)) => Some(ip),
        Some((ddns::Response::NoChg(ip), _)) => Some(ip),
        Some((ddns::Response::UserError(e), _)) => {
            return Err(anyhow::anyhow!(
                "User Error '{}' for {} on previous run. Fix the error and \
                clear the cache before running again.",
                e,
                hostname
            ))
        }
        Some((ddns::Response::ServerError(e), mtime)) => {
            let age = std::time::SystemTime::now().duration_since(mtime)?;
            let backoff_time = std::time::Duration::from_secs(client_config.server_backoff * 60);
            if age < backoff_time {
                let age_str = if age.as_secs() >= 120 {
                    format!("{} minutes", age.as_secs() / 60)
                } else {
                    format!("{} seconds", age.as_secs())
                };
                return Err(anyhow::anyhow!(
                    "Server Error '{}' {} ago, waiting {} minutes before retry. Clear the cache to reset.",
                    e,
                    age_str,
                    backoff_time.as_secs() / 60,
                ));
            } else {
                None
            }
        }
        None => None,
    };
    if old_ip == Some(ip) {
        println!("IP for {} already up to date ({})", hostname, ip);
        return Ok(());
    }

    println!("Updating IP for {} to {}", hostname, ip);
    let client = ddns::Client::new(
        &client_config.username,
        &client_config.password,
        &client_config.dyndns_url,
    );
    let response = client
        .update(hostname, ip)
        .await
        .with_context(|| format!("Failed to update DNS for {}", hostname))?;
    cache
        .put(hostname, &response)
        .with_context(|| format!("Failed to update cache for {}", hostname))?;
    match response {
        ddns::Response::Good(_) => println!("IP for {} updated", hostname),
        ddns::Response::NoChg(_) => println!("Warning: IP for {} unchanged", hostname),
        error_response => {
            return Err(anyhow::anyhow!("Failed up update DNS: {}", error_response));
        }
    }
    Ok(())
}

fn clear_cache(hostname: &str, cache_dir: Option<PathBuf>) -> Result<()> {
    let cache_dir = cache_dir.unwrap_or(PathBuf::from(DEFAULT_CACHE_DIR));
    let cache = ResponseCache::new(cache_dir);
    cache
        .clear(hostname)
        .with_context(|| format!("Failed to clear cache for {}", hostname))?;
    Ok(())
}
