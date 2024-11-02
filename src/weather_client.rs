use std::num::ParseFloatError;

use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;
use tracing::info;

pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum CoordinateParseError {
    #[error("Coordinate is Foramt Failed")]
    Format,
    #[error("Coordinate is Parse Failed")]
    ParseFloat(ParseFloatError),
    #[error("Coordinate is Invalid Value")]
    InvalidValue,
}

impl Coordinate {
    pub fn parse(location: String) -> Result<Coordinate, CoordinateParseError> {
        let parts: Vec<&str> = location.split(",").collect();
        if parts.len() != 2 {
            return Err(CoordinateParseError::Format);
        };

        let latitude = parts[0]
            .trim()
            .parse::<f64>()
            .map_err(CoordinateParseError::ParseFloat)?;
        let longitude = parts[1]
            .trim()
            .parse::<f64>()
            .map_err(CoordinateParseError::ParseFloat)?;
        if (latitude < -90.0) || (latitude > 90.0) {
            return Err(CoordinateParseError::InvalidValue);
        } else if (longitude < -180.0) || (longitude > 180.0) {
            return Err(CoordinateParseError::InvalidValue);
        }

        Ok(Coordinate {
            latitude,
            longitude,
        })
    }
}

#[derive(Clone)]
pub struct WeatherClient {
    base_url: String,
    http_client: Client,
    authorization_token: SecretString,
}

impl WeatherClient {
    pub fn new(
        base_url: String,
        authorization_token: SecretString,
        timeout: std::time::Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            base_url,
            http_client,
            authorization_token,
        }
    }

    pub async fn get_weather_forecast(
        &self,
        location: &Coordinate,
    ) -> Result<Value, reqwest::Error> {
        let location = format!("{:.4},{:.4}", location.latitude, location.longitude);
        let url = format!(
            "{}/forecast?location={}&apikey={}",
            self.base_url,
            location,
            self.authorization_token.expose_secret()
        );
        let forecast_response = self
            .http_client
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await?
            .error_for_status()?;
        info!(
            location = &location,
            "forecast status: {}",
            forecast_response.status()
        );
        let forecast_json = forecast_response.json().await?;
        Ok(forecast_json)
    }
}
