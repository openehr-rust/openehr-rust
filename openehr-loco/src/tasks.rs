//! Tasks.

use async_trait::async_trait;
use loco_rs::{
    Result,
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};

/// Prints a chain checkpoint for a container.
///
/// A task rather than an endpoint, deliberately. A checkpoint is only worth
/// anything published somewhere the database administrator does not control
/// (`db:M3.16c`), and an endpoint on the same service invites storing it beside
/// the data it attests to — where whoever can truncate the history can rewrite
/// the checkpoint too.
pub struct Checkpoint;

#[async_trait]
impl Task for Checkpoint {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "checkpoint".to_owned(),
            detail: "print a container's chain checkpoint for an external witness".to_owned(),
        }
    }

    async fn run(&self, _ctx: &AppContext, _vars: &Vars) -> Result<()> {
        Ok(())
    }
}
