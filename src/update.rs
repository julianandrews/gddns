use std::net::IpAddr;

use anyhow::{Context, Result};

use crate::config;
use crate::ddns;
use crate::response_cache::{ResponseCache, ResponseCacheError};

pub async fn update_host<'cache, 'hostname: 'cache>(
    hostname: &'hostname str,
    client_config: &config::ClientConfig,
    response_cache: &mut ResponseCache<'cache>,
    ip: IpAddr,
) -> Result<()> {
    let cache_entry = match response_cache.get(hostname) {
        Ok(entry) => entry,
        Err(ResponseCacheError::Parse(s)) => {
            eprintln!("Ignoring bad cache entry {}.", s);
            None
        }
        Err(e) => Err(e).context("Failed to load cache")?,
    };
    let old_ip = match cache_entry {
        Some((ddns::DdnsResult::Good(ip), _)) => Some(ip),
        Some((ddns::DdnsResult::NoChg(ip), _)) => Some(ip),
        Some((ddns::DdnsResult::FatalError(code, text), _)) => {
            return Err(anyhow::anyhow!(
                "Fatal Error on previous run: \"{} {}\". Fix the error and \
                clear the cache before running again.",
                code,
                text,
            ))
        }
        Some((ddns::DdnsResult::RetryableError(code, text), mtime)) => {
            let age = std::time::SystemTime::now().duration_since(*mtime)?;
            let backoff_time = std::time::Duration::from_secs(client_config.server_backoff * 60);
            if age < backoff_time {
                let age_str = if age.as_secs() >= 120 {
                    format!("{} minutes", age.as_secs() / 60)
                } else {
                    format!("{} seconds", age.as_secs())
                };
                return Err(anyhow::anyhow!(
                    "Server Error {} ago: \"{} {}\". Waiting {} minutes before retry.",
                    age_str,
                    code,
                    text,
                    backoff_time.as_secs() / 60,
                ));
            } else {
                None
            }
        }
        _ => None,
    };

    match old_ip {
        Some(old_ip) if old_ip == &ip => return Ok(()),
        Some(old_ip) => println!("Updating IP for {} from {} to {}.", hostname, old_ip, ip),
        None => println!("No cached value. Setting IP for {} to {}.", hostname, ip),
    }

    let client = ddns::Client::from(client_config);
    let response = client.update(hostname, ip).await;
    response_cache
        .put(hostname, &response)
        .context("Failed to update cache")?;
    match response {
        ddns::DdnsResult::Good(_) => println!("IP updated for {}.", hostname),
        ddns::DdnsResult::NoChg(_) => println!("Warning: IP unchanged for {}.", hostname),
        error_response => {
            return Err(anyhow::anyhow!("Failed up update DNS: {}", error_response));
        }
    }
    Ok(())
}

pub async fn update_all<'cache, 'config: 'cache>(
    config: &'config config::Config,
    response_cache: &mut ResponseCache<'cache>,
    ip: IpAddr,
) -> Result<()> {
    let mut errors = vec![];
    for (hostname, client_config) in &config.hosts {
        if let Err(e) = update_host(hostname, client_config, response_cache, ip).await {
            errors.push(e.context(format!("Failed to update {}", hostname)));
        }
    }
    if !errors.is_empty() {
        Err(UpdateErrors { errors })?
    }
    Ok(())
}

#[derive(Debug)]
struct UpdateErrors {
    errors: Vec<anyhow::Error>,
}

impl std::fmt::Display for UpdateErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, error) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{:#}", error)?;
        }
        Ok(())
    }
}

impl std::error::Error for UpdateErrors {}
