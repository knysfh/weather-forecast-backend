use std::time::{Duration, SystemTime};

use anyhow::anyhow;
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Redirect, Response};
use axum::{extract::State, Form};
use axum_messages::Messages;
use reqwest::StatusCode;
use secrecy::SecretBox;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use tracing::info;

use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::start_up::AppState;

#[derive(Deserialize, Serialize)]
pub struct FailedLoginAttempt {
    pub failed_attempt_count: usize,
    pub last_attempt: SystemTime,
}

impl Default for FailedLoginAttempt {
    fn default() -> Self {
        FailedLoginAttempt {
            failed_attempt_count: 0,
            last_attempt: SystemTime::now(),
        }
    }
}

pub struct LoginInfo {
    session: Session,
    failed_info: FailedLoginAttempt,
}

impl LoginInfo {
    pub const LOGIN_INFO_KEY: &'static str = "user.login";

    pub async fn add_failed_count(&mut self) {
        self.failed_info.failed_attempt_count += 1;
        self.failed_info.last_attempt = SystemTime::now();
        self.update_login_failed_info(&self.session, &self.failed_info)
            .await
    }

    pub async fn flush_failed_info(&self) {
        let new_failed_info = FailedLoginAttempt::default();
        self.update_login_failed_info(&self.session, &new_failed_info)
            .await
    }

    pub async fn update_login_failed_info(
        &self,
        session: &Session,
        failed_info: &FailedLoginAttempt,
    ) {
        session
            .insert(Self::LOGIN_INFO_KEY, failed_info)
            .await
            .unwrap()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for LoginInfo
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(req, state).await?;

        let failed_info: FailedLoginAttempt = session
            .get(Self::LOGIN_INFO_KEY)
            .await
            .unwrap()
            .unwrap_or_default();

        Ok(Self {
            session,
            failed_info,
        })
    }
}

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: SecretBox<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UserData {
    pub user_id: String,
    pub user_name: String,
}

#[tracing::instrument(skip(state, login_info, session, messages, form), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn login(
    State(state): State<AppState>,
    session: Session,
    mut login_info: LoginInfo,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Response, Redirect> {
    if let Some(login_failed_info) = session
        .get::<FailedLoginAttempt>(LoginInfo::LOGIN_INFO_KEY)
        .await
        .unwrap()
    {
        if login_failed_info.failed_attempt_count > 3 {
            let elapsed = SystemTime::now()
                .duration_since(login_failed_info.last_attempt)
                .unwrap_or(Duration::from_secs(0));

            if elapsed < Duration::from_secs(300) {
                let max_attempt_err = LoginError::MaxAttemptsExceeded(anyhow!(
                    "You have exceeded the maximum number of error attempts"
                ));
                info!("User logging failed, maximum retry limit reached");
                return Err(login_redirect(max_attempt_err, messages));
            } else {
                login_info.flush_failed_info().await;
            }
        }
    };

    let credentials = Credentials {
        username: form.username.clone(),
        password: form.password,
    };
    let pool = state.connect_pool;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            let user_data = UserData {
                user_id: user_id.to_string(),
                user_name: form.username.to_string(),
            };
            session.insert("user.data", user_data).await.unwrap();
            let failed_info = FailedLoginAttempt::default();
            login_info
                .update_login_failed_info(&session, &failed_info)
                .await;
            Ok(Redirect::to("/admin/dashboard").into_response())
        }
        Err(error) => {
            let e = match error {
                AuthError::InvalidCredentials(_) => {
                    login_info.add_failed_count().await;
                    LoginError::AuthError(error.into())
                }
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(error.into()),
            };
            info!("User logging failed, authentication failed");
            Err(login_redirect(e, messages))
        }
    }
}

pub fn login_redirect(e: LoginError, messages: Messages) -> Redirect {
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
    #[error("Maximum login attempts exceeded. Please try again later")]
    MaxAttemptsExceeded(#[source] anyhow::Error),
}
