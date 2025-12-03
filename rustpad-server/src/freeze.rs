//! File freeze functionality for persisting documents beyond the default expiry.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// Metadata about a frozen document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenDocument {
    /// Unique identifier for the document
    pub document_id: String,
    /// Owner's authentication token
    pub owner_token: String,
    /// Programming language of the document
    pub language: String,
    /// File extension based on language
    pub file_extension: String,
    /// Timestamp when document was frozen
    pub frozen_at: DateTime<Utc>,
    /// Timestamp when document expires (30 days from freeze)
    pub expires_at: DateTime<Utc>,
    /// Path to the frozen file on disk
    pub file_path: PathBuf,
    /// Size of the file in bytes
    pub file_size: u64,
}

/// Configuration for file freeze feature
#[derive(Debug, Clone)]
pub struct FreezeConfig {
    /// Whether file freeze feature is enabled
    pub enabled: bool,
    /// Directory where frozen files are saved
    pub save_dir: PathBuf,
    /// Maximum file size in bytes
    pub max_file_size: u64,
}

impl Default for FreezeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            save_dir: PathBuf::from("./frozen_documents"),
            max_file_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

impl FreezeConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let enabled = std::env::var("ENABLE_FILE_FREEZE")
            .unwrap_or_else(|_| String::from("false"))
            .parse()
            .unwrap_or(false);

        let save_dir = std::env::var("SAVE_DIR")
            .unwrap_or_else(|_| String::from("./frozen_documents"))
            .into();

        Self {
            enabled,
            save_dir,
            max_file_size: 10 * 1024 * 1024,
        }
    }
}

/// Manager for frozen documents
#[derive(Debug)]
pub struct FreezeManager {
    config: FreezeConfig,
    metadata_cache: parking_lot::RwLock<HashMap<String, Vec<FrozenDocument>>>,
}

impl FreezeManager {
    /// Create a new freeze manager
    pub fn new(config: FreezeConfig) -> Result<Self> {
        if config.enabled {
            fs::create_dir_all(&config.save_dir)
                .context("Failed to create save directory")?;
            info!("File freeze enabled, save directory: {:?}", config.save_dir);
        }

        Ok(Self {
            config,
            metadata_cache: parking_lot::RwLock::new(HashMap::new()),
        })
    }

    /// Generate an owner token (UUID v4)
    pub fn generate_owner_token() -> String {
        Uuid::new_v4().to_string()
    }

    /// Get file extension for a language
    fn get_extension(language: &str) -> &str {
        match language {
            "rust" => "rs",
            "python" => "py",
            "javascript" => "js",
            "typescript" => "ts",
            "java" => "java",
            "cpp" | "c++" => "cpp",
            "c" => "c",
            "go" => "go",
            "ruby" => "rb",
            "php" => "php",
            "swift" => "swift",
            "kotlin" => "kt",
            "scala" => "scala",
            "html" => "html",
            "css" => "css",
            "json" => "json",
            "xml" => "xml",
            "yaml" | "yml" => "yaml",
            "markdown" => "md",
            "sql" => "sql",
            "bash" | "shell" => "sh",
            _ => "txt",
        }
    }

    /// Freeze a document
    pub fn freeze_document(
        &self,
        document_id: &str,
        username: &str,
        language: &str,
        content: &str,
    ) -> Result<FrozenDocument> {
        if !self.config.enabled {
            bail!("File freeze feature is not enabled");
        }

        let content_bytes = content.as_bytes();
        if content_bytes.len() as u64 > self.config.max_file_size {
            bail!(
                "Document size ({} bytes) exceeds maximum ({})",
                content_bytes.len(),
                self.config.max_file_size
            );
        }

        // Create directory structure: {SAVE_DIR}/frozen/{username}/
        let owner_dir = self.config.save_dir.join("frozen").join(username);
        fs::create_dir_all(&owner_dir)
            .context("Failed to create owner directory")?;

        let file_extension = Self::get_extension(language);
        let filename = format!("{}.{}", document_id, file_extension);
        let file_path = owner_dir.join(&filename);

        // Write the file
        fs::write(&file_path, content_bytes)
            .context("Failed to write frozen document")?;

        let frozen_at = Utc::now();
        let expires_at = frozen_at + Duration::days(30);

        let frozen_doc = FrozenDocument {
            document_id: document_id.to_string(),
            owner_token: username.to_string(),
            language: language.to_string(),
            file_extension: file_extension.to_string(),
            frozen_at,
            expires_at,
            file_path: file_path.clone(),
            file_size: content_bytes.len() as u64,
        };

        // Save metadata
        self.save_metadata(&frozen_doc)?;

        // Cache the metadata
        let mut cache = self.metadata_cache.write();
        cache
            .entry(username.to_string())
            .or_insert_with(Vec::new)
            .push(frozen_doc.clone());

        info!(
            "Frozen document: id={}, username={}, size={} bytes",
            document_id, username, content_bytes.len()
        );

        Ok(frozen_doc)
    }

    /// List frozen documents for a user
    pub fn list_frozen_documents(&self, username: &str) -> Result<Vec<FrozenDocument>> {
        if !self.config.enabled {
            bail!("File freeze feature is not enabled");
        }

        // Check cache first
        {
            let cache = self.metadata_cache.read();
            if let Some(docs) = cache.get(username) {
                return Ok(docs.clone());
            }
        }

        // Load from filesystem
        let owner_dir = self.config.save_dir.join("frozen").join(username);
        if !owner_dir.exists() {
            return Ok(Vec::new());
        }

        let metadata_file = owner_dir.join("metadata.json");
        if !metadata_file.exists() {
            return Ok(Vec::new());
        }

        let metadata_content = fs::read_to_string(&metadata_file)
            .context("Failed to read metadata file")?;
        let documents: Vec<FrozenDocument> = serde_json::from_str(&metadata_content)
            .context("Failed to parse metadata")?;

        // Update cache
        let mut cache = self.metadata_cache.write();
        cache.insert(username.to_string(), documents.clone());

        Ok(documents)
    }

    /// Get a specific frozen document content
    pub fn get_frozen_document(&self, username: &str, document_id: &str) -> Result<String> {
        if !self.config.enabled {
            bail!("File freeze feature is not enabled");
        }

        let documents = self.list_frozen_documents(username)?;
        let doc = documents
            .iter()
            .find(|d| d.document_id == document_id)
            .context("Document not found")?;

        if !doc.file_path.exists() {
            bail!("Frozen document file not found");
        }

        let content = fs::read_to_string(&doc.file_path)
            .context("Failed to read frozen document")?;

        Ok(content)
    }

    /// Delete a frozen document
    pub fn delete_frozen_document(&self, username: &str, document_id: &str) -> Result<()> {
        if !self.config.enabled {
            bail!("File freeze feature is not enabled");
        }

        let owner_dir = self.config.save_dir.join("frozen").join(username);
        let metadata_file = owner_dir.join("metadata.json");

        if !metadata_file.exists() {
            bail!("No frozen documents found for this user");
        }

        // Load existing metadata
        let content = fs::read_to_string(&metadata_file)
            .context("Failed to read metadata file")?;
        let mut documents: Vec<FrozenDocument> = serde_json::from_str(&content)
            .context("Failed to parse metadata")?;

        // Find and remove the document
        let doc_index = documents
            .iter()
            .position(|d| d.document_id == document_id)
            .context("Document not found")?;

        let doc = documents.remove(doc_index);

        // Delete the file
        if doc.file_path.exists() {
            fs::remove_file(&doc.file_path)
                .context("Failed to delete document file")?;
        }

        // Update metadata
        if documents.is_empty() {
            // Remove entire owner directory if no documents left
            fs::remove_dir_all(&owner_dir)
                .context("Failed to remove owner directory")?;
        } else {
            // Save updated metadata
            let metadata_json = serde_json::to_string_pretty(&documents)?;
            fs::write(&metadata_file, metadata_json)
                .context("Failed to save metadata")?;
        }

        // Update cache
        let mut cache = self.metadata_cache.write();
        if documents.is_empty() {
            cache.remove(username);
        } else {
            cache.insert(username.to_string(), documents);
        }

        info!("Deleted frozen document: id={}, username={}", document_id, username);

        Ok(())
    }

    /// Save metadata to disk
    fn save_metadata(&self, frozen_doc: &FrozenDocument) -> Result<()> {
        let owner_dir = self
            .config
            .save_dir
            .join("frozen")
            .join(&frozen_doc.owner_token);
        let metadata_file = owner_dir.join("metadata.json");

        // Load existing metadata
        let mut documents: Vec<FrozenDocument> = if metadata_file.exists() {
            let content = fs::read_to_string(&metadata_file)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Update or add the document
        if let Some(existing) = documents
            .iter_mut()
            .find(|d| d.document_id == frozen_doc.document_id)
        {
            *existing = frozen_doc.clone();
        } else {
            documents.push(frozen_doc.clone());
        }

        // Save updated metadata
        let metadata_json = serde_json::to_string_pretty(&documents)?;
        fs::write(&metadata_file, metadata_json)?;

        Ok(())
    }

    /// Clean up expired frozen documents
    pub fn cleanup_expired(&self) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let frozen_dir = self.config.save_dir.join("frozen");
        if !frozen_dir.exists() {
            return Ok(0);
        }

        let mut cleaned_count = 0;
        let now = Utc::now();

        for entry in fs::read_dir(&frozen_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let owner_token = entry.file_name().to_string_lossy().to_string();
            let metadata_file = entry.path().join("metadata.json");

            if !metadata_file.exists() {
                continue;
            }

            let content = fs::read_to_string(&metadata_file)?;
            let mut documents: Vec<FrozenDocument> = serde_json::from_str(&content)?;

            let original_len = documents.len();
            documents.retain(|doc| {
                if doc.expires_at < now {
                    // Delete the file
                    if doc.file_path.exists() {
                        if let Err(e) = fs::remove_file(&doc.file_path) {
                            warn!("Failed to delete expired file: {}", e);
                        }
                    }
                    false
                } else {
                    true
                }
            });

            cleaned_count += original_len - documents.len();

            if documents.is_empty() {
                // Remove the entire owner directory if empty
                if let Err(e) = fs::remove_dir_all(entry.path()) {
                    warn!("Failed to remove empty owner directory: {}", e);
                }
            } else {
                // Save updated metadata
                let metadata_json = serde_json::to_string_pretty(&documents)?;
                fs::write(&metadata_file, metadata_json)?;
            }

            // Update cache
            let mut cache = self.metadata_cache.write();
            if documents.is_empty() {
                cache.remove(&owner_token);
            } else {
                cache.insert(owner_token, documents);
            }
        }

        if cleaned_count > 0 {
            info!("Cleaned up {} expired frozen documents", cleaned_count);
        }

        Ok(cleaned_count)
    }
}
