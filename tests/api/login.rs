use reqwest::header;

use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_messgae_is_set_on_failure() {
    let app = spawn_app().await;

    let login_body =
        serde_json::json!({"username": "random_username", "password": "random_password"});
    let response = app.post_login(&login_body).await;

    assert_eq!(response.status().as_u16(), 303);
    assert_is_redirect_to(&response, "/login");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#))
}

#[tokio::test]
async fn successed_login_with_session_id_cookie() {
    let app = spawn_app().await;
    let _ = app.get_login_html().await;
    let response = app.test_user.login(&app).await;

    assert_eq!(response.status().as_u16(), 303);
    assert_is_redirect_to(&response, "/login");

    let cookie_header = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("Failed to parse cookie.");
    assert!(cookie_header.to_str().unwrap().contains("session_id"));
}
