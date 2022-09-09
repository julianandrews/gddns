use std::net::IpAddr;

use anyhow::Result;

/// Cache of past runs used to prevent repeated requests to the DDNS server.
///
/// The cache consists of a base directory containing one file per hostname. The file must be named
/// after the hostname and contain an IPv4 address in dotted decimal notation.
#[derive(Debug, Clone)]
pub struct IpCache {
    dir: std::path::PathBuf,
}

impl IpCache {
    pub fn new<P: Into<std::path::PathBuf>>(dir: P) -> Self {
        Self { dir: dir.into() }
    }

    /// Gets the IP for the last succesful run for a host.
    ///
    /// This function will return `None` if no cache file is found.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to read the cache file or if the cache file
    /// exists but does not contain a valid IPv4 address.
    pub fn get(&self, hostname: &str) -> Result<Option<IpAddr>> {
        let data = match std::fs::read(self.cache_file(hostname)) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => Err(e)?,
        };
        let ip = String::from_utf8_lossy(&data).parse()?;
        Ok(Some(ip))
    }

    /// Updates the IP address cache for a host.
    ///
    /// This function will create the cache directory if it does not exist, and will overwrite any
    /// existing
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to create the cache directory or write the
    /// cache file.
    pub fn put(&self, hostname: &str, ip: IpAddr) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(self.cache_file(hostname), ip.to_string())?;
        Ok(())
    }

    /// Cleares the IP address cache for a host.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to remove the cache file.
    pub fn clear(&self, hostname: &str) -> Result<()> {
        std::fs::remove_file(self.cache_file(hostname))?;
        Ok(())
    }

    fn cache_file(&self, hostname: &str) -> std::path::PathBuf {
        self.dir.join(hostname)
    }
}
