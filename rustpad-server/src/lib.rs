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

use crate::{database::Database, freeze::FreezeManager, rustpad::Rustpad};

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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            expiry_days: 1,
            database: None,
            freeze_manager: None,
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

    socket
        .or(text)
        .or(stats)
        .or(freeze)
        .or(download)
        .or(list_frozen)
        .or(delete_frozen)
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
    owner_token: Option<String>,
    language: Option<String>,
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

/// Handler for POST /api/documents/{id}/freeze
async fn freeze_handler(
    id: String,
    req: FreezeRequest,
    state: ServerState,
) -> Result<impl Reply, Rejection> {
    let freeze_manager = state
        .freeze_manager
        .as_ref()
        .ok_or_else(|| warp::reject::custom(CustomReject(anyhow::anyhow!("Freeze feature not enabled"))))?;

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

    // Get or generate owner token
    let owner_token = req.owner_token.unwrap_or_else(FreezeManager::generate_owner_token);

    // Freeze the document
    let frozen_doc = freeze_manager
        .freeze_document(&id, &owner_token, &language, &content)
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

    // Extract token from Authorization header
    let owner_token = auth
        .and_then(|h| h.strip_prefix("Bearer ").map(String::from))
        .ok_or_else(|| {
            warp::reject::custom(CustomReject(anyhow::anyhow!("Missing or invalid Authorization header")))
        })?;

    let documents = freeze_manager
        .list_frozen_documents(&owner_token)
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

    // Extract token from Authorization header
    let owner_token = auth
        .and_then(|h| h.strip_prefix("Bearer ").map(String::from))
        .ok_or_else(|| {
            warp::reject::custom(CustomReject(anyhow::anyhow!("Missing or invalid Authorization header")))
        })?;

    freeze_manager
        .delete_frozen_document(&owner_token, &id)
        .map_err(|e| warp::reject::custom(CustomReject(e)))?;

    Ok(warp::reply::with_status(
        "Document deleted",
        warp::http::StatusCode::OK,
    ))
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
