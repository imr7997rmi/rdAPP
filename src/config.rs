use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Host {
    pub name: String,
    /// host o host:puerto
    pub address: String,
    pub user: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub width: Option<u16>,
    #[serde(default)]
    pub height: Option<u16>,
    /// Escalado del escritorio remoto en % (100–500)
    #[serde(default)]
    pub scale: Option<u32>,
    /// Argumentos extra para ironrdp-viewer
    #[serde(default)]
    pub args: Vec<String>,
}

impl Host {
    pub fn keychain_account(&self) -> String {
        format!("{}@{}", self.user, self.address)
    }
}

#[derive(Deserialize)]
struct File {
    #[serde(default, rename = "host")]
    hosts: Vec<Host>,
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}

/// iCloud Drive si existe; si no, ~/.config/rdapp. `RDAPP_CONFIG` lo sobrescribe.
pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("RDAPP_CONFIG") {
        return PathBuf::from(p);
    }
    let icloud = home().join("Library/Mobile Documents/com~apple~CloudDocs");
    let dir = if icloud.is_dir() {
        icloud.join("rdAPP")
    } else {
        home().join(".config/rdapp")
    };
    dir.join("hosts.toml")
}

const EXAMPLE: &str = r#"# rdAPP — lista de equipos (se sincroniza por iCloud Drive)
# Las contraseñas NO van aquí: se piden la primera vez y se guardan en el Llavero.

[[host]]
name = "Ejemplo Windows 10"
address = "192.168.1.50"      # o "host:3389"
user = "usuario"
# domain = "EMPRESA"
# width = 1920
# height = 1080
# scale = 100
# args = ["--no-server-pointer"]
"#;

pub fn ensure_exists(path: &Path) {
    if !path.exists() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, EXAMPLE);
    }
}

pub fn load(path: &Path) -> Result<Vec<Host>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("No se puede leer hosts.toml: {e}"))?;
    let f: File = toml::from_str(&text).map_err(|e| {
        let msg = e.message().to_string();
        format!("Error en hosts.toml: {msg}")
    })?;
    Ok(f.hosts)
}
