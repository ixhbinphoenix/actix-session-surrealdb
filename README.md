# actix-session-surrealdb

A library for [actix-session](https://github.com/actix/actix-extras/tree/master/actix-session) which implements the `SessionStore` trait for [SurrealDB](https://surrealdb.com).

NOT AN OFFICIAL LIBRARY FROM [`actix`](https://github.com/actix)!

## Support
This library was built for a Project of mine, and as such, is only tested for my specific use-case. I will try to support other use-cases, so please open Issues for errors you encounter,
but this is more "best-effort" support than full.

This library is not tested very well, so use at your own risk. You also probably shouldn't use this in your production, but I'm not your Boss so do as you wish.

Minimum Supported Rust Version (MSRV): `1.75`

## Example Usage

You can use the `SurrealSessionStore` similarly to the `CookieSessionStore` or `RedisSessionStore`, but you'll have to connect and check your database first
```rust
#[actix_web::main]
async fn main() -> io::Result<()> {
    let db = Surreal::new<Ws>("127.0.0.1:8000").await.unwrap();

    db.signin({
        user: "root",
        pass: "root",
    }).await.unwrap();
    
    db.use_ns("test").use_db("test").await.unwrap();

    let session_store = SurrealSessionStore::from_connection(db, "sessions");

    let key = Key::generate();

    HttpServer::new(move || {
        App::new()
            .wrap(
                SessionMiddleware::builder(session_store.clone(), key.clone())
                   .cookie_same_site(actix_web::cookie::SameSite::None)
                   .cookie_secure(true)
                   .cookie_http_only(true)
                   .session_lifecycle(
                        PersistentSession::default()
                            .session_ttl_policy(actix_session::config::TtlExtensionPolicy::OnStateChanges)
                            .session_ttl(Duration::days(7)),
                   )
                   .build()
            )
    })
    .bind(("127.0.0.1", "8080"))?
    .run()
    .await
}
```

## License

This Project is licensed under the MIT License which can be found [here](./LICENSE).
