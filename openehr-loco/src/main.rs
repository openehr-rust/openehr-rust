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

use loco_rs::cli;
use openehr_loco::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    // `cli::main::<App>()`, not the `MigratorTrait` form: that one belongs to
    // `with-db`, which this crate does not take (`db:S1.5`). Migrations here
    // are the store's business, and before 1.0 there are none (`db:O10.14`).
    cli::main::<App>().await
}
