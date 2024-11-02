use axum::response::Response;
use axum::{body::Body, response::IntoResponse};

use reqwest::StatusCode;

pub async fn home() -> impl IntoResponse {
    let html_content = r#"<!DOCTYPE html>
<html lang="en">

<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Home</title>
</head>

<body>
    <p>Welcome to home page!</p>
    <p>You should <a href="/login">login</a></p>
</body>

</html>"#
        .to_owned();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::new(html_content))
        .unwrap()
}
