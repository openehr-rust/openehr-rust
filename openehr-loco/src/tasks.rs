//! Operator tasks.
//!
//! # Tasks open their own store
//!
//! [`Hooks::before_run`](loco_rs::app::Hooks::before_run) is **not** on the
//! path a task takes: `cli::main` builds the context and calls `run_task`
//! directly, so `shared_store` is empty. A task that read the store from the
//! context would find nothing and report, from a command line, the `503` this
//! crate uses to mean "not ready".
//!
//! That is the same lesson as `boot` versus `before_run` in [`crate::app`],
//! arriving from the other direction.
//!
//! # Why these are tasks and not endpoints
//!
//! A checkpoint is worth something only when it is published somewhere the
//! database administrator does not control (`db:M3.16c`). An endpoint on the
//! service that holds the data invites storing it beside the data it attests
//! to, where whoever can truncate the history can rewrite the checkpoint too.
//!
//! The same argument applies to [`Verify`], one step weaker: a verification
//! endpoint answering "all fine" is only as trustworthy as the process serving
//! it, and an attacker who has reached the database is one step from the
//! process. A task run from elsewhere, on a schedule, by someone else, is the
//! arrangement that means anything.

use async_trait::async_trait;
use loco_rs::{
    Result,
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use openehr::base::HierObjectId;
use openehr_store::{Store as _, integrity};

use crate::app::{open_store, open_store_at};

/// Opens the database this task is to inspect.
///
/// `path:` names one explicitly; without it the store the service itself uses.
/// The explicit form is what checks a **restored backup**, which is the copy an
/// operator most wants verified and never the one the service has open.
fn store_for(vars: &Vars) -> Result<openehr_sqlite::SqliteStore> {
    vars.cli
        .get("path")
        .map_or_else(open_store, |path| open_store_at(std::path::Path::new(path)))
}

/// Reads the `container` argument.
///
/// Named rather than positional: a task invoked with the wrong identifier
/// reports confidently on the wrong record, and `container:<uid>` is harder to
/// get wrong by accident than an argument in a position.
fn container(vars: &Vars) -> Result<HierObjectId> {
    let raw = vars.cli.get("container").ok_or_else(|| {
        loco_rs::Error::Message("expected container:<versioned-object-uid>".to_owned())
    })?;
    raw.parse()
        .map_err(|_| loco_rs::Error::Message(format!("not an identifier: {raw}")))
}

/// Prints a chain checkpoint for a container, for an external witness.
///
/// ```sh
/// cargo loco task checkpoint container:87284370-2D4B-4E3D-A3F3-F303D2F4F34B \
///   path:/backups/openehr.sqlite3
/// ```
///
/// The output carries a count, a head digest, and the last version's
/// identifier, and **no clinical content** — so it can go somewhere clinical
/// data may not: an append-only log, a third party, a printout in a safe.
pub struct Checkpoint;

#[async_trait]
impl Task for Checkpoint {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "checkpoint".to_owned(),
            detail: "print a container's chain checkpoint for an external witness".to_owned(),
        }
    }

    async fn run(&self, _ctx: &AppContext, vars: &Vars) -> Result<()> {
        println!("{}", checkpoint_line(vars)?);
        Ok(())
    }
}

/// The line [`Checkpoint`] prints.
///
/// Split out so a test can assert what it *says*. A test that only checked
/// `run` returned `Ok` would have passed against the empty body this task had
/// until now, which is the defect being fixed rather than a way to check it.
///
/// # Errors
///
/// Returns [`loco_rs::Error::Message`] if the database cannot be opened or the
/// container does not exist.
pub fn checkpoint_line(vars: &Vars) -> Result<String> {
    store_for(vars)?
        .chain_checkpoint(&container(vars)?)
        .map_err(|e| loco_rs::Error::Message(e.to_string()))
}

/// Verifies a container's stored history and **fails** if it is not intact.
///
/// ```sh
/// cargo loco task verify container:87284370-2D4B-4E3D-A3F3-F303D2F4F34B
/// ```
///
/// Checks more than the chain: `openehr_store::integrity::verify_versions`
/// recomputes each version's content digest from the stored bytes, which is
/// what catches a document edited in place while its chain columns were left
/// alone (`db:M3.16d`).
///
/// # Why this returns an error rather than printing a verdict
///
/// Because the runs that matter are the ones nobody watches — a cron job, a
/// pipeline step, a nightly sweep. A verification tool that exits `0` after
/// finding a breach is one whose finding nobody sees.
///
/// It fails on anything not intact, which includes `UnknownKey` and `Empty`.
/// Those are not breaches, and they are not passes: a check that could not be
/// completed must not report as one that was.
pub struct Verify;

#[async_trait]
impl Task for Verify {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "verify".to_owned(),
            detail: "verify a container's chain and stored content; fails if not intact".to_owned(),
        }
    }

    async fn run(&self, _ctx: &AppContext, vars: &Vars) -> Result<()> {
        println!("{}", verify_line(vars)?);
        Ok(())
    }
}

/// The line [`Verify`] prints, or the error it fails with.
///
/// Split out for the same reason as [`checkpoint_line`], and here it matters
/// more: the verdict is the whole output, so a test that could not read it
/// could only check that nothing threw.
///
/// # Errors
///
/// Returns [`loco_rs::Error::Message`] if the history is not intact, naming the
/// verdict — or if the database cannot be read.
pub fn verify_line(vars: &Vars) -> Result<String> {
    let store = store_for(vars)?;
    let container = container(vars)?;
    let rows = store
        .all_versions(&container)
        .map_err(|e| loco_rs::Error::Message(e.to_string()))?;

    // No keys, and the line says what that costs. This service holds no chain
    // key and must not: a key held by the process that writes the rows attests
    // to nothing (`db:M3.16c`). So the best verdict available here is
    // `Unkeyed`, and a reader is told rather than left to infer.
    let verdict = integrity::verify_versions(&rows, &[]);
    if verdict.is_intact() {
        return Ok(format!(
            "{container}: {} versions, {verdict:?} — unsigned, so this detects an \
             edit by someone who cannot recompute a digest and not one by an \
             attacker holding the database",
            rows.len()
        ));
    }
    Err(loco_rs::Error::Message(format!(
        "{container}: history is NOT intact: {verdict:?}"
    )))
}
