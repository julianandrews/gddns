mod config;
mod ddns;
mod response_cache;

use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use config::{Command, Config};
use response_cache::ResponseCache;

static DEFAULT_CACHE_DIR: &str = concat!("/var/cache/", env!("CARGO_PKG_NAME"));

#[tokio::main]
async fn main() -> Result<()> {
    let args = config::Args::parse();
    match args.command {
        None => update_from_config(args.config_file, args.cache_dir, args.ip).await,
        Some(Command::Daemon(comm_args)) => {
            run_daemon(
                comm_args.config_file,
                args.cache_dir,
                comm_args.poll_interval,
            )
            .await
        }
        Some(Command::UpdateHost(comm_args)) => {
            let ip = match comm_args.ip {
                Some(ip) => ip,
                None => public_ip::addr().await.context("Failed to get public IP")?,
            };
            let mut response_cache = match args.cache_dir {
                Some(dir) => ResponseCache::new(dir),
                None => ResponseCache::new(DEFAULT_CACHE_DIR),
            };
            update_host(
                &comm_args.hostname,
                &comm_args.client_config,
                &mut response_cache,
                ip,
            )
            .await
        }
        Some(Command::ClearCache(comm_args)) => clear_cache(&comm_args.hostname, args.cache_dir),
    }
}

async fn run_daemon(
    config_file: PathBuf,
    cache_dir: Option<PathBuf>,
    poll_interval: Option<u64>,
) -> Result<()> {
    let config = config::load(&config_file).context("Failed to load config")?;
    let mut response_cache = ResponseCache::new(
        cache_dir
            .or_else(|| config.cache_dir.clone())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CACHE_DIR)),
    );
    let poll_interval = std::time::Duration::from_secs(
        poll_interval.or(config.daemon_poll_interval).unwrap_or(300),
    );
    loop {
        match public_ip::addr().await {
            Some(ip) => {
                if let Err(error) = update_all(&config, &mut response_cache, ip).await {
                    eprintln!("{}", error);
                }
            }
            None => eprintln!("Failed to get public IP address"),
        }
        std::thread::sleep(poll_interval);
    }
}

async fn update_from_config(
    config_file: PathBuf,
    cache_dir: Option<PathBuf>,
    ip: Option<IpAddr>,
) -> Result<()> {
    let config = config::load(&config_file).context("Failed to load config")?;
    let mut response_cache = ResponseCache::new(
        cache_dir
            .or_else(|| config.cache_dir.clone())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CACHE_DIR)),
    );
    let ip = match ip {
        Some(ip) => ip,
        None => public_ip::addr().await.context("Failed to get public IP")?,
    };
    update_all(&config, &mut response_cache, ip).await
}

async fn update_all<'cache, 'config: 'cache>(
    config: &'config Config,
    response_cache: &mut ResponseCache<'cache>,
    ip: IpAddr,
) -> Result<()> {
    let mut update_failed = false;
    for (hostname, client_config) in &config.hosts {
        if let Err(error) = update_host(hostname, client_config, response_cache, ip).await {
            update_failed = true;
            eprintln!("Failed to update {}:\n  {}", hostname, error);
        }
    }
    if update_failed {
        Err(anyhow::anyhow!("Failed to update at least one host"))
    } else {
        Ok(())
    }
}

async fn update_host<'cache, 'hostname: 'cache>(
    hostname: &'hostname str,
    client_config: &config::ClientConfig,
    response_cache: &mut ResponseCache<'cache>,
    ip: IpAddr,
) -> Result<()> {
    let cache_entry = response_cache
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
            let age = std::time::SystemTime::now().duration_since(*mtime)?;
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
    if old_ip == Some(&ip) {
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
    response_cache
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
    let cache_dir = cache_dir.unwrap_or_else(|| PathBuf::from(DEFAULT_CACHE_DIR));
    let mut cache = ResponseCache::new(cache_dir);
    cache
        .clear(hostname)
        .with_context(|| format!("Failed to clear cache for {}", hostname))?;
    Ok(())
}
