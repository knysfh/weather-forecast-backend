use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_messages::Messages;
use sqlx::PgPool;
use thiserror::Error;
use tower_sessions::{session, Session};
use uuid::Uuid;

use crate::{errors::DbError, routers::login::UserData, start_up::AppState};

#[derive(Error, Debug)]
pub enum DashboardError {
    #[error("Session not found")]
    SessionNotFound(String),
    #[error("Invalid session data")]
    InvalidSessionData(String),
    #[error(transparent)]
    DatabaseError(#[from] DbError),
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
    session: Session,
) -> Result<Response, Redirect> {
    let user_data: UserData = session
        .get("user.data")
        .await
        .map_err(|e| {
            DashboardError::InvalidSessionData(format!("Session query error, details: {}", e))
        })?
        .ok_or(DashboardError::SessionNotFound(format!(
            "User session data not found"
        )))?;

    let user_id = Uuid::parse_str(&user_data.user_id).map_err(DashboardError::UuidParseError)?;
    let user_name = user_data.user_name;
    let token = get_token_value(user_id, &state.connect_pool).await?;
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
pub async fn _get_username(user_id: Uuid, pool: &PgPool) -> Result<String, DashboardError> {
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
    .map_err(|e| DashboardError::DatabaseError(e.into()))?;
    Ok(row.username)
}

#[tracing::instrument(name = "Get token value", skip(pool))]
pub async fn get_token_value(user_id: Uuid, pool: &PgPool) -> Result<String, DashboardError> {
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
    .map_err(|e| DashboardError::DatabaseError(e.into()))?;
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
        .await
        .map_err(|e| DashboardError::DatabaseError(e.into()))?;
        Ok(token)
    }
}

pub async fn log_out(session: Session, messages: Messages) -> Result<Redirect, Redirect> {
    let value: Option<UserData> = session
        .remove("user.data")
        .await
        .map_err(|_| DashboardError::SessionNotFound("User session data not found".to_string()))?;
    session.cycle_id().await.unwrap();
    if value.is_some() {
        messages.info(format!(
            "<p><i>{}</i></p>",
            htmlescape::encode_minimal("You have successfully logged out.")
        ));
    }
    Ok(Redirect::to("/login"))
}
