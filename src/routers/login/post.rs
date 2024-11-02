use axum::debug_handler;
use axum::response::{IntoResponse, Redirect, Response};
use axum::{extract::State, Form};
use axum_extra::extract::CookieJar;
use axum_messages::Messages;
use secrecy::SecretBox;
use tower_sessions::Session;
use tracing::info;

use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::start_up::{AppState, SessionData};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretBox<String>,
}

#[debug_handler]
#[tracing::instrument(skip(state, cookie, session, messages, form), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn login(
    State(state): State<AppState>,
    cookie: CookieJar,
    session: Session,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Response, Redirect> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    let pool = state.connect_pool;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            let session_data = SessionData {
                user_id: user_id.to_string(),
                create_timestamp: chrono::Utc::now().to_string(),
                expire_time: 300,
            };
            match cookie.get("session_id") {
                Some(session_id) => {
                    let _ = session.insert(&session_id.to_string(), session_data).await;
                    Ok(Redirect::to("/admin/dashboard").into_response())
                }
                None => {
                    info!("User logging failed, invalid session_id");
                    Err(login_redirect(LoginError::Unauthorized, messages))
                }
            }
        }
        Err(error) => {
            let e = match error {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(error.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(error.into()),
            };
            info!("User logging failed, authentication failed");
            Err(login_redirect(e, messages))
        }
    }
}

fn login_redirect(e: LoginError, messages: Messages) -> Redirect {
    messages.error(format!(
        "<p><i>{}</i></p>",
        htmlescape::encode_minimal(&e.to_string())
    ));
    Redirect::to("/login")
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Unauthorized: Please log in")]
    Unauthorized,
}
