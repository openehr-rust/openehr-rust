//! The server binary.
//!
//! # Why this file did not exist until now
//!
//! The README said `cargo run`. There was no `[[bin]]` target and no
//! `config/`, so `cargo run` answered *"a bin target must be available"* and
//! had done since the crate was written.
//!
//! It went unnoticed for the reason this repository keeps finding: the tests
//! that exist build the router directly, and the router is the interesting
//! part. Nothing exercised `boot`, so nothing exercised the one path an
//! operator actually takes. `W0.3` again — a documented instruction is a claim,
//! and this one was never run.

#![forbid(unsafe_code)]

use openehr_loco::app;

#[tokio::main]
async fn main() -> axa::Result<()> {
    // `axa::cli::parse`, not `Cli::parse`: it names this service in `--help`
    // and refuses a command that destroys data outside development.
    app::run(axa::cli::parse(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
    ))
    .await
}
