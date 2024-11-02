use std::time;

use axum::{
    body::Body,
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};

use axum_messages::MessagesManagerLayer;
use chrono::Utc;
use reqwest::StatusCode;
use sqlx::{postgres::PgPoolOptions, PgPool, Pool, Postgres};
use tokio::net::TcpListener;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

use crate::{
    configuration::{DatabaseSettings, Settings},
    routers::{admin_dashboard, home, log_out, login, login_form, update_weather_data},
    weather_client::WeatherClient,
};

pub fn get_connection_pool(configuration: DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

async fn session_middleware(
    session: Session,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let utc_now = Utc::now();
    session
        .insert("last_activity_time", utc_now)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let response = next.run(request).await;
    Ok(response)
}

pub struct Application {
    port: u16,
    server: Server,
}

pub struct Server {
    listener: TcpListener,
    router: Router,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SessionData {
    pub user_id: String,
    pub create_timestamp: String,
    pub expire_time: i32,
}

#[derive(Clone)]
pub struct AppState {
    pub connect_pool: Pool<Postgres>,
    pub weather_client: WeatherClient,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connect_pool = get_connection_pool(configuration.database);
        let weather_client = configuration.weather_client.client();
        let shared_state = AppState {
            connect_pool,
            weather_client,
        };
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_name("session_id");
        let listener = TcpListener::bind(address).await?;
        let port = listener.local_addr().unwrap().port();
        let admin_router = Router::new()
            .route("/", get(|| async {}))
            .route("/dashboard", get(admin_dashboard))
            .route("/logout", post(log_out));
        let router = Router::new()
            .route("/", get(home))
            .route("/home", get(home))
            .route("/login", get(login_form).post(login))
            .nest("/admin", admin_router)
            .route("/update_weather", post(update_weather_data))
            .layer(middleware::from_fn(session_middleware))
            .layer(MessagesManagerLayer)
            .layer(session_layer)
            .with_state(shared_state);

        let server = Server { listener, router };
        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        axum::serve(self.server.listener, self.server.router).await
    }
}
