use actix_session::storage::SessionKey;
use rand::distr::{Alphanumeric, SampleString};


// Stolen from actix/actix-extras/actix-session
// (https://github.com/actix/actix-extras/blob/master/actix-session/src/storage/utils.rs)
// Full credit to the original authors
// Would have just imported their code, but it's pub(crate).
// Originally Licensed under Apache-2.0 and MIT
pub(crate) fn generate_session_key() -> SessionKey {
    let string = Alphanumeric.sample_string(&mut rand::rng(), 64);
    string.try_into().expect("Failed to create SessionKey from random string")
}
