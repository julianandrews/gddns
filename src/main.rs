mod config;
mod ddns;
mod response_cache;
mod update;

use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use config::Command;
use response_cache::ResponseCache;
use update::{update_all, update_host};

static DEFAULT_CACHE_DIR: &str = concat!("/var/cache/", env!("CARGO_PKG_NAME"));

#[tokio::main]
async fn main() -> std::result::Result<(), ()> {
    let args = config::Args::parse();
    let result = match args.command {
        None => update_from_config(args.config_file, args.cache_dir, args.ip).await,
        Some(Command::UpdateHost(comm_args)) => {
            update_from_args(
                comm_args.ip,
                args.cache_dir,
                &comm_args.hostname,
                &comm_args.client_config,
            )
            .await
        }
        Some(Command::Daemon(comm_args)) => {
            run_daemon(
                comm_args.config_file,
                args.cache_dir,
                comm_args.poll_interval,
            )
            .await
        }
        Some(Command::ClearCache(comm_args)) => clear_cache(&comm_args.hostname, args.cache_dir),
    };
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{:#}", e);
            Err(())
        }
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
    )?;
    let ip = match ip {
        Some(ip) => ip,
        None => public_ip::addr().await.context("Failed to get public IP")?,
    };
    update_all(&config, &mut response_cache, ip).await
}

async fn update_from_args(
    ip: Option<IpAddr>,
    cache_dir: Option<PathBuf>,
    hostname: &str,
    client_config: &config::ClientConfig,
) -> Result<()> {
    let ip = match ip {
        Some(ip) => ip,
        None => public_ip::addr().await.context("Failed to get public IP")?,
    };
    let mut response_cache = match cache_dir {
        Some(dir) => ResponseCache::new(dir),
        None => ResponseCache::new(DEFAULT_CACHE_DIR),
    }?;
    update_host(hostname, client_config, &mut response_cache, ip).await
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
    )?;
    let poll_interval = std::time::Duration::from_secs(
        poll_interval.or(config.daemon_poll_interval).unwrap_or(300),
    );
    loop {
        response_cache.check_disk_changes()?;
        match public_ip::addr().await {
            Some(ip) => {
                if let Err(error) = update_all(&config, &mut response_cache, ip).await {
                    eprintln!("{:#}", error);
                }
            }
            None => eprintln!("Failed to get public IP address."),
        }
        std::thread::sleep(poll_interval);
    }
}

fn clear_cache(hostname: &str, cache_dir: Option<PathBuf>) -> Result<()> {
    let cache_dir = cache_dir.unwrap_or_else(|| PathBuf::from(DEFAULT_CACHE_DIR));
    let mut cache = ResponseCache::new(cache_dir)?;
    cache
        .clear(hostname)
        .with_context(|| format!("Failed to clear cache for {}", hostname))?;
    Ok(())
}
