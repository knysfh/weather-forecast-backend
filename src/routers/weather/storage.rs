use chrono::{DateTime, ParseError, Utc};
use serde::Deserialize;
use serde_json::Value;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::weather_client::Coordinate;

#[derive(Deserialize, Debug)]
struct WeatherForecastResponse {
    timelines: Timelines,
}
#[derive(Deserialize, Debug)]
struct Timelines {
    hourly: Vec<WeatherData>,
}
#[derive(Deserialize, Debug)]
struct WeatherData {
    time: DateTime<Utc>,
    values: WeatherValues,
}
#[derive(Deserialize, Debug)]
struct WeatherValues {
    #[serde(rename = "precipitationProbability")]
    precipitation_probability: f64,
    #[serde(rename = "sleetIntensity")]
    sleet_intensity: f64,
    #[serde(rename = "snowIntensity")]
    snow_intensity: f64,
    temperature: f64,
    #[serde(rename = "temperatureApparent")]
    temperature_apparent: f64,
    #[serde(rename = "windSpeed")]
    wind_speed: f64,
}

struct WeatherInfoData {
    user_id: Uuid,
    latitude: f64,
    longitude: f64,
    city_name: String,
    precipitation_probability: f64,
    sleet_intensity: f64,
    snow_intensity: f64,
    temperature: f64,
    temperature_apparent: f64,
    wind_speed: f64,
    forecast_time: DateTime<Utc>,
}

#[derive(Error, Debug)]
pub enum ForecastParseError {
    #[error("Invalid time format: {0}")]
    TimeParseError(#[from] ParseError),
    #[error("Weather data into databse error: {0}")]
    WeatherDataIntoDatabaseError(#[from] sqlx::Error),
    #[error("Json data parse Error: {0}")]
    JsonParseError(#[from] serde_json::Error),
}

#[tracing::instrument(
    name = "Parse forecast data",
    skip(json_data, location, city_name, pool)
)]
pub async fn parse_forecast_data(
    json_data: Value,
    location: &Coordinate,
    city_name: String,
    user_id: &Uuid,
    pool: &PgPool,
) -> Result<(), ForecastParseError> {
    let forecast_data: WeatherForecastResponse =
        serde_json::from_value(json_data).map_err(|e| ForecastParseError::JsonParseError(e))?;
    for weather_data in forecast_data.timelines.hourly {
        let weather_info_data = WeatherInfoData {
            user_id: *user_id,
            latitude: location.latitude,
            longitude: location.longitude,
            city_name: city_name.clone(),
            precipitation_probability: weather_data.values.precipitation_probability,
            sleet_intensity: weather_data.values.sleet_intensity,
            snow_intensity: weather_data.values.snow_intensity,
            temperature: weather_data.values.temperature,
            temperature_apparent: weather_data.values.temperature_apparent,
            wind_speed: weather_data.values.wind_speed,
            forecast_time: weather_data.time,
        };
        save_weather_data(weather_info_data, pool).await?;
    }
    Ok(())
}

#[tracing::instrument(name = "Save weather data", skip(data, pool))]
async fn save_weather_data(data: WeatherInfoData, pool: &PgPool) -> Result<(), ForecastParseError> {
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO weather_info
            (id, user_id, latitude, longitude, city_name, precipitation_probability, sleet_intensity,snow_intensity,temperature,temperature_apparent,wind_speed,forecast_time)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (user_id, forecast_time, latitude, longitude) DO UPDATE 
            SET 
                precipitation_probability = $6,
                sleet_intensity = $7,
                snow_intensity = $8,
                temperature = $9,
                temperature_apparent = $10,
                wind_speed = $11
        "#,
        id,
        data.user_id,
        data.latitude,
        data.longitude,
        data.city_name,
        data.precipitation_probability,
        data.sleet_intensity,
        data.snow_intensity,
        data.temperature,
        data.temperature_apparent,
        data.wind_speed,
        data.forecast_time.naive_utc(),
    ).execute(pool).await.map_err(|e| {tracing::error!(
        error = %e, 
        "Database insertion failed"
    );ForecastParseError::WeatherDataIntoDatabaseError(e)})?;
    Ok(())
}
