use anyhow::Result;
use std::collections::{btree_map, BTreeMap};
use std::time::SystemTime;

use super::ddns::Response;

/// Filesystem backed cache of past runs used to prevent repeated requests to the DDNS server.
///
/// The cache consists of a base directory containing one file per hostname. The file must be named
/// after the hostname and contains the text of the DDNS response.
#[derive(Debug, Clone)]
pub struct ResponseCache<'a> {
    dir: std::path::PathBuf,
    cache: BTreeMap<&'a str, (Response, SystemTime)>,
}

impl<'a> ResponseCache<'a> {
    pub fn new<P: Into<std::path::PathBuf>>(dir: P) -> Self {
        Self {
            dir: dir.into(),
            cache: BTreeMap::new(),
        }
    }

    /// Gets the response for the last succesful run for a host.
    ///
    /// This function will return `None` if no cache file is found.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to read the cache file or if the cache file
    /// exists but does not contain a valid response.
    pub fn get<'b: 'a>(&mut self, hostname: &'b str) -> Result<Option<&(Response, SystemTime)>> {
        match self.cache.entry(hostname) {
            btree_map::Entry::Occupied(entry) => Ok(Some(entry.into_mut())),
            btree_map::Entry::Vacant(entry) => {
                let cache_file = self.dir.join(hostname);
                let data = match std::fs::read(&cache_file) {
                    Ok(data) => data,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                    Err(e) => Err(e)?,
                };
                let response = String::from_utf8_lossy(&data).parse()?;
                let mtime = std::fs::metadata(&cache_file)?.modified()?;
                Ok(Some(entry.insert((response, mtime))))
            }
        }
    }

    /// Updates the IP address cache for a host.
    ///
    /// This function will create the cache directory if it does not exist, and will overwrite any
    /// existing entry.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to create the cache directory or write the
    /// cache file.
    pub fn put<'b: 'a>(&mut self, hostname: &'b str, response: &Response) -> Result<()> {
        if let Some((cached_response, _mtime)) = self.cache.get(hostname) {
            if cached_response == response {
                return Ok(());
            }
        }
        self.cache
            .insert(hostname, (response.clone(), SystemTime::now()));
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(self.dir.join(hostname), response.to_string())?;
        Ok(())
    }

    /// Cleares the IP address cache for a host.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to remove the cache file.
    pub fn clear(&mut self, hostname: &str) -> Result<()> {
        self.cache.remove(hostname);
        std::fs::remove_file(self.dir.join(hostname))?;
        Ok(())
    }
}
