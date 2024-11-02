use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use axum_messages::Messages;
use sqlx::PgPool;
use thiserror::Error;
use tower_sessions::{session, Session};
use uuid::Uuid;

use crate::start_up::{AppState, SessionData};

#[derive(Error, Debug)]
pub enum DashboardError {
    #[error("Session not found")]
    SessionNotFound,
    #[error("Invalid session data")]
    InvalidSessionData,
    #[error("Database error: {0}")]
    DatabaseError(#[from] anyhow::Error),
    #[error("Invalid UUID: {0}")]
    UuidParseError(#[from] uuid::Error),
}

impl From<DashboardError> for Redirect {
    fn from(_value: DashboardError) -> Self {
        Redirect::to("/login")
    }
}

pub async fn admin_dashboard(
    State(state): State<AppState>,
    cookie: CookieJar,
    session: Session,
) -> Result<Response, Redirect> {
    let session_id = cookie
        .get("session_id")
        .ok_or(DashboardError::SessionNotFound)?;

    let session_data: SessionData = session
        .get(&session_id.to_string())
        .await
        .map_err(|_| DashboardError::InvalidSessionData)?
        .ok_or(DashboardError::SessionNotFound)?;

    let user_id = Uuid::parse_str(&session_data.user_id).map_err(DashboardError::UuidParseError)?;

    let user_name = get_username(user_id, &state.connect_pool)
        .await
        .map_err(DashboardError::DatabaseError)?;
    let token = get_token_value(user_id, &state.connect_pool)
        .await
        .map_err(DashboardError::DatabaseError)?;
    Ok(render_dashboard(&user_name, &token).into_response())
}

fn render_dashboard(user_name: &str, token: &str) -> Html<String> {
    Html(
        format!(
            r#"<!DOCTYPE html>
<html lang="en">

<head>
<meta http-equiv="content-type" content="text/html; charset=utf-8">
<title>Admin dashboard</title>
</head>

<body>
<p>Welcome {}!</p>
<p>Available actions:</p>
<ol>
<li>
    <form name="logoutForm" action="/admin/logout" method="post">
        <input type="submit" value="Logout">
    </form>
</li>
<li>
    <p>Your token:{}</p>    
</li>    
</ol>
</body>

</html>"#,
            user_name, token
        )
        .to_string(),
    )
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;
    Ok(row.username)
}

#[tracing::instrument(name = "Get token value", skip(pool))]
pub async fn get_token_value(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT token
        FROM tokens
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;
    if let Some(row) = row {
        Ok(row.token)
    } else {
        let token = format!("{}", session::Id::default());
        sqlx::query!(
            r#"
            INSERT INTO tokens (user_id, token)
            VALUES ($1, $2)
            "#,
            user_id,
            token
        )
        .execute(pool)
        .await?;
        Ok(token)
    }
}

pub async fn log_out(
    cookie: CookieJar,
    session: Session,
    messages: Messages,
) -> Result<Redirect, Redirect> {
    let session_id = cookie
        .get("session_id")
        .ok_or(DashboardError::SessionNotFound)?
        .to_string();
    let value: Option<SessionData> = session
        .remove(&session_id.to_string())
        .await
        .map_err(|_| DashboardError::SessionNotFound)?;
    if value.is_some() {
        messages.info(format!(
            "<p><i>{}</i></p>",
            htmlescape::encode_minimal("You have successfully logged out.")
        ));
    }
    Ok(Redirect::to("/login"))
}
