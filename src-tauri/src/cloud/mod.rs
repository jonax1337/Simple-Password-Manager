//! Cloud-sync client. Mirror of `server/` crate on the desktop side.
//!
//! Layering:
//!   crypto  — Argon2id KDF, SHA-256 derivations, AES-GCM blob seal/open.
//!   client  — typed HTTP wrapper around the server's REST surface.
//!   session — runtime auth/state for one linked account.
//!
//! Threat model: master password never leaves the device. The server holds
//! (i) an Argon2-of-Argon2 auth hash and (ii) the AES-GCM-encrypted vault
//! blob. A full server DB dump still requires brute-forcing the master
//! password through both Argon2 layers + AES-GCM to extract plaintext.

pub mod client;
pub mod crypto;
pub mod session;

pub use client::CloudClient;
pub use session::CloudSession;
