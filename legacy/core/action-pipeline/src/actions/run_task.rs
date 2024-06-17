use moon_action::{Action, ActionStatus};
use moon_action_context::{ActionContext, TargetState};
use moon_app_context::AppContext;
use moon_logger::warn;
use moon_platform::Runtime;
use moon_project::Project;
use moon_target::Target;
use moon_task_runner::TaskRunner;
use starbase_styles::color;
use std::env;
use std::sync::Arc;
use tracing::instrument;

const LOG_TARGET: &str = "moon:action:run-task";

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn run_task(
    action: &mut Action,
    context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    project: &Project,
    target: &Target,
    _runtime: &Runtime,
) -> miette::Result<ActionStatus> {
    env::set_var("MOON_RUNNING_ACTION", "run-task");

    let task = project.get_task(&target.task_id)?;

    // Must be set before running the task in case it fails and
    // and error is bubbled up the stack
    action.allow_failure = task.options.allow_failure;

    // If the task is persistent, set the status early since it "never finshes",
    // and the runner will error about a missing hash if it's a dependency
    if task.is_persistent() {
        context.set_target_state(&task.target, TargetState::Passthrough);
    }

    let operations = TaskRunner::new(&app_context, project, task)?
        .run(&context, &action.node)
        .await?
        .operations;

    action.flaky = operations.is_flaky();
    action.status = operations.get_final_status();
    action.operations = operations;

    if action.has_failed() && action.allow_failure {
        warn!(
            target: LOG_TARGET,
            "Task {} has failed, but is marked to allow failures, continuing pipeline",
            color::label(&task.target),
        );
    }

    Ok(action.status)
}
