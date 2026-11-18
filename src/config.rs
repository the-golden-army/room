use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    pub data_dir: PathBuf,
    pub files_dir: PathBuf,
    pub server_name: String,
    pub max_upload: u64,
    pub page_size: i64,
    pub lan_ssid_hint: String,
}

impl Config {
    pub fn from_env() -> Self {
        let data_dir = env::var("ROOM_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data"));
        let files_dir = env::var("ROOM_FILES_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| data_dir.join("files"));
        let bind = env::var("ROOM_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
        let server_name = env::var("ROOM_NAME").unwrap_or_else(|_| "ROOM".into());
        let max_upload = env::var("ROOM_MAX_UPLOAD_MB")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(|mb| mb * 1024 * 1024)
            .unwrap_or(2 * 1024 * 1024 * 1024);
        let page_size = env::var("ROOM_PAGE_SIZE")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(60);
        let lan_ssid_hint = env::var("ROOM_NETWORK_NAME").unwrap_or_default();
        Config {
            bind,
            data_dir,
            files_dir,
            server_name,
            max_upload,
            page_size,
            lan_ssid_hint,
        }
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("room.db")
    }

    /// The port this server listens on, e.g. 8080.
    pub fn port(&self) -> Option<u16> {
        self.bind.rsplit_once(':')?.1.parse().ok()
    }

    /// A URL other devices on the same network can open.
    ///
    /// A UDP socket "connect" sends no packets; it only asks the OS which local
    /// address it would use to reach a peer. That is the LAN address of the
    /// machine running ROOM, which is exactly what a phone needs to type in.
    pub fn lan_url(&self) -> Option<String> {
        let port = self.port()?;
        let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
        sock.connect("8.8.8.8:80").ok()?;
        let ip = sock.local_addr().ok()?.ip();
        if ip.is_loopback() || ip.is_unspecified() {
            return None;
        }
        Some(format!("http://{}:{}", ip, port))
    }
}
