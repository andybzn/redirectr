//! `src/routes/mod.rs`
//! Request handlers for the endpoints registered by [`crate::app`].
//!
//! The redirect handlers are re-exported here, so `routes::get`,
//! `routes::post`, and `routes::default` refer to the `redirect` module.
pub mod health;
mod redirect;

pub use redirect::*;
