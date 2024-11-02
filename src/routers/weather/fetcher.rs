use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::start_up::AppState;
use crate::weather_client::Coordinate;
use crate::weather_client::CoordinateParseError;

use super::storage::parse_forecast_data;
use super::storage::ForecastParseError;

#[derive(Deserialize)]
pub struct WeatherRequestInfo {
    token: String,
    location: String,
    city_name: String,
}

#[derive(Serialize)]
pub struct WeatherResponse {
    status: String,
    content: String,
}

#[derive(Error, Debug)]
pub enum UpdateWeatherError {
    #[error("Invalid JSON format: {0}")]
    UserPostJsonError(#[from] JsonRejection),
    #[error("Validation error: {0}")]
    UserValidationError(String),
    #[error("Internal server error")]
    InternalError,
    #[error("Invalid Location format: {0}")]
    LocationError(#[from] CoordinateParseError),
    #[error("Request weather server error: {0}")]
    WeatherServerError(#[from] reqwest::Error),
    #[error("Forecast parse error: {0}")]
    ForecastWriteError(#[from] ForecastParseError),
}

impl IntoResponse for UpdateWeatherError {
    fn into_response(self) -> axum::response::Response {
        let (status_code, status, content) = match &self {
            UpdateWeatherError::UserPostJsonError(json_rejection) => {
                let content_message = match json_rejection {
                    JsonRejection::JsonDataError(_) => "Invalid JSON data format",
                    JsonRejection::JsonSyntaxError(_) => "JSON syntax error",
                    JsonRejection::MissingJsonContentType(_) => {
                        "Missing content-type: application/json header"
                    }
                    _ => "Unknown JSON error",
                };
                (StatusCode::BAD_REQUEST, "JSON_ERROR", content_message)
            }
            UpdateWeatherError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "An internal server error occurred",
            ),
            UpdateWeatherError::UserValidationError(msg) => {
                (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.as_str())
            }
            UpdateWeatherError::LocationError(location_parse) => {
                let content_message = match location_parse {
                    CoordinateParseError::Format => "Location format error",
                    CoordinateParseError::ParseFloat(_) => "Location parse number error",
                    CoordinateParseError::InvalidValue => "Location range error",
                };
                (StatusCode::BAD_REQUEST, "JSON_ERROR", content_message)
            }
            UpdateWeatherError::WeatherServerError(_) => (
                StatusCode::BAD_REQUEST,
                "WEATHER_SERVER_ERROR",
                "Weather server error",
            ),
            UpdateWeatherError::ForecastWriteError(error) => match error {
                ForecastParseError::TimeParseError(_) => (
                    StatusCode::BAD_REQUEST,
                    "TIME_PARSE_ERROR",
                    "Forecast time format parse error",
                ),
                ForecastParseError::WeatherDataIntoDatabaseError(_) => (
                    StatusCode::BAD_REQUEST,
                    "WEATHER_DATABASE_ERROR",
                    "Weather data write database error",
                ),
                ForecastParseError::JsonParseError(_) => (
                    StatusCode::BAD_REQUEST,
                    "JSON_ERROR",
                    "Weather data json parse error",
                ),
            },
        };

        let body = Json(WeatherResponse {
            status: status.to_string(),
            content: content.to_string(),
        });

        (status_code, body).into_response()
    }
}

pub async fn update_weather_data(
    State(state): State<AppState>,
    weather_request: Result<Json<WeatherRequestInfo>, JsonRejection>,
) -> Result<Json<WeatherResponse>, UpdateWeatherError> {
    let Json(request) = weather_request.map_err(UpdateWeatherError::UserPostJsonError)?;

    let user_token = request.token;
    let validate_bool = validate_token(&user_token, &state.connect_pool)
        .await
        .map_err(|e| UpdateWeatherError::UserValidationError(e.to_string()))?;
    let mut weather_response = WeatherResponse {
        status: "SUCCESS_UPDATE".to_owned(),
        content: "Success update weather info".to_owned(),
    };
    if !validate_bool {
        weather_response.content = "Permission error, please log in again".to_owned();
        weather_response.status = "FAILED_UPDATE".to_owned();
        return Ok(Json(weather_response));
    }
    let location =
        Coordinate::parse(request.location).map_err(UpdateWeatherError::LocationError)?;
    let city_name = request.city_name;
    let forecast_value = state
        .weather_client
        .get_weather_forecast(&location)
        .await
        .map_err(|err| UpdateWeatherError::WeatherServerError(err))?;
    let user_id = get_user_id_by_token(&state.connect_pool, &user_token).await?;
    dbg!(&user_id);
    let _ = parse_forecast_data(
        forecast_value,
        &location,
        city_name,
        &user_id,
        &state.connect_pool,
    )
    .await
    .map_err(|err| UpdateWeatherError::ForecastWriteError(err))?;
    Ok(Json(weather_response))
}

#[tracing::instrument(name = "Update weather validate token", skip(token, pool))]
async fn validate_token(token: &str, pool: &PgPool) -> Result<bool, anyhow::Error> {
    let row = sqlx::query!(
        r#"SELECT EXISTS(SELECT 1 FROM tokens WHERE token = $1)"#,
        token
    )
    .fetch_one(pool)
    .await?;
    Ok(row.exists.unwrap_or(false))
}

pub async fn get_user_id_by_token(pool: &PgPool, token: &str) -> Result<Uuid, UpdateWeatherError> {
    let user_id = sqlx::query_scalar!("SELECT user_id FROM tokens WHERE token = $1", token)
        .fetch_optional(pool)
        .await
        .map_err(|e| UpdateWeatherError::UserValidationError(e.to_string()))?
        .ok_or(UpdateWeatherError::UserValidationError(
            "Uuid does not exist".to_string(),
        ))?;

    Ok(user_id)
}
