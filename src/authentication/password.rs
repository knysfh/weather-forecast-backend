use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, SecretBox};
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

use crate::{errors::DbError, telemetry::spawn_blocking_with_tracing};

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(String),
    #[error(transparent)]
    DatabaseError(#[from] DbError),
    #[error("unexpected error")]
    UnexpectedError(String),
}

pub struct Credentials {
    pub username: String,
    pub password: SecretBox<String>,
}

pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = SecretBox::<String>::new(Box::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
    gZiV/M1gPc22ElAH/Jh1Hw$\
    CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    ));

    if let Some((stored_user_id, stored_password_hash)) =
        get_user_password(&credentials.username, pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    };

    spawn_blocking_with_tracing(|| {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .map_err(|e| {
        AuthError::UnexpectedError(format!(
            "An exception occurred in the thread while verifying the hashed password, details: {}",
            e
        ))
    })??;

    user_id.ok_or_else(|| {
        AuthError::UnexpectedError("An error occurred while getting the user_id".to_string())
    })
}

pub async fn get_user_password(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(Uuid, SecretBox<String>)>, AuthError> {
    let row = sqlx::query!(
        r#"
            SELECT user_id, password_hash
            FROM users
            WHERE username = $1
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AuthError::DatabaseError(e.into()))?
    .map(|row| (row.user_id, SecretBox::new(Box::new(row.password_hash))));
    Ok(row)
}

fn verify_password_hash(
    expected_password_hash: SecretBox<String>,
    password_candidate: SecretBox<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .map_err(|e| {
            AuthError::UnexpectedError(format!(
                "Error during password hashing, details: {}",
                e.to_string()
            ))
        })?;
    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .map_err(|_| AuthError::InvalidCredentials(format!("Incorrect username or password.",)))
}
