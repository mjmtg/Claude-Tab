use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInput {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default = "default_input_type")]
    pub input_type: String,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    pub default: Option<String>,
}

fn default_input_type() -> String {
    "text".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkingDirConfig {
    Fixed { path: String },
    Prompt,
    FromInput { key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub config_path: Option<String>,
    #[serde(default)]
    pub servers: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub working_directory: Option<WorkingDirConfig>,
    #[serde(default)]
    pub prompt_template: Option<String>,
    #[serde(default)]
    pub auto_execute: bool,
    #[serde(default)]
    pub mcp_servers: Option<McpConfig>,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub inputs: Vec<ProfileInput>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileLaunchRequest {
    pub profile_id: String,
    #[serde(default)]
    pub input_values: HashMap<String, String>,
    #[serde(default)]
    pub working_directory: Option<String>,
}

pub struct ProfileStore {
    global_dir: PathBuf,
    profiles: RwLock<HashMap<String, Profile>>,
}

impl ProfileStore {
    pub fn new() -> Self {
        let home = dirs_home();
        let global_dir = home.join(".claude-tabs").join("profiles");
        Self {
            global_dir,
            profiles: RwLock::new(HashMap::new()),
        }
    }

    pub async fn init(&self) {
        if let Err(e) = std::fs::create_dir_all(&self.global_dir) {
            error!(error = %e, "Failed to create profiles directory");
            return;
        }
        self.reload().await;
    }

    pub async fn reload(&self) {
        let mut profiles = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(&self.global_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    match load_profile_file(&path) {
                        Ok(profile) => {
                            debug!(id = %profile.id, "Loaded profile");
                            profiles.insert(profile.id.clone(), profile);
                        }
                        Err(e) => {
                            warn!(path = %path.display(), error = %e, "Failed to load profile");
                        }
                    }
                }
            }
        }
        info!(count = profiles.len(), "Profiles loaded");
        *self.profiles.write().await = profiles;
    }

    pub async fn list(&self) -> Vec<Profile> {
        let profiles = self.profiles.read().await;
        let mut list: Vec<_> = profiles.values().cloned().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub async fn get(&self, id: &str) -> Option<Profile> {
        self.profiles.read().await.get(id).cloned()
    }

    pub async fn save(&self, profile: Profile) -> Result<(), String> {
        let filename = format!("{}.json", &profile.id);
        let path = self.global_dir.join(&filename);
        let json = serde_json::to_string_pretty(&profile)
            .map_err(|e| format!("Failed to serialize profile: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write profile file: {}", e))?;
        info!(id = %profile.id, path = %path.display(), "Profile saved");
        self.profiles.write().await.insert(profile.id.clone(), profile);
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<(), String> {
        let filename = format!("{}.json", id);
        let path = self.global_dir.join(&filename);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete profile file: {}", e))?;
        }
        self.profiles.write().await.remove(id);
        info!(id = %id, "Profile deleted");
        Ok(())
    }

    pub fn resolve_prompt(&self, template: &str, input_values: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in input_values {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        result
    }

    pub fn write_temp_mcp_config(
        &self,
        session_id: &str,
        mcp_config: &McpConfig,
    ) -> Result<Option<PathBuf>, String> {
        if let Some(ref servers) = mcp_config.servers {
            let tmp_dir = dirs_home().join(".claude-tabs").join("tmp");
            std::fs::create_dir_all(&tmp_dir)
                .map_err(|e| format!("Failed to create tmp dir: {}", e))?;
            let path = tmp_dir.join(format!("mcp-{}.json", session_id));
            let config_json = serde_json::json!({ "mcpServers": servers });
            let json = serde_json::to_string_pretty(&config_json)
                .map_err(|e| format!("Failed to serialize MCP config: {}", e))?;
            std::fs::write(&path, json)
                .map_err(|e| format!("Failed to write temp MCP config: {}", e))?;
            debug!(path = %path.display(), "Wrote temp MCP config");
            Ok(Some(path))
        } else if let Some(ref config_path) = mcp_config.config_path {
            Ok(Some(PathBuf::from(config_path)))
        } else {
            Ok(None)
        }
    }
}

fn load_profile_file(path: &Path) -> Result<Profile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .expect("HOME environment variable must be set")
}

pub fn cleanup_temp_mcp_config(session_id: &str) {
    let path = dirs_home()
        .join(".claude-tabs")
        .join("tmp")
        .join(format!("mcp-{}.json", session_id));
    if path.exists() {
        if let Err(e) = std::fs::remove_file(&path) {
            warn!(error = %e, path = %path.display(), "Failed to cleanup temp MCP config");
        } else {
            debug!(path = %path.display(), "Cleaned up temp MCP config");
        }
    }
}
