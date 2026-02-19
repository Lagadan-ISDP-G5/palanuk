use std::sync::{Arc, LazyLock, Mutex};
use zenoh::{Config, Session, Wait};

static SHARED_SESSION: LazyLock<Mutex<Option<Arc<Session>>>> =
    LazyLock::new(|| Mutex::new(None));

/// Opens a shared Zenoh session, or returns the existing one.
/// If a custom config file is provided, it will only be used for the first session opened.
pub fn shared_session(config: Config) -> Result<Arc<Session>, String> {
    let mut guard = SHARED_SESSION.lock().map_err(|e| e.to_string())?;
    if let Some(ref session) = *guard {
        return Ok(Arc::clone(session));
    }
    let session = Wait::wait(zenoh::open(config))
        .map_err(|e| format!("Failed to open Zenoh session: {e}"))?;
    let session = Arc::new(session);
    *guard = Some(Arc::clone(&session));
    Ok(session)
}

/// Closes the shared session if this is the last Arc holder.
/// Returns true if the session was actually closed.
pub fn close_shared_session() -> bool {
    let mut guard = match SHARED_SESSION.lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    if let Some(session) = guard.take() {
        // Only close if we hold the last strong reference
        match Arc::try_unwrap(session) {
            Ok(session) => {
                let _ = Wait::wait(session.close());
                true
            }
            Err(arc) => {
                // Still in use, put it back
                *guard = Some(arc);
                false
            }
        }
    } else {
        false
    }
}
