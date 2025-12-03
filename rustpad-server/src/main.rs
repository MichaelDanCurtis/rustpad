use rustpad_server::{server, auth::{AuthConfig, AuthManager}, database::Database, freeze::{FreezeConfig, FreezeManager}, ServerConfig};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| String::from("3030"))
        .parse()
        .expect("Unable to parse PORT");

    let freeze_config = FreezeConfig::from_env();
    let freeze_manager = if freeze_config.enabled {
        Some(std::sync::Arc::new(
            FreezeManager::new(freeze_config.clone())
                .expect("Unable to initialize FreezeManager"),
        ))
    } else {
        None
    };

    let auth_config = AuthConfig::from_env(freeze_config.enabled, &freeze_config.save_dir);
    let auth_manager = if auth_config.enabled {
        Some(std::sync::Arc::new(
            AuthManager::new(auth_config)
                .expect("Unable to initialize AuthManager"),
        ))
    } else {
        None
    };

    let config = ServerConfig {
        expiry_days: std::env::var("EXPIRY_DAYS")
            .unwrap_or_else(|_| String::from("1"))
            .parse()
            .expect("Unable to parse EXPIRY_DAYS"),
        database: match std::env::var("SQLITE_URI") {
            Ok(uri) => Some(
                Database::new(&uri)
                    .await
                    .expect("Unable to connect to SQLITE_URI"),
            ),
            Err(_) => None,
        },
        freeze_manager,
        auth_manager,
    };

    warp::serve(server(config)).run(([0, 0, 0, 0], port)).await;
}
