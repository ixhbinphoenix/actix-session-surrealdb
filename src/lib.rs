//! An actix_session [`actix_session::storage::SessionStore`] for [`surrealdb`]
//!
//! This crate implements the [`actix_session::storage::SessionStore`] for [`surrealdb`] allowing
//! you to Store session state in a [`surrealdb`] database
//!
//! ## Example
//! The [`SurrealSessionStore`] can be used just like any other [`actix_session::storage::SessionStore`]
//! with the difference that it needs an already connected DBConnection
//!
//! ```
//! use actix_session::{config::PersistentSession, SessionMiddleware};
//! use actix_session_surrealdb::SurrealSessionStore;
//! use actix_web::{cookie::{time::Duration, Key}, App, HttpServer};
//! use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};
//!
//! #[actix_web::main]
//! async fn main -> io::Result<()> {
//!     let db = Surreal::new::<Ws>("127.0.0.1:8000").await.expect("DB to connect");
//!
//!     db.signin(Root {
//!         username: "root",
//!         password: "root"
//!     })
//!     .await
//!     .expect("DB Credentials to be correct");
//!
//!     db.use_ns("test").use_db("test").await.unwrap();
//!
//!     let key = Key::generate();
//!
//!     HttpServer::new(move || {
//!         App::new()
//!             .wrap(
//!                 SessionMiddleware::builder(
//!                     SurrealSessionStore::from_connection(db.clone(), "sessions"),
//!                     key.clone()
//!                 )
//!                 .cookie_same_site(actix_web::cookie::SameSite::None)
//!                 .cookie_secure(true)
//!                 .cookie_http_only(true)
//!                 .session_lifecycle(
//!                     PersistentSession::default()
//!                         .session_ttl_extension_policy(actix_session::config::TtlExtensionPolicy::OnStateChanges)
//!                         .session_ttl(Duration::days(7)),
//!                 )
//!                 .build(),
//!             )
//!     })
//!     .bind(("127.0.0.1", "8080"))?
//!     .run()
//!     .await
//! }
//! ```


mod dates;
mod session_key;

use std::collections::HashMap;

use actix_session::storage::{LoadError, SaveError, SessionKey, SessionStore, UpdateError};
use actix_web::cookie::time::Duration;
use anyhow::{anyhow, Error};
use chrono::{DateTime, Utc};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use session_key::generate_session_key;
use surrealdb::{
    Surreal, engine::remote::ws::Client, types::SurrealValue
};

use crate::dates::add_duration_to_current;

/// SurrealDB Database Connection
pub type DBConnection = Surreal<Client>;

#[derive(Clone)]
pub struct SurrealSessionStore {
    client: DBConnection,
    tb: String,
}

impl SurrealSessionStore {
    /// Creates a SurrealSessionStore from an existing and logged in connection
    ///
    /// Takes the [DBConnection] and the database table to be used as args
    ///
    /// This function does NOT check for signin status, namespace or database. It also doesn't
    /// error if one of these are set up wrong.
    /// Ensure that these are set correctly before using it.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// #[actix_web::main]
    /// async fn main() -> io::Result<()> {
    ///     let db = Surreal::new("ws://127.0.0.1:8000").await.unwrap();
    ///
    ///     db.signin({
    ///         username: "root",
    ///         password: "root"
    ///     }).await.unwrap();
    ///
    ///     db.use_ns("test").use_db("test").await.unwrap();
    ///
    ///     let session_store = SurrealSessionStore::from_connection(db, "sessions");
    /// }
    /// ```
    pub fn from_connection(db: DBConnection, tb: &str) -> SurrealSessionStore {
        SurrealSessionStore {
            client: db,
            tb: tb.to_owned(),
        }
    }
}

pub(crate) type SessionState = HashMap<String, String>;

/// Database record for the session tokens
#[derive(Debug, Serialize, Deserialize, SurrealValue)]
pub(crate) struct KeyRecord {
    token: String,
    expiry: surrealdb::types::Datetime,
}

#[derive(Debug, Serialize, Deserialize, SurrealValue)]
pub(crate) struct KeyRecordPatch {
    token: Option<String>,
    expiry: Option<surrealdb::types::Datetime>,
}

impl SessionStore for SurrealSessionStore {
    async fn load(&self, session_key: &SessionKey) -> Result<Option<SessionState>, LoadError> {
        debug!("Loading sessionstate from db..");
        /* let thingy = Thing {
            tb: self.tb.clone(),
            id: Id::String(session_key.as_ref().to_owned()),
        }; */
        let thingy = (self.tb.clone(), session_key.as_ref().to_owned());

        let res: Result<Option<KeyRecord>, surrealdb::Error> = self.client.select(thingy.clone()).await;

        if res.is_err() {
            error!("Reading database record failed!\n{}", res.unwrap_err());
            return Err(LoadError::Other(anyhow!("Reading database record failed!")));
        }

        let record_opt = res.unwrap();
        if record_opt.is_none() {
            debug!("Reading database record failed! (Option is none)");
            return Ok(None);
        }

        let record = record_opt.unwrap();

        if record.expiry.timestamp_millis() < Utc::now().timestamp_millis() {
            let _: Option<KeyRecord> = self.client.delete(thingy).await.expect("Deleting database record failed!");
            return Ok(None);
        }

        serde_json::from_str(&record.token)
            .map_err(|e| {
                error!("Deserialization failed: {e:?} \n token was: {}", &record.token);
                e
            })
            .map_err(Into::into)
            .map_err(LoadError::Deserialization)
    }

    async fn save(&self, session_state: SessionState, ttl: &Duration) -> Result<SessionKey, SaveError> {
        let body = serde_json::to_string(&session_state).map_err(Into::into).map_err(SaveError::Serialization)?;
        let session_key = generate_session_key();
        let id = session_key.as_ref().to_owned();

        let expiry_time: DateTime<Utc> = match add_duration_to_current(ttl) {
            Some(a) => a,
            None => {
                return Err(SaveError::Other(anyhow!("Invalid duration length!")));
            }
        };

        let res: Result<Option<KeyRecord>, surrealdb::Error> = self
            .client
            .create((self.tb.clone(), id))
            .content(KeyRecord {
                token: body,
                expiry: expiry_time.into(),
            })
            .await;

        // eprintln!("SURREAL SESSION SAVE RESULT: {res:?}");

        match res {
            Err(e) => return Err(SaveError::Other(anyhow!("Failed to create database record: {e}"))),
            Ok(None) => return Err(SaveError::Other(anyhow!("Create returned None!"))),
            Ok(Some(_)) => return Ok(session_key),
        }
    }

    async fn update(
        &self, session_key: SessionKey, session_state: SessionState, ttl: &Duration,
    ) -> Result<SessionKey, UpdateError> {
        let body = serde_json::to_string(&session_state).map_err(Into::into).map_err(UpdateError::Serialization)?;

        let id = session_key.as_ref().to_owned();
        let thingy = (self.tb.clone(), id);

        let expiry_time: DateTime<Utc> = match add_duration_to_current(ttl) {
            Some(a) => a,
            None => {
                return Err(UpdateError::Other(anyhow!("Invalid duration length!")));
            }
        };

        let updated = KeyRecordPatch {
            token: Some(body),
            expiry: Some(expiry_time.into()),
        };

        let res: Result<Option<KeyRecord>, surrealdb::Error> = self.client.update(thingy).merge(updated).await;

        if res.is_err() {
            Err(UpdateError::Other(anyhow!("Failed to update database record!")))
        } else {
            Ok(session_key)
        }
    }

    async fn update_ttl(&self, session_key: &SessionKey, ttl: &Duration) -> Result<(), Error> {
        let id = session_key.as_ref().to_owned();
        let thingy = (self.tb.clone(), id);

        if ttl.is_zero() || ttl.is_negative() {
            let _: Option<KeyRecord> = self.client.delete(thingy).await.map_err(|_| anyhow!("Failed to delete database record"))?;
            Ok(())
        } else {
            let expiry_time: DateTime<Utc> = match add_duration_to_current(ttl) {
                Some(a) => a,
                None => {
                    return Err(anyhow!("Invalid duration length!"));
                }
            };

            let updated = KeyRecordPatch {
                token: None,
                expiry: Some(expiry_time.into()),
            };

            let _: Option<KeyRecord> = self.client.update(thingy).merge(updated).await.map_err(|_| anyhow!("Failed to update database record"))?;
            Ok(())
        }
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<(), Error> {
        debug!("Deleting session from DB");
        let id = session_key.as_ref().to_owned();
        let thingy = (self.tb.clone(), id);

        let res = self.client.delete::<Option<KeyRecord>>(thingy).await.map_err(|_| anyhow!("Failed to delete database record"));
        if res.is_ok() {
            debug!("Deleting from DB worked");
            Ok(())
        } else {
            let err = res.unwrap_err();
            debug!("Error deleting from DB: {:?}", err);
            Err(err)
        }
    }
}
