use racquet::{AppState, build_router, config, connection::ConnectionManager, db};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "racquet=info,tower_http=debug".parse().unwrap()),
        )
        .init();

    let config = config::Config::from_env();
    tracing::info!("configuration loaded");

    let pool = db::create_pool(&config.database_url).await;
    tracing::info!("database pool created");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("migrations failed");
    tracing::info!("database migrations applied");

    let state = AppState {
        db: pool,
        cm: Arc::new(ConnectionManager::new()),
        jwt_secret: config.jwt_secret,
        google_client_id: config.google_client_id,
        google_client_secret: config.google_client_secret,
        google_redirect_uri: config.google_redirect_uri,
        frontend_url: config.frontend_url,
        allowed_emails: config.allowed_emails,
        test_mode: config.test_mode,
        static_dir: config.static_dir,
    };

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .unwrap();
    tracing::info!("listening on 0.0.0.0:{}", config.port);
    axum::serve(listener, app).await.unwrap();
}
