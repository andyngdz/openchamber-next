//! Loads one directory's catalog: its OpenCode config, providers, models and agents, then
//! lets `oc-state` build the picker's view (`useConfigStore.ts:2405-2442`).

use futures::channel::mpsc::Sender;
use oc_client::{Client, ClientError, Directory};
use oc_state::{Catalog, SelectionAction};

use super::protocol::{Closed, Update, emit, selection};

/// Loads the four answers together and reports them as one catalog change, then the
/// directory's pending permissions and forms: a loaded catalog means its directory has an
/// open session, so its dock is read here too (design "Live").
pub(crate) async fn load(
    client: &Client,
    directory: &str,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    let action = match fetch(client, directory).await {
        Ok(catalog) => SelectionAction::CatalogLoaded {
            directory: directory.to_owned(),
            catalog,
        },
        Err(error) => SelectionAction::CatalogFailed {
            directory: directory.to_owned(),
            message: format!("Could not load the model and agent list: {error}"),
        },
    };
    selection(updates, action).await?;
    load_requests(client, directory, updates).await
}

/// `GET /api/permission/request` and `GET /api/form` for one directory. A failure keeps the
/// entries the directory already had and marks it stale.
pub(crate) async fn load_requests(
    client: &Client,
    directory: &str,
    updates: &mut Sender<Update>,
) -> Result<(), Closed> {
    let scoped = Directory::new(directory.to_owned());
    let permissions = client
        .permissions(&scoped)
        .await
        .map_err(|error| format!("Could not load the pending permissions: {error}"));
    let update = Update::Permissions {
        directory: directory.to_owned(),
        result: permissions,
    };
    emit(updates, update).await?;
    let forms = client
        .forms(&scoped)
        .await
        .map_err(|error| format!("Could not load the pending forms: {error}"));
    let update = Update::Forms {
        directory: directory.to_owned(),
        result: forms,
    };
    emit(updates, update).await
}

async fn fetch(client: &Client, directory: &str) -> Result<Catalog, ClientError> {
    let directory = Directory::new(directory.to_owned());
    let (config, providers, models, agents) = futures::try_join!(
        client.config(&directory),
        client.providers(&directory),
        client.models(&directory),
        client.agents(&directory),
    )?;
    Ok(Catalog::from_parts(providers, models, agents, config))
}

#[cfg(test)]
#[path = "catalog_test.rs"]
mod catalog_test;
