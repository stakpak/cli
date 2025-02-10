use crate::client::models::{Action, ActionStatus};

pub async fn run_interactive_actions(
    action_queue: Vec<Action>,
    print: &impl Fn(&str),
    short_circuit_actions: bool,
) -> Result<Vec<Action>, String> {
    let mut updated_actions = Vec::with_capacity(action_queue.len());
    for action in action_queue.into_iter().filter(|a| a.is_pending()) {
        let updated_action = action.run_interactive(print).await?;

        if short_circuit_actions {
            if let Action::RunCommand {
                exit_code: Some(code),
                ..
            } = &updated_action
            {
                if *code != 0 {
                    updated_actions.push(updated_action);
                    return Ok(updated_actions);
                }
            }
        }

        updated_actions.push(updated_action);
    }

    Ok(updated_actions)
}

pub async fn run_remote_actions(
    action_queue: Vec<Action>,
    print: &impl Fn(&str),
) -> Result<Vec<Action>, String> {
    let mut updated_actions = Vec::with_capacity(action_queue.len());
    for action in action_queue.clone().into_iter() {
        if !action.is_pending() || !matches!(action, Action::RunCommand { .. }) {
            continue;
        }

        if action.get_status() == &ActionStatus::PendingHumanApproval {
            // To Print Warnings and Reasoning
            if updated_actions.is_empty() {
                action.clone().run(print).await?;
            }
            updated_actions.extend(action_queue.iter().skip(updated_actions.len()).cloned());
            return Ok(updated_actions);
        }

        let updated_action = action.run(print).await?;
        updated_actions.push(updated_action);
    }

    Ok(updated_actions)
}
