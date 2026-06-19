use crate::*;

#[test]
fn stale_tabby_retry_message_is_ignored() {
    let (mut app, _) = App::new();
    app.tabby_retry_generation = 10;
    app.status = "keep".into();

    let _ = app.update(Message::FetchTabbyModelsRetry(9));

    assert_eq!(app.status, "keep");
    assert_eq!(app.tabby_retry_generation, 10);
}

#[test]
fn manual_tabby_fetch_bumps_retry_generation() {
    let (mut app, _) = App::new();
    app.tabby_retry_generation = 7;
    app.tabby_url_input = "http://localhost:5000".into();

    let _ = app.update(Message::FetchTabbyModels);

    assert_eq!(app.tabby_retry_generation, 8);
}

#[test]
fn tabby_url_change_invalidates_retry_generation() {
    let (mut app, _) = App::new();
    app.tabby_retry_generation = 11;
    app.tabby_connect_retry_left = 2;

    let _ = app.update(Message::TabbyUrlChanged(
        "http://localhost:5001".to_string(),
    ));

    assert_eq!(app.tabby_retry_generation, 12);
    assert_eq!(app.tabby_connect_retry_left, 0);
}

#[test]
fn tabby_token_change_invalidates_retry_generation() {
    let (mut app, _) = App::new();
    app.tabby_retry_generation = 5;
    app.tabby_connect_retry_left = 1;

    let _ = app.update(Message::TabbyTokenChanged("secret".to_string()));

    assert_eq!(app.tabby_retry_generation, 6);
    assert_eq!(app.tabby_connect_retry_left, 0);
}

#[test]
fn clear_tabby_invalidates_retry_generation() {
    let (mut app, _) = App::new();
    app.tabby_retry_generation = 3;
    app.tabby_connect_retry_left = 2;
    app.tabby_url_input = "http://localhost:5000".into();
    app.tabby_token_input = "secret".into();

    let _ = app.update(Message::ClearTabby);

    assert_eq!(app.tabby_retry_generation, 4);
    assert_eq!(app.tabby_connect_retry_left, 0);
    assert!(app.tabby_url_input.is_empty());
    assert!(app.tabby_token_input.is_empty());
}
