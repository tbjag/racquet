use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub google_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    pub id: String,
    pub room_id: String,
    pub user_id: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageWithUsername {
    pub id: String,
    pub room_id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub created_at: String,
}

pub async fn upsert_user_by_email(
    pool: &SqlitePool,
    id: &str,
    email: &str,
    username: &str,
    google_id: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, email, username, google_id) VALUES (?, ?, ?, ?) \
         ON CONFLICT(email) DO UPDATE SET username = excluded.username, google_id = excluded.google_id",
    )
    .bind(id)
    .bind(email)
    .bind(username)
    .bind(google_id)
    .execute(pool)
    .await?;

    find_user_by_email(pool, email)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

pub async fn find_user_by_email(
    pool: &SqlitePool,
    email: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, username, google_id, created_at FROM users WHERE email = ?",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

pub async fn update_username(
    pool: &SqlitePool,
    user_id: &str,
    username: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET username = ? WHERE id = ?")
        .bind(username)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_user_by_id(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, username, google_id, created_at FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn insert_room(pool: &SqlitePool, id: &str, name: &str, created_by: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO rooms (id, name, created_by) VALUES (?, ?, ?)")
        .bind(id)
        .bind(name)
        .bind(created_by)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_rooms(pool: &SqlitePool) -> Result<Vec<Room>, sqlx::Error> {
    sqlx::query_as::<_, Room>("SELECT id, name, created_by, created_at FROM rooms ORDER BY created_at ASC")
        .fetch_all(pool)
        .await
}

pub async fn insert_message(pool: &SqlitePool, id: &str, room_id: &str, user_id: &str, content: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO messages (id, room_id, user_id, content) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(room_id)
        .bind(user_id)
        .bind(content)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_messages(pool: &SqlitePool, room_id: &str, before: Option<&str>, limit: i64) -> Result<Vec<MessageWithUsername>, sqlx::Error> {
    let limit = limit.min(100).max(1);

    match before {
        Some(before_id) => {
            sqlx::query_as::<_, MessageWithUsername>(
                "SELECT m.id, m.room_id, m.user_id, u.username, m.content, m.created_at \
                 FROM messages m JOIN users u ON m.user_id = u.id \
                 WHERE m.room_id = ? AND m.rowid < (SELECT rowid FROM messages WHERE id = ?) \
                 ORDER BY m.rowid DESC LIMIT ?"
            )
            .bind(room_id)
            .bind(before_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, MessageWithUsername>(
                "SELECT m.id, m.room_id, m.user_id, u.username, m.content, m.created_at \
                 FROM messages m JOIN users u ON m.user_id = u.id \
                 WHERE m.room_id = ? \
                 ORDER BY m.rowid DESC LIMIT ?"
            )
            .bind(room_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}

pub async fn find_room_by_id(pool: &SqlitePool, room_id: &str) -> Result<Option<Room>, sqlx::Error> {
    sqlx::query_as::<_, Room>("SELECT id, name, created_by, created_at FROM rooms WHERE id = ?")
        .bind(room_id)
        .fetch_optional(pool)
        .await
}
