//! Node configuration — persisted in `~/.exochain/config.toml`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Node configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// HTTP API port.
    pub api_port: u16,
    /// P2P listen port.
    pub p2p_port: u16,
    /// Seed node addresses for bootstrapping.
    #[serde(default)]
    pub seeds: Vec<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            api_port: 8080,
            p2p_port: 4001,
            seeds: Vec::new(),
        }
    }
}

/// Resolve the data directory, creating it if necessary.
pub fn resolve_data_dir(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let dir = match explicit {
        Some(d) => d,
        None => {
            let proj = directories::ProjectDirs::from("io", "exochain", "exochain")
                .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
            proj.data_dir().to_path_buf()
        }
    };
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Load config from `config.toml` in the data directory, or create defaults.
pub fn load_or_create(data_dir: &Path) -> anyhow::Result<NodeConfig> {
    let config_path = data_dir.join("config.toml");
    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        let cfg: NodeConfig = toml::from_str(&contents)?;
        Ok(cfg)
    } else {
        let cfg = NodeConfig::default();
        let contents = toml::to_string_pretty(&cfg)?;
        std::fs::write(&config_path, contents)?;
        tracing::info!(path = %config_path.display(), "Created default config");
        Ok(cfg)
    }
}
