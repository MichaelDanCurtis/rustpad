//! Artifact storage for AI-generated multi-file outputs.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Configuration for artifact storage
#[derive(Debug, Clone)]
pub struct ArtifactConfig {
    /// Whether artifact storage is enabled
    pub enabled: bool,
    /// Directory where artifacts are stored
    pub storage_dir: PathBuf,
}

impl Default for ArtifactConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            storage_dir: PathBuf::from("./artifacts"),
        }
    }
}

impl ArtifactConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let enabled = std::env::var("ENABLE_ARTIFACTS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let storage_dir = std::env::var("ARTIFACTS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./artifacts"));

        Self {
            enabled,
            storage_dir,
        }
    }
}

/// Metadata for a stored artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Unique artifact ID
    pub id: String,
    /// Username who created the artifact
    pub username: String,
    /// Original document ID that prompted the artifact
    pub document_id: String,
    /// AI model used to generate the artifact
    pub model: String,
    /// User prompt that generated the artifact
    pub prompt: String,
    /// Number of files in this artifact
    pub file_count: usize,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Total size in bytes
    pub total_size: u64,
}

/// A single file within an artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFile {
    /// Filename
    pub name: String,
    /// File content
    pub content: String,
    /// File size in bytes
    pub size: u64,
}

/// Complete artifact with metadata and files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Metadata
    #[serde(flatten)]
    pub metadata: ArtifactMetadata,
    /// Files in this artifact
    pub files: Vec<ArtifactFile>,
}

/// Manager for artifact storage operations
#[derive(Debug)]
pub struct ArtifactManager {
    config: ArtifactConfig,
}

impl ArtifactManager {
    /// Create a new artifact manager
    pub fn new(config: ArtifactConfig) -> Result<Self> {
        if config.enabled {
            fs::create_dir_all(&config.storage_dir)
                .context("Failed to create artifacts directory")?;
            info!("Artifact storage enabled, directory: {:?}", config.storage_dir);
        }

        Ok(Self { config })
    }

    /// Check if artifact storage is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Store a new artifact
    pub fn store_artifact(
        &self,
        username: &str,
        document_id: &str,
        model: &str,
        prompt: &str,
        files: Vec<ArtifactFile>,
    ) -> Result<ArtifactMetadata> {
        if !self.config.enabled {
            anyhow::bail!("Artifact storage is not enabled");
        }

        // Generate unique artifact ID
        let artifact_id = uuid::Uuid::new_v4().to_string();

        // Calculate total size
        let total_size: u64 = files.iter().map(|f| f.size).sum();

        // Create metadata
        let metadata = ArtifactMetadata {
            id: artifact_id.clone(),
            username: username.to_string(),
            document_id: document_id.to_string(),
            model: model.to_string(),
            prompt: prompt.to_string(),
            file_count: files.len(),
            created_at: Utc::now(),
            total_size,
        };

        // Create user directory if it doesn't exist
        let user_dir = self.config.storage_dir.join(username);
        fs::create_dir_all(&user_dir)?;

        // Create artifact directory
        let artifact_dir = user_dir.join(&artifact_id);
        fs::create_dir_all(&artifact_dir)?;

        // Save metadata
        let metadata_path = artifact_dir.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_path, metadata_json)?;

        // Save each file
        for file in &files {
            let file_path = artifact_dir.join(&file.name);
            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&file_path, &file.content)?;
        }

        info!(
            "Stored artifact {} for user {} with {} files",
            artifact_id,
            username,
            files.len()
        );

        Ok(metadata)
    }

    /// List artifacts for a user
    pub fn list_artifacts(&self, username: &str) -> Result<Vec<ArtifactMetadata>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let user_dir = self.config.storage_dir.join(username);
        if !user_dir.exists() {
            return Ok(Vec::new());
        }

        let mut artifacts = Vec::new();

        for entry in fs::read_dir(&user_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let metadata_path = entry.path().join("metadata.json");
            if !metadata_path.exists() {
                continue;
            }

            let metadata_json = fs::read_to_string(&metadata_path)?;
            let metadata: ArtifactMetadata = serde_json::from_str(&metadata_json)?;
            artifacts.push(metadata);
        }

        // Sort by creation time, newest first
        artifacts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(artifacts)
    }

    /// Get a specific artifact with all its files
    pub fn get_artifact(&self, username: &str, artifact_id: &str) -> Result<Artifact> {
        if !self.config.enabled {
            anyhow::bail!("Artifact storage is not enabled");
        }

        let artifact_dir = self.config.storage_dir.join(username).join(artifact_id);
        if !artifact_dir.exists() {
            anyhow::bail!("Artifact not found");
        }

        // Load metadata
        let metadata_path = artifact_dir.join("metadata.json");
        let metadata_json = fs::read_to_string(&metadata_path)?;
        let metadata: ArtifactMetadata = serde_json::from_str(&metadata_json)?;

        // Load all files
        let mut files = Vec::new();
        for entry in fs::read_dir(&artifact_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            // Skip metadata.json
            if path.file_name().and_then(|n| n.to_str()) == Some("metadata.json") {
                continue;
            }

            if path.is_file() {
                let name = path
                    .strip_prefix(&artifact_dir)?
                    .to_string_lossy()
                    .to_string();
                let content = fs::read_to_string(&path)?;
                let size = content.len() as u64;

                files.push(ArtifactFile {
                    name,
                    content,
                    size,
                });
            }
        }

        Ok(Artifact { metadata, files })
    }

    /// Delete an artifact
    pub fn delete_artifact(&self, username: &str, artifact_id: &str) -> Result<()> {
        if !self.config.enabled {
            anyhow::bail!("Artifact storage is not enabled");
        }

        let artifact_dir = self.config.storage_dir.join(username).join(artifact_id);
        if !artifact_dir.exists() {
            anyhow::bail!("Artifact not found");
        }

        fs::remove_dir_all(&artifact_dir)?;
        info!("Deleted artifact {} for user {}", artifact_id, username);

        Ok(())
    }
}
