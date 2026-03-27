use crate::errors::AppError;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use password_hash::SaltString;
use rand_core::OsRng;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("password hashing failed: {e}")))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("invalid password hash: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn create_token(user_id: &str, username: &str, secret: &str) -> Result<String, AppError> {
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
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
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
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
            let header = auth_header
                .ok_or_else(|| AppError::Unauthorized("missing authorization header".to_string()))?;

            let token = header
                .strip_prefix("Bearer ")
                .ok_or_else(|| AppError::Unauthorized("invalid authorization header format".to_string()))?;

            let claims = verify_token(token, &jwt_secret)?;

            Ok(AuthUser {
                user_id: claims.sub,
                username: claims.username,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let password = "securepassword123";
        let hash = hash_password(password).expect("hashing should succeed");

        // Hash should not be the plaintext password
        assert_ne!(hash, password);

        // Verification with correct password should return true
        let result = verify_password(password, &hash).expect("verify should succeed");
        assert!(result, "correct password should verify as true");
    }

    #[test]
    fn test_verify_wrong_password() {
        let hash = hash_password("correctpassword").expect("hashing should succeed");

        let result = verify_password("wrongpassword", &hash).expect("verify should succeed");
        assert!(!result, "wrong password should verify as false");
    }

    #[test]
    fn test_create_and_decode_token() {
        let secret = "test-secret-key";
        let user_id = "user-123";
        let username = "alice";

        let token = create_token(user_id, username, secret).expect("token creation should succeed");
        assert!(!token.is_empty(), "token should not be empty");

        let claims = verify_token(&token, secret).expect("token verification should succeed");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.username, username);
        assert!(claims.exp > 0, "expiry should be set");
    }

    #[test]
    fn test_expired_token() {
        use jsonwebtoken::{encode, EncodingKey, Header};

        let secret = "test-secret-key";
        let claims = Claims {
            sub: "user-123".to_string(),
            username: "alice".to_string(),
            exp: 0, // epoch = already expired
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
