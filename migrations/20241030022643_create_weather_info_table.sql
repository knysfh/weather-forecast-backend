-- Add migration script here
CREATE TABLE weather_info (
    id uuid PRIMARY KEY,
    user_id uuid REFERENCES users (user_id),
    -- 地理信息
    latitude FLOAT NOT NULL,
    longitude FLOAT NOT NULL,
    city_name VARCHAR(100),
    -- 降水相关指标
    precipitation_probability FLOAT,
    sleet_intensity FLOAT,
    snow_intensity FLOAT,
    -- 温度相关指标
    temperature FLOAT,
    temperature_apparent FLOAT,
    -- 风速相关指标
    wind_speed FLOAT,
    -- 时间
    forecast_time TIMESTAMP NOT NULL,
    -- 元数据,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, forecast_time, latitude, longitude)
);