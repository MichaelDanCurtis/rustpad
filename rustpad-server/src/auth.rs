//! Simple authentication system for file freeze feature.

use anyhow::{bail, Context, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// User account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Username
    pub username: String,
    /// Hashed password
    password_hash: String,
    /// Creation timestamp
    pub created_at: String,
}

/// Configuration for authentication
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Whether authentication is enabled
    pub enabled: bool,
    /// Directory where user data is stored
    pub data_dir: PathBuf,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            data_dir: PathBuf::from("./frozen_documents/users"),
        }
    }
}

impl AuthConfig {
    /// Create config from environment and freeze config
    pub fn from_env(freeze_enabled: bool, save_dir: &PathBuf) -> Self {
        Self {
            enabled: freeze_enabled,
            data_dir: save_dir.join("users"),
        }
    }
}

/// Manager for user authentication
#[derive(Debug)]
pub struct AuthManager {
    config: AuthConfig,
    users_cache: parking_lot::RwLock<HashMap<String, User>>,
}

impl AuthManager {
    /// Create a new auth manager
    pub fn new(config: AuthConfig) -> Result<Self> {
        if config.enabled {
            fs::create_dir_all(&config.data_dir)
                .context("Failed to create auth data directory")?;
            info!("Authentication enabled, data directory: {:?}", config.data_dir);
        }

        Ok(Self {
            config,
            users_cache: parking_lot::RwLock::new(HashMap::new()),
        })
    }

    /// Register a new user
    pub fn register(&self, username: &str, password: &str) -> Result<User> {
        if !self.config.enabled {
            bail!("Authentication feature is not enabled");
        }

        // Validate username
        if username.is_empty() || username.len() < 3 {
            bail!("Username must be at least 3 characters");
        }
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            bail!("Username can only contain letters, numbers, underscores, and hyphens");
        }

        // Validate password
        if password.len() < 6 {
            bail!("Password must be at least 6 characters");
        }

        // Check if user already exists
        if self.user_exists(username)? {
            bail!("Username already exists");
        }

        // Hash password
        let password_hash = hash(password, DEFAULT_COST)
            .context("Failed to hash password")?;

        let user = User {
            username: username.to_string(),
            password_hash,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // Save user
        self.save_user(&user)?;

        // Cache user (without password hash in response)
        let mut cache = self.users_cache.write();
        cache.insert(username.to_string(), user.clone());

        info!("User registered: {}", username);

        Ok(user)
    }

    /// Authenticate a user
    pub fn login(&self, username: &str, password: &str) -> Result<User> {
        if !self.config.enabled {
            bail!("Authentication feature is not enabled");
        }

        // Load user
        let user = self.load_user(username)
            .context("Invalid username or password")?;

        // Verify password
        let valid = verify(password, &user.password_hash)
            .context("Failed to verify password")?;

        if !valid {
            bail!("Invalid username or password");
        }

        info!("User logged in: {}", username);

        Ok(user)
    }

    /// Check if a user exists
    fn user_exists(&self, username: &str) -> Result<bool> {
        // Check cache first
        {
            let cache = self.users_cache.read();
            if cache.contains_key(username) {
                return Ok(true);
            }
        }

        // Check filesystem
        let user_file = self.config.data_dir.join(format!("{}.json", username));
        Ok(user_file.exists())
    }

    /// Load user from disk
    fn load_user(&self, username: &str) -> Result<User> {
        // Check cache first
        {
            let cache = self.users_cache.read();
            if let Some(user) = cache.get(username) {
                return Ok(user.clone());
            }
        }

        // Load from disk
        let user_file = self.config.data_dir.join(format!("{}.json", username));
        if !user_file.exists() {
            bail!("User not found");
        }

        let content = fs::read_to_string(&user_file)
            .context("Failed to read user file")?;
        let user: User = serde_json::from_str(&content)
            .context("Failed to parse user data")?;

        // Update cache
        let mut cache = self.users_cache.write();
        cache.insert(username.to_string(), user.clone());

        Ok(user)
    }

    /// Save user to disk
    fn save_user(&self, user: &User) -> Result<()> {
        let user_file = self.config.data_dir.join(format!("{}.json", user.username));
        let user_json = serde_json::to_string_pretty(user)?;
        fs::write(&user_file, user_json)
            .context("Failed to write user file")?;
        Ok(())
    }

    /// Validate a username (without password)
    pub fn validate_user(&self, username: &str) -> Result<bool> {
        if !self.config.enabled {
            return Ok(true); // If auth is disabled, all usernames are valid
        }
        self.user_exists(username)
    }
}
