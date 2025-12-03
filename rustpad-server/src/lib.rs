//! Server backend for the Rustpad collaborative text editor.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use dashmap::DashMap;
use log::{error, info};
use rand::Rng;
use serde::Serialize;
use tokio::time::{self, Instant};
use warp::{filters::BoxedFilter, ws::Ws, Filter, Rejection, Reply};

use crate::{ai::AiManager, artifacts::ArtifactManager, auth::AuthManager, database::Database, freeze::FreezeManager, rustpad::Rustpad};

pub mod ai;
pub mod artifacts;
pub mod auth;
pub mod database;
pub mod freeze;
mod ot;
mod rustpad;

/// An entry stored in the global server map.
///
/// Each entry corresponds to a single document. This is garbage collected by a
/// background task after one day of inactivity, to avoid server memory usage
/// growing without bound.
struct Document {
    last_accessed: Instant,
    rustpad: Arc<Rustpad>,
}

impl Document {
    fn new(rustpad: Arc<Rustpad>) -> Self {
        Self {
            last_accessed: Instant::now(),
            rustpad,
        }
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        self.rustpad.kill();
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct CustomReject(anyhow::Error);

impl warp::reject::Reject for CustomReject {}

/// The shared state of the server, accessible from within request handlers.
#[derive(Clone)]
struct ServerState {
    /// Concurrent map storing in-memory documents.
    documents: Arc<DashMap<String, Document>>,
    /// Connection to the database pool, if persistence is enabled.
    database: Option<Database>,
    /// File freeze manager for 30-day persistence.
    freeze_manager: Option<Arc<FreezeManager>>,
    /// Authentication manager for user accounts.
    auth_manager: Option<Arc<AuthManager>>,
    /// AI manager for OpenRouter integration.
    ai_manager: Option<Arc<AiManager>>,
    /// Artifact manager for multi-file AI outputs.
    artifact_manager: Option<Arc<ArtifactManager>>,
}

/// Statistics about the server, returned from an API endpoint.
#[derive(Serialize)]
struct Stats {
    /// System time when the server started, in seconds since Unix epoch.
    start_time: u64,
    /// Number of documents currently tracked by the server.
    num_documents: usize,
    /// Number of documents persisted in the database.
    database_size: usize,
}

/// Server configuration.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// Number of days to clean up documents after inactivity.
    pub expiry_days: u32,
    /// Database object, for persistence if desired.
    pub database: Option<Database>,
    /// Freeze manager for 30-day document persistence.
    pub freeze_manager: Option<Arc<FreezeManager>>,
    /// Authentication manager for user accounts.
    pub auth_manager: Option<Arc<AuthManager>>,
    /// AI manager for OpenRouter integration.
    pub ai_manager: Option<Arc<AiManager>>,
    /// Artifact manager for multi-file AI outputs.
    pub artifact_manager: Option<Arc<ArtifactManager>>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            expiry_days: 1,
            database: None,
            freeze_manager: None,
            auth_manager: None,
            ai_manager: None,
            artifact_manager: None,
        }
    }
}

/// A combined filter handling all server routes.
pub fn server(config: ServerConfig) -> BoxedFilter<(impl Reply,)> {
    warp::path("api")
        .and(backend(config))
        .or(frontend())
        .boxed()
}

/// Construct routes for static files from React.
fn frontend() -> BoxedFilter<(impl Reply,)> {
    warp::fs::dir("dist").boxed()
}

/// Construct backend routes, including WebSocket handlers.
fn backend(config: ServerConfig) -> BoxedFilter<(impl Reply,)> {
    let state = ServerState {
        documents: Default::default(),
        database: config.database,
        freeze_manager: config.freeze_manager.clone(),
        auth_manager: config.auth_manager.clone(),
        ai_manager: config.ai_manager.clone(),
        artifact_manager: config.artifact_manager.clone(),
    };
    tokio::spawn(cleaner(state.clone(), config.expiry_days));
    
    // Spawn freeze cleanup task if enabled
    if let Some(ref freeze_manager) = config.freeze_manager {
        tokio::spawn(freeze_cleaner(Arc::clone(freeze_manager)));
    }

    let state_filter = warp::any().map(move || state.clone());

    let socket = warp::path!("socket" / String)
        .and(warp::ws())
        .and(state_filter.clone())
        .and_then(socket_handler);

    let text = warp::path!("text" / String)
        .and(state_filter.clone())
        .and_then(text_handler);

    let start_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime returned before UNIX_EPOCH")
        .as_secs();
    let stats = warp::path!("stats")
        .and(warp::any().map(move || start_time))
        .and(state_filter.clone())
        .and_then(stats_handler);

    let freeze = warp::path("documents")
        .and(warp::path!(String / "freeze"))
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(freeze_handler);

    let download = warp::path("documents")
        .and(warp::path!(String / "download"))
        .and(warp::get())
        .and(state_filter.clone())
        .and_then(download_handler);

    let list_frozen = warp::path!("documents" / "list")
        .and(warp::get())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(list_frozen_handler);

    let delete_frozen = warp::path("documents")
        .and(warp::path!(String / "delete"))
        .and(warp::delete())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(delete_frozen_handler);

    let register = warp::path!("auth" / "register")
        .and(warp::post())
        .and(warp::body::json())
        .and(state_filter.clone())
        .and_then(register_handler);

    let login = warp::path!("auth" / "login")
        .and(warp::post())
        .and(warp::body::json())
        .and(state_filter.clone())
        .and_then(login_handler);

    let ai_models = warp::path!("ai" / "models")
        .and(warp::get())
        .and(state_filter.clone())
        .and_then(ai_models_handler);

    let ai_chat = warp::path!("ai" / "chat")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(ai_chat_handler);

    let artifacts_list = warp::path!("artifacts" / "list")
        .and(warp::get())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(artifacts_list_handler);

    let artifacts_get = warp::path!("artifacts" / String)
        .and(warp::get())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(artifacts_get_handler);

    let artifacts_store = warp::path!("artifacts" / "store")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(artifacts_store_handler);

    let artifacts_delete = warp::path!("artifacts" / String)
        .and(warp::delete())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(artifacts_delete_handler);

    let admin_users = warp::path!("admin" / "users")
        .and(warp::get())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(admin_users_handler);

    let admin_update_ai = warp::path!("admin" / "users" / String / "ai")
        .and(warp::put())
        .and(warp::body::json())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(admin_update_ai_handler);

    let admin_delete_user = warp::path!("admin" / "users" / String)
        .and(warp::delete())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(admin_delete_user_handler);

    let admin_get_settings = warp::path!("admin" / "settings")
        .and(warp::get())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(admin_get_settings_handler);

    let admin_update_api_key = warp::path!("admin" / "settings" / "api-key")
        .and(warp::put())
        .and(warp::body::json())
        .and(warp::header::optional("Authorization"))
        .and(state_filter.clone())
        .and_then(admin_update_api_key_handler);

    socket
        .or(text)
        .or(stats)
        .or(freeze)
        .or(download)
        .or(list_frozen)
        .or(delete_frozen)
        .or(register)
        .or(login)
        .or(ai_models)
        .or(ai_chat)
        .or(artifacts_list)
        .or(artifacts_get)
        .or(artifacts_store)
        .or(artifacts_delete)
        .or(admin_users)
        .or(admin_update_ai)
        .or(admin_delete_user)
        .or(admin_get_settings)
        .or(admin_update_api_key)
        .boxed()
}

/// Handler for the `/api/socket/{id}` endpoint.
async fn socket_handler(id: String, ws: Ws, state: ServerState) -> Result<impl Reply, Rejection> {
    use dashmap::mapref::entry::Entry;

    let mut entry = match state.documents.entry(id.clone()) {
        Entry::Occupied(e) => e.into_ref(),
        Entry::Vacant(e) => {
            let rustpad = Arc::new(match &state.database {
                Some(db) => db.load(&id).await.map(Rustpad::from).unwrap_or_default(),
                None => Rustpad::default(),
            });
            if let Some(db) = &state.database {
                tokio::spawn(persister(id, Arc::clone(&rustpad), db.clone()));
            }
            e.insert(Document::new(rustpad))
        }
    };

    let value = entry.value_mut();
    value.last_accessed = Instant::now();
    let rustpad = Arc::clone(&value.rustpad);
    Ok(ws.on_upgrade(|socket| async move { rustpad.on_connection(socket).await }))
}

/// Handler for the `/api/text/{id}` endpoint.
async fn text_handler(id: String, state: ServerState) -> Result<impl Reply, Rejection> {
    Ok(match state.documents.get(&id) {
        Some(value) => value.rustpad.text(),
        None => {
            if let Some(db) = &state.database {
                db.load(&id)
                    .await
                    .map(|document| document.text)
                    .unwrap_or_default()
            } else {
                String::new()
            }
        }
    })
}

/// Handler for the `/api/stats` endpoint.
async fn stats_handler(start_time: u64, state: ServerState) -> Result<impl Reply, Rejection> {
    let num_documents = state.documents.len();
    let database_size = match state.database {
        None => 0,
        Some(db) => match db.count().await {
            Ok(size) => size,
            Err(e) => return Err(warp::reject::custom(CustomReject(e))),
        },
    };
    Ok(warp::reply::json(&Stats {
        start_time,
        num_documents,
        database_size,
    }))
}

const HOUR: Duration = Duration::from_secs(3600);

/// Reclaims memory for documents.
async fn cleaner(state: ServerState, expiry_days: u32) {
    loop {
        time::sleep(HOUR).await;
        let mut keys = Vec::new();
        for entry in &*state.documents {
            if entry.last_accessed.elapsed() > HOUR * 24 * expiry_days {
                keys.push(entry.key().clone());
            }
        }
        info!("cleaner removing keys: {:?}", keys);
        for key in keys {
            state.documents.remove(&key);
        }
    }
}

const PERSIST_INTERVAL: Duration = Duration::from_secs(3);
const PERSIST_INTERVAL_JITTER: Duration = Duration::from_secs(1);

/// Persists changed documents after a fixed time interval.
async fn persister(id: String, rustpad: Arc<Rustpad>, db: Database) {
    let mut last_revision = 0;
    while !rustpad.killed() {
        let interval = PERSIST_INTERVAL
            + rand::thread_rng().gen_range(Duration::ZERO..=PERSIST_INTERVAL_JITTER);
        time::sleep(interval).await;
        let revision = rustpad.revision();
        if revision > last_revision {
            info!("persisting revision {} for id = {}", revision, id);
            if let Err(e) = db.store(&id, &rustpad.snapshot()).await {
                error!("when persisting document {}: {}", id, e);
            } else {
                last_revision = revision;
            }
        }
    }
}

/// Request body for freezing a document
#[derive(serde::Deserialize)]
struct FreezeRequest {
    language: Option<String>,
}

/// Request body for authentication
#[derive(serde::Deserialize)]
struct AuthRequest {
    username: String,
    password: String,
    #[serde(default)]
    ai_enabled: bool,
    #[serde(default)]
    is_admin: bool,
}

/// Response for authentication
#[derive(Serialize)]
struct AuthResponse {
    username: String,
    created_at: String,
    ai_enabled: bool,
    is_admin: bool,
}

/// Response for freezing a document
#[derive(Serialize)]
struct FreezeResponse {
    owner_token: String,
    document_id: String,
    frozen_at: String,
    expires_at: String,
    file_extension: String,
}

/// Extract username from Basic Auth header
fn extract_basic_auth(auth_header: Option<String>) -> Result<(String, String), warp::Rejection> {
    let auth_header = auth_header.ok_or_else(|| {
        warp::reject::custom(CustomReject(anyhow::anyhow!("Missing Authorization header")))
    })?;

    let basic = auth_header.strip_prefix("Basic ").ok_or_else(|| {
        warp::reject::custom(CustomReject(anyhow::anyhow!("Invalid Authorization format")))
    })?;

    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD.decode(basic).map_err(|_| {
        warp::reject::custom(CustomReject(anyhow::anyhow!("Invalid Base64 encoding")))
    })?;

    let credentials = String::from_utf8(decoded).map_err(|_| {
        warp::reject::custom(CustomReject(anyhow::anyhow!("Invalid UTF-8 in credentials")))
    })?;

    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(warp::reject::custom(CustomReject(anyhow::anyhow!(
            "Invalid credentials format"
        ))));
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Handler for POST /api/documents/{id}/freeze
async fn freeze_handler(
    id: String,
    req: FreezeRequest,
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let freeze_manager = state
        .freeze_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Freeze feature not enabled"))))?;

    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Extract and validate credentials
    let (username, password) = extract_basic_auth(auth)?;
    auth_manager
        .login(&username, &password)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    // Get the current document content
    let content = match state.documents.get(&id) {
        Some(doc) => doc.rustpad.text(),
        None => {
            // Try loading from database
            if let Some(db) = &state.database {
                db.load(&id)
                    .await
                    .map(|doc| doc.text)
                    .unwrap_or_default()
            } else {
                return Err(warp::reject::custom(CustomReject(anyhow::anyhow!(
                    "Document not found"
                ))));
            }
        }
    };

    // Get language
    let language = req.language.or_else(|| {
        state.documents.get(&id).and_then(|doc| {
            let snapshot = doc.rustpad.snapshot();
            snapshot.language
        })
    }).unwrap_or_else(|| "plaintext".to_string());

    // Freeze the document
    let frozen_doc = freeze_manager
        .freeze_document(&id, &username, &language, &content)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::json(&FreezeResponse {
        owner_token: frozen_doc.owner_token,
        document_id: frozen_doc.document_id,
        frozen_at: frozen_doc.frozen_at.to_rfc3339(),
        expires_at: frozen_doc.expires_at.to_rfc3339(),
        file_extension: frozen_doc.file_extension,
    }))
}

/// Handler for GET /api/documents/{id}/download
async fn download_handler(id: String, state: ServerState) -> Result<impl Reply, Rejection> {
    let content = match state.documents.get(&id) {
        Some(doc) => doc.rustpad.text(),
        None => {
            if let Some(db) = &state.database {
                db.load(&id)
                    .await
                    .map(|doc| doc.text)
                    .unwrap_or_default()
            } else {
                return Err(warp::reject::custom(CustomReject(anyhow::anyhow!(
                    "Document not found"
                ))));
            }
        }
    };

    Ok(warp::reply::with_header(
        content,
        "Content-Disposition",
        format!("attachment; filename=\"{}.txt\"", id),
    ))
}

/// Handler for GET /api/documents/list
async fn list_frozen_handler(
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let freeze_manager = state
        .freeze_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Freeze feature not enabled"))))?;

    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Extract and validate credentials
    let (username, password) = extract_basic_auth(auth)?;
    auth_manager
        .login(&username, &password)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    let documents = freeze_manager
        .list_frozen_documents(&username)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::json(&documents))
}

/// Handler for DELETE /api/documents/{id}/delete
async fn delete_frozen_handler(
    id: String,
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let freeze_manager = state
        .freeze_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Freeze feature not enabled"))))?;

    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Extract and validate credentials
    let (username, password) = extract_basic_auth(auth)?;
    auth_manager
        .login(&username, &password)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    freeze_manager
        .delete_frozen_document(&username, &id)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::with_status(
        "Document deleted",
        warp::http::StatusCode::OK,
    ))
}

/// Handler for POST /api/auth/register
async fn register_handler(
    req: AuthRequest,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    let user = auth_manager
        .register(&req.username, &req.password, req.ai_enabled, req.is_admin)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::json(&AuthResponse {
        username: user.username,
        created_at: user.created_at,
        ai_enabled: user.ai_enabled,
        is_admin: user.is_admin,
    }))
}

/// Handler for POST /api/auth/login
async fn login_handler(
    req: AuthRequest,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    let user = auth_manager
        .login(&req.username, &req.password)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::json(&AuthResponse {
        username: user.username,
        created_at: user.created_at,
        ai_enabled: user.ai_enabled,
        is_admin: user.is_admin,
    }))
}

/// Cleanup task for expired frozen documents
async fn freeze_cleaner(freeze_manager: Arc<FreezeManager>) {
    loop {
        time::sleep(HOUR * 6).await; // Run every 6 hours
        match freeze_manager.cleanup_expired() {
            Ok(count) if count > 0 => info!("Cleaned up {} expired frozen documents", count),
            Err(e) => error!("Error during freeze cleanup: {}", e),
            _ => {}
        }
    }
}

/// Request body for AI chat
#[derive(serde::Deserialize)]
struct AiChatRequest {
    model: String,
    messages: Vec<ai::ChatMessage>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    temperature: Option<f32>,
}

/// Handler for GET /api/ai/models
async fn ai_models_handler(state: ServerState) -> Result<impl Reply, Rejection> {
    let ai_manager = state
        .ai_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("AI features not enabled"))))?;

    if !ai_manager.is_enabled() {
        return Err(warp::reject::custom(CustomReject(anyhow::anyhow!(
            "AI features not configured"
        ))));
    }

    let models = ai_manager.get_available_models();
    Ok(warp::reply::json(&models))
}

/// Handler for POST /api/ai/chat
async fn ai_chat_handler(
    req: AiChatRequest,
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let ai_manager = state
        .ai_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("AI features not enabled"))))?;

    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Extract and validate credentials
    let (username, password) = extract_basic_auth(auth)?;
    let user = auth_manager
        .login(&username, &password)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    // Check if user has AI access
    if !user.ai_enabled {
        return Err(warp::reject::custom(CustomReject(anyhow::anyhow!(
            "AI features not enabled for this user"
        ))));
    }

    // Make the API call
    let response = ai_manager
        .chat_completion(&req.model, req.messages, req.max_tokens, req.temperature)
        .await
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::json(&response))
}

/// Request body for storing artifacts
#[derive(serde::Deserialize)]
struct ArtifactStoreRequest {
    document_id: String,
    model: String,
    prompt: String,
    files: Vec<artifacts::ArtifactFile>,
}

/// Handler for GET /api/artifacts/list
async fn artifacts_list_handler(
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let artifact_manager = state
        .artifact_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Artifact storage not enabled"))))?;

    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Extract and validate credentials
    let (username, password) = extract_basic_auth(auth)?;
    auth_manager
        .login(&username, &password)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    let artifacts = artifact_manager
        .list_artifacts(&username)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::json(&artifacts))
}

/// Handler for GET /api/artifacts/{id}
async fn artifacts_get_handler(
    artifact_id: String,
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let artifact_manager = state
        .artifact_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Artifact storage not enabled"))))?;

    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Extract and validate credentials
    let (username, password) = extract_basic_auth(auth)?;
    auth_manager
        .login(&username, &password)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    let artifact = artifact_manager
        .get_artifact(&username, &artifact_id)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::json(&artifact))
}

/// Handler for POST /api/artifacts/store
async fn artifacts_store_handler(
    req: ArtifactStoreRequest,
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let artifact_manager = state
        .artifact_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Artifact storage not enabled"))))?;

    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Extract and validate credentials
    let (username, password) = extract_basic_auth(auth)?;
    auth_manager
        .login(&username, &password)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    let metadata = artifact_manager
        .store_artifact(&username, &req.document_id, &req.model, &req.prompt, req.files)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::json(&metadata))
}

/// Handler for DELETE /api/artifacts/{id}
async fn artifacts_delete_handler(
    artifact_id: String,
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let artifact_manager = state
        .artifact_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Artifact storage not enabled"))))?;

    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Extract and validate credentials
    let (username, password) = extract_basic_auth(auth)?;
    auth_manager
        .login(&username, &password)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    artifact_manager
        .delete_artifact(&username, &artifact_id)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::with_status(
        "Artifact deleted",
        warp::http::StatusCode::OK,
    ))
}

/// User info for admin panel (without password hash)
#[derive(Serialize)]
struct AdminUserInfo {
    username: String,
    created_at: String,
    ai_enabled: bool,
    is_admin: bool,
}

/// Request to update AI access
#[derive(serde::Deserialize)]
struct UpdateAiAccessRequest {
    ai_enabled: bool,
}

/// Helper function to check admin access
fn check_admin_access(auth: Option<String>, auth_manager: &AuthManager) -> Result<(), Rejection> {
    let (username, password) = extract_basic_auth(auth)?;
    let user = auth_manager
        .login(&username, &password)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    if !user.is_admin {
        return Err(warp::reject::custom(CustomReject(anyhow::anyhow!(
            "Admin access required"
        ))));
    }

    Ok(())
}

/// Handler for GET /api/admin/users
async fn admin_users_handler(
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Check admin access
    check_admin_access(auth, auth_manager)?;

    let users = auth_manager
        .list_users()
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    // Convert to admin user info (remove password hash)
    let admin_users: Vec<AdminUserInfo> = users
        .into_iter()
        .map(|u| AdminUserInfo {
            username: u.username,
            created_at: u.created_at,
            ai_enabled: u.ai_enabled,
            is_admin: u.is_admin,
        })
        .collect();

    Ok(warp::reply::json(&admin_users))
}

/// Handler for PUT /api/admin/users/{username}/ai
async fn admin_update_ai_handler(
    username: String,
    req: UpdateAiAccessRequest,
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Check admin access
    check_admin_access(auth, auth_manager)?;

    auth_manager
        .update_ai_access(&username, req.ai_enabled)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::with_status(
        "AI access updated",
        warp::http::StatusCode::OK,
    ))
}

/// Handler for DELETE /api/admin/users/{username}
async fn admin_delete_user_handler(
    username: String,
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Check admin access
    check_admin_access(auth, auth_manager)?;

    auth_manager
        .delete_user(&username)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::with_status(
        "User deleted",
        warp::http::StatusCode::OK,
    ))
}

/// Admin settings response
#[derive(Serialize)]
struct AdminSettings {
    ai_enabled: bool,
    api_key_configured: bool,
    api_key_preview: Option<String>,
}

/// Update API key request
#[derive(serde::Deserialize)]
struct UpdateApiKeyRequest {
    api_key: String,
}

/// Handler for GET /api/admin/settings
async fn admin_get_settings_handler(
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Check admin access
    check_admin_access(auth, auth_manager)?;

    let ai_manager = state.ai_manager.as_ref();
    let (ai_enabled, api_key_configured, api_key_preview) = if let Some(ai) = ai_manager {
        let key = ai.get_api_key();
        let configured = !key.is_empty();
        let preview = if configured {
            Some(format!("{}...{}", &key[..8.min(key.len())], &key[key.len().saturating_sub(4)..]))
        } else {
            None
        };
        (ai.is_enabled(), configured, preview)
    } else {
        (false, false, None)
    };

    Ok(warp::reply::json(&AdminSettings {
        ai_enabled,
        api_key_configured,
        api_key_preview,
    }))
}

/// Handler for PUT /api/admin/settings/api-key
async fn admin_update_api_key_handler(
    req: UpdateApiKeyRequest,
    auth: Option<String>,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Auth not enabled"))))?;

    // Check admin access
    check_admin_access(auth, auth_manager)?;

    let ai_manager = state
        .ai_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("AI not enabled"))))?;

    ai_manager
        .update_api_key(&req.api_key)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::with_status(
        "API key updated",
        warp::http::StatusCode::OK,
    ))
}
