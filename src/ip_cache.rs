use std::io::Read;
use std::net::IpAddr;

#[derive(Debug)]
pub struct IpCache {
    dir: std::path::PathBuf,
}

impl IpCache {
    pub fn new(dir: std::path::PathBuf) -> Self {
        Self { dir }
    }

    pub fn get(&self, domain: &str) -> Result<Option<IpAddr>, Box<dyn std::error::Error>> {
        let cache_file = self.dir.join(std::path::PathBuf::from(domain));
        let mut file = match std::fs::File::open(cache_file) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(Box::new(e)),
        };
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let ip = contents.parse()?;
        Ok(Some(ip))
    }

    pub fn put(&self, domain: &str, ip: IpAddr) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.dir)?;
        let cache_file = self.dir.join(std::path::PathBuf::from(domain));
        std::fs::write(cache_file, ip.to_string())?;
        Ok(())
    }
}
