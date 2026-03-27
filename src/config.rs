pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:racquet.db?mode=rwc".to_string());

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!("JWT_SECRET not set, using insecure default");
            "racquet-dev-secret-do-not-use-in-prod".to_string()
        });

        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        Self {
            database_url,
            jwt_secret,
            port,
        }
    }
}
