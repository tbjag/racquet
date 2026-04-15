use std::path::Path;

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub frontend_url: String,
    pub allowed_emails: Vec<String>,
    pub test_mode: bool,
    pub static_dir: Option<String>,
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

        let test_mode = std::env::var("RACQUET_TEST_MODE")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let google_client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| {
            if test_mode {
                "test-client-id".to_string()
            } else {
                panic!("GOOGLE_CLIENT_ID must be set")
            }
        });

        let google_client_secret =
            std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_else(|_| {
                if test_mode {
                    "test-client-secret".to_string()
                } else {
                    panic!("GOOGLE_CLIENT_SECRET must be set")
                }
            });

        let google_redirect_uri = std::env::var("GOOGLE_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:3000/api/auth/google/callback".to_string());

        let frontend_url = std::env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:5173".to_string());

        let allowed_emails = load_allowed_emails();

        let static_dir = std::env::var("STATIC_DIR").ok().filter(|s| !s.is_empty());

        Self {
            database_url,
            jwt_secret,
            port,
            google_client_id,
            google_client_secret,
            google_redirect_uri,
            frontend_url,
            allowed_emails,
            test_mode,
            static_dir,
        }
    }
}

fn load_allowed_emails() -> Vec<String> {
    let path = std::env::var("ALLOWED_EMAILS_FILE")
        .unwrap_or_else(|_| "allowed_emails.txt".to_string());

    if !Path::new(&path).exists() {
        tracing::warn!("allowed_emails file not found at {path}, no emails whitelisted");
        return Vec::new();
    }

    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        tracing::error!("failed to read allowed_emails file: {e}");
        String::new()
    });

    content
        .lines()
        .map(|l| l.trim().to_lowercase())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}
