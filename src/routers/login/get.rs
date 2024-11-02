use axum::response::IntoResponse;
use axum::response::Response;
use axum_messages::Messages;
use reqwest::StatusCode;
use std::fmt::Write;

pub async fn login_form(flash_message: Messages) -> impl IntoResponse {
    let mut error_html = String::new();
    for fm in flash_message {
        writeln!(error_html, "<p><i>{}</i></p>", fm.message).unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">

<head>
<meta http-equiv="content-type" content="text/html; charset=utf-8">
<title>Login</title>
</head>

<body>
{error_html}
<form action="/login" method="post">
<label>Username
<input type="text" placeholder="Enter Username" name="username">
</label>
<label>Password
<input type="password" placeholder="Enter Password" name="password">
</label>
<button type="submit">Login</button>
</form>
</body>

</html>"#,
        ))
        .unwrap()
}
