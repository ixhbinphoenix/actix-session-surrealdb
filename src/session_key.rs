use actix_session::storage::SessionKey;
use rand::{distributions::Alphanumeric, rngs::OsRng, Rng as _};


// Stolen from actix/actix-extras/actix-session
// (https://github.com/actix/actix-extras/blob/master/actix-session/src/storage/utils.rs)
// Full credit to the original authors
// Would have just imported their code, but it's pub(crate).
// Originally Licensed under Apache-2.0 and MIT
pub(crate) fn generate_session_key() -> SessionKey {
    let value = std::iter::repeat(()).map(|()| OsRng.sample(Alphanumeric)).take(64).collect::<Vec<_>>();

    String::from_utf8(value).unwrap().try_into().unwrap()
}
