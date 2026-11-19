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
    /// Host-issued code that lets the first device in, or gets the owner back in
    /// after losing every approved device. Empty = feature off.
    pub setup_code: String,
    /// Open server: anyone who can reach ROOM joins without waiting for
    /// approval. The host can still revoke a device afterwards.
    pub open: bool,
    /// Whether ROOM_OPEN was actually set in the environment. An operator who
    /// exports it has stated a policy and it must beat the stored value; if it
    /// is absent the stored value is the only opinion there is. Without this
    /// distinction `ROOM_OPEN=1` silently did nothing on a server that had
    /// already written `open = 0` on its first start.
    pub open_pinned: bool,
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
        let setup_code = env::var("ROOM_SETUP_CODE").unwrap_or_default().trim().to_string();
        let open_env = env::var("ROOM_OPEN").ok();
        let open_pinned = open_env.is_some();
        let open = open_env
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false);
        Config {
            bind,
            data_dir,
            files_dir,
            server_name,
            max_upload,
            page_size,
            lan_ssid_hint,
            setup_code,
            open,
            open_pinned,
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
