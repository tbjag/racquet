use crate::errors::AppError;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};

pub fn create_token(
    user_id: &str,
    username: &str,
    email: &str,
    secret: &str,
) -> Result<String, AppError> {
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        email: email.to_string(),
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub email: String,
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
    pub email: String,
}

impl axum::extract::FromRequestParts<crate::AppState> for AuthUser {
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &crate::AppState,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let jwt_secret = state.jwt_secret.clone();

        async move {
            let header = auth_header.ok_or_else(|| {
                tracing::debug!("request missing authorization header");
                AppError::Unauthorized("missing authorization header".to_string())
            })?;

            let token = header.strip_prefix("Bearer ").ok_or_else(|| {
                tracing::debug!("invalid authorization header format");
                AppError::Unauthorized("invalid authorization header format".to_string())
            })?;

            let claims = verify_token(token, &jwt_secret).map_err(|e| {
                tracing::warn!("token verification failed");
                e
            })?;

            Ok(AuthUser {
                user_id: claims.sub,
                username: claims.username,
                email: claims.email,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_decode_token() {
        let secret = "test-secret-key";
        let user_id = "user-123";
        let username = "alice";
        let email = "alice@gmail.com";

        let token =
            create_token(user_id, username, email, secret).expect("token creation should succeed");
        assert!(!token.is_empty(), "token should not be empty");

        let claims = verify_token(&token, secret).expect("token verification should succeed");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.username, username);
        assert_eq!(claims.email, email);
        assert!(claims.exp > 0, "expiry should be set");
    }

    #[test]
    fn test_expired_token() {
        use jsonwebtoken::{EncodingKey, Header, encode};

        let secret = "test-secret-key";
        let claims = Claims {
            sub: "user-123".to_string(),
            username: "alice".to_string(),
            email: "alice@gmail.com".to_string(),
            exp: 0,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("encoding should succeed");

        let result = verify_token(&token, secret);
        assert!(result.is_err(), "expired token should fail verification");
    }

    #[test]
    fn test_invalid_token() {
        let result = verify_token("not-a-valid-jwt-token", "test-secret-key");
        assert!(result.is_err(), "invalid token should fail verification");
    }
}
