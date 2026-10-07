//! Accounts and session tokens in SQLite. Tokens are stored as SHA-256 hashes so a
//! leaked database does not leak usable sessions.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rusqlite::{params, Connection, ErrorCode, OptionalExtension};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum AuthError {
    UsernameTaken,
    BadCredentials,
    Internal(String),
}

impl From<rusqlite::Error> for AuthError {
    fn from(e: rusqlite::Error) -> Self {
        AuthError::Internal(e.to_string())
    }
}

/// Cheap to clone; every call runs on the blocking thread pool.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS users (
                 id INTEGER PRIMARY KEY,
                 username TEXT NOT NULL UNIQUE COLLATE NOCASE,
                 password_hash TEXT NOT NULL,
                 created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS tokens (
                 token_hash TEXT PRIMARY KEY,
                 user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                 created_at INTEGER NOT NULL
             );",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Creates the account and returns a first session token.
    pub async fn register(&self, username: String, password: String) -> Result<String, AuthError> {
        let password_hash = blocking(move || hash_password(&password)).await??;
        let db = self.clone();
        blocking(move || {
            let conn = db.lock();
            let inserted = conn.execute(
                "INSERT INTO users (username, password_hash, created_at) VALUES (?1, ?2, ?3)",
                params![username, password_hash, now_secs()],
            );
            match inserted {
                Err(rusqlite::Error::SqliteFailure(e, _))
                    if e.code == ErrorCode::ConstraintViolation =>
                {
                    Err(AuthError::UsernameTaken)
                }
                Err(e) => Err(e.into()),
                Ok(_) => issue_token(&conn, conn.last_insert_rowid()),
            }
        })
        .await?
    }

    /// Checks the password and returns `(canonical username, new token)`.
    pub async fn login(
        &self,
        username: String,
        password: String,
    ) -> Result<(String, String), AuthError> {
        let db = self.clone();
        let user = blocking(move || {
            db.lock()
                .query_row(
                    "SELECT id, username, password_hash FROM users WHERE username = ?1",
                    params![username],
                    |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()
        })
        .await??;
        let Some((user_id, username, stored)) = user else {
            return Err(AuthError::BadCredentials);
        };
        if !blocking(move || verify_password(&password, &stored)).await? {
            return Err(AuthError::BadCredentials);
        }
        let db = self.clone();
        let token = blocking(move || issue_token(&db.lock(), user_id)).await??;
        Ok((username, token))
    }

    /// Resolves a session token to its username.
    pub async fn user_for_token(&self, token: String) -> Result<Option<String>, AuthError> {
        let db = self.clone();
        Ok(blocking(move || {
            db.lock()
                .query_row(
                    "SELECT u.username FROM tokens t JOIN users u ON u.id = t.user_id
                     WHERE t.token_hash = ?1",
                    params![hash_token(&token)],
                    |r| r.get(0),
                )
                .optional()
        })
        .await??)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A panic mid-query cannot leave the connection itself inconsistent.
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AuthError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))
}

fn issue_token(conn: &Connection, user_id: i64) -> Result<String, AuthError> {
    let token = hex::encode(random_bytes::<32>()?);
    conn.execute(
        "INSERT INTO tokens (token_hash, user_id, created_at) VALUES (?1, ?2, ?3)",
        params![hash_token(&token), user_id, now_secs()],
    )?;
    Ok(token)
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::encode_b64(&random_bytes::<16>()?)
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AuthError::Internal(e.to_string()))
}

fn verify_password(password: &str, stored: &str) -> bool {
    PasswordHash::new(stored).is_ok_and(|h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}

/// Bytes from the OS CSPRNG.
pub fn random_bytes<const N: usize>() -> Result<[u8; N], AuthError> {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(buf)
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
