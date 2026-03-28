use std::sync::Arc;
use racquet::{config, connection::ConnectionManager, db, AppState, build_router};

#[tokio::main]
async fn main() {
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

    sqlx::migrate!().run(&pool).await.expect("migrations failed");
    tracing::info!("database migrations applied");

    let state = AppState {
        db: pool,
        cm: Arc::new(ConnectionManager::new()),
        jwt_secret: config.jwt_secret,
    };

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .unwrap();
    tracing::info!("listening on 0.0.0.0:{}", config.port);
    axum::serve(listener, app).await.unwrap();
}
