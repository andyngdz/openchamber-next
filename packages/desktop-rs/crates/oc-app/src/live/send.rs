//! The send sequence: create a Chat folder, create the session with the picked agent and
//! model, or switch an open session and then prompt. A failure of any step keeps the draft
//! and shows the composer's error.

use futures::channel::mpsc::Sender;
use oc_client::types::ModelRef;
use oc_client::{Client, Directory};
use oc_state::{Action, SessionId, is_same_model_ref};

use super::protocol::{Closed, CreateTarget, Update, apply, emit};

/// `POST /api/session` and open it, with no prompt.
pub(crate) async fn create(
    client: &Client,
    target: &CreateTarget,
    agent: Option<&str>,
    model: Option<&ModelRef>,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    if let Err(message) = ensure_folder(client, target).await {
        return apply(updates, Action::CreateFailed(message)).await;
    }
    let directory = Directory::new(target.directory().to_owned());
    match client.create_session(&directory, agent, model).await {
        Ok(session) => {
            let update = Update::Created {
                session,
                directory: target.directory().to_owned(),
            };
            emit(updates, update).await
        }
        Err(error) => {
            let message = format!("Could not create a session: {error}");
            apply(updates, Action::CreateFailed(message)).await
        }
    }
}

/// `POST /api/session` from an empty draft, then the first prompt.
pub(crate) async fn create_and_prompt(
    client: &Client,
    target: &CreateTarget,
    text: &str,
    agent: Option<&str>,
    model: Option<&ModelRef>,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    if let Err(message) = ensure_folder(client, target).await {
        return apply(updates, Action::PromptFailed(message)).await;
    }
    let directory = Directory::new(target.directory().to_owned());
    let session = match client.create_session(&directory, agent, model).await {
        Ok(session) => session,
        Err(error) => {
            let message = format!("Could not create a session: {error}");
            return apply(updates, Action::PromptFailed(message)).await;
        }
    };
    match client.send_prompt(&directory, &session.id, text).await {
        Ok(_) => {
            let update = Update::CreatedAndSent {
                session,
                directory: target.directory().to_owned(),
                text: text.to_owned(),
            };
            emit(updates, update).await
        }
        Err(error) => {
            // The session exists now; open it so a retry sends into it instead of creating
            // another one, and keep the draft under the composer error.
            let created = Update::Created {
                session,
                directory: target.directory().to_owned(),
            };
            emit(updates, created).await?;
            let message = format!("Could not send the prompt: {error}");
            apply(updates, Action::PromptFailed(message)).await
        }
    }
}

/// An open session's send: the agent and model to switch to, then the prompt.
pub(crate) struct OpenSend<'a> {
    /// The open session.
    pub(crate) session: &'a SessionId,
    /// The directory it lives in.
    pub(crate) directory: &'a str,
    /// The composer text.
    pub(crate) text: &'a str,
    /// The agent the composer picked.
    pub(crate) agent: Option<&'a str>,
    /// The model the composer picked.
    pub(crate) model: Option<&'a ModelRef>,
    /// The model the state's session record holds, which the model compare reads.
    pub(crate) record_model: Option<&'a ModelRef>,
}

/// Whether the send goes on to its next step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Next,
    Stop,
}

impl super::protocol::SendRequest {
    /// Borrows the request as the send sequence reads it.
    pub(crate) fn open(&self) -> OpenSend<'_> {
        OpenSend {
            session: &self.session,
            directory: &self.directory,
            text: &self.text,
            agent: self.agent.as_deref(),
            model: self.model.as_ref(),
            record_model: self.record_model.as_ref(),
        }
    }
}

/// Switches an open session to the picked agent and model, then prompts.
pub(crate) async fn prompt(
    client: &Client,
    open: OpenSend<'_>,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    if let Some(agent) = open.agent
        && sync_agent(client, &open, agent, updates).await? == Step::Stop
    {
        return Ok(());
    }
    if let Some(model) = open.model
        && sync_model(client, &open, model, updates).await? == Step::Stop
    {
        return Ok(());
    }
    let directory = Directory::new(open.directory.to_owned());
    match client
        .send_prompt(&directory, open.session.as_str(), open.text)
        .await
    {
        Ok(_) => {
            let update = Update::Sent {
                text: open.text.to_owned(),
            };
            emit(updates, update).await
        }
        Err(error) => {
            let message = format!("Could not send the prompt: {error}");
            apply(updates, Action::PromptFailed(message)).await
        }
    }
}

/// A fresh record decides whether the agent differs; a failure keeps the draft.
async fn sync_agent(
    client: &Client,
    open: &OpenSend<'_>,
    agent: &str,
    updates: &mut Sender<Update>,
) -> Result<Step, Closed> {
    let directory = Directory::new(open.directory.to_owned());
    let fresh = match client
        .session_record(&directory, open.session.as_str())
        .await
    {
        Ok(fresh) => fresh,
        Err(error) => {
            let message = format!("Could not read the session: {error}");
            apply(updates, Action::PromptFailed(message)).await?;
            return Ok(Step::Stop);
        }
    };
    if fresh.agent.as_deref() == Some(agent) {
        return Ok(Step::Next);
    }
    if let Err(error) = client
        .switch_agent(&directory, open.session.as_str(), agent)
        .await
    {
        let message = format!("Could not switch the session's agent: {error}");
        apply(updates, Action::PromptFailed(message)).await?;
        return Ok(Step::Stop);
    }
    Ok(Step::Next)
}

/// The state's record decides whether the model, or only its variant, changed.
async fn sync_model(
    client: &Client,
    open: &OpenSend<'_>,
    model: &ModelRef,
    updates: &mut Sender<Update>,
) -> Result<Step, Closed> {
    let changed = open
        .record_model
        .is_none_or(|current| !is_same_model_ref(current, model));
    if !changed {
        return Ok(Step::Next);
    }
    let directory = Directory::new(open.directory.to_owned());
    if let Err(error) = client
        .switch_model(&directory, open.session.as_str(), model)
        .await
    {
        let message = format!("Could not switch the session's model: {error}");
        apply(updates, Action::PromptFailed(message)).await?;
        return Ok(Step::Stop);
    }
    Ok(Step::Next)
}

/// A Chat folder must exist before a session is created in it; a project directory does.
async fn ensure_folder(client: &Client, target: &CreateTarget) -> Result<(), String> {
    let CreateTarget::Chat { folder } = target else {
        return Ok(());
    };
    let directory = Directory::new(folder.clone());
    match client.mkdir(&directory).await {
        Ok(result) if result.success => Ok(()),
        Ok(_) => Err("Could not create the chat folder".to_owned()),
        Err(error) => Err(format!("Could not create the chat folder: {error}")),
    }
}

#[cfg(test)]
#[path = "send_test.rs"]
mod send_test;
