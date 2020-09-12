//! # josekit
//!
//! `josekit` is a JOSE (Javascript Object Signing and Encryption: JWT, JWS, JWE, JWA, JWK) library.
pub mod jwa;
pub mod jwe;
pub mod jwk;
pub mod jws;
pub mod jwt;

mod jose_error;
mod jose_header;
mod der;
mod util;

pub use crate::jose_error::JoseError;
pub use crate::jose_header::JoseHeader;

#[cfg(doctest)]
use doc_comment::doctest;

#[cfg(doctest)]
doctest!("../README.md");
