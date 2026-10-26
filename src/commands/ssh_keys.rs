//! `stargazer ssh-keys ...` (the console's Credentials): GET/POST /api/v1/credentials,
//! PUT/DELETE .../{id}, POST .../{id}/bindings, DELETE .../{id}/bindings/{serverId}.
//! Installing writes the key into the server's cloud-init keys; it's applied at the next boot.

use std::io::Read;

use serde_json::json;

use crate::cli::SshKeysCmd;
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_list, print_message, print_object, Column};

use super::common::{id_of, in_project, need_yes, order_project, SERVERS, SSH_KEYS};

const COLUMNS: &[Column] = &[
    col("ID", "id"),
    col("NAME", "name"),
    col("FINGERPRINT", "fingerprint"),
    col("SERVERS", "bindings"),
    col("PROJECT", "project.name"),
];

const DETAIL: &[Column] = &[
    col("ID", "id"),
    col("Name", "name"),
    col("Type", "type"),
    col("Fingerprint", "fingerprint"),
    col("Installed on", "bindings"),
    col("Project", "project.name"),
    col("Added", "created_at"),
];

pub fn run(cmd: &SshKeysCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        SshKeysCmd::List(filter) => {
            let keys: Vec<_> =
                SSH_KEYS.list(&client)?.into_iter().filter(|k| in_project(k, filter.project.as_deref())).collect();
            print_list(ctx.format, &keys, COLUMNS, "No SSH keys.");
            Ok(())
        }
        SshKeysCmd::Add { name, file, project } => {
            let public_key = read_key(file)?;
            if public_key.contains("PRIVATE KEY") {
                return Err(CliError::usage("that's a private key: give the .pub file"));
            }
            let project = order_project(&client, project.as_deref())?;
            let answer = client.post(
                "/api/v1/credentials",
                Some(json!({ "project_id": id_of(&project)?, "name": name, "public_key": public_key.trim() })),
            )?;
            print_object(ctx.format, answer.get("credential").unwrap_or(&answer), DETAIL);
            Ok(())
        }
        SshKeysCmd::Rename { key, new_name } => {
            let id = SSH_KEYS.resolve_id(&client, key, None)?;
            let answer = client.put(&format!("/api/v1/credentials/{id}"), json!({ "name": new_name }))?;
            print_object(ctx.format, answer.get("credential").unwrap_or(&answer), DETAIL);
            Ok(())
        }
        SshKeysCmd::Delete { key, confirm } => {
            let found = SSH_KEYS.resolve(&client, key, None)?;
            need_yes(confirm.yes, "this deletes the key from the project")?;
            let answer = client.delete(&format!("/api/v1/credentials/{}", id_of(&found)?))?;
            print_message(
                ctx.format,
                json!({ "deleted": true, "id": found["id"], "message": answer["message"] }),
                "Deleted.",
            );
            Ok(())
        }
        SshKeysCmd::Install { key, server } => {
            let found = SSH_KEYS.resolve(&client, key, None)?;
            let server = SERVERS.resolve_id(&client, server, found["project"]["id"].as_str())?;
            let answer = client.post(
                &format!("/api/v1/credentials/{}/bindings", id_of(&found)?),
                Some(json!({ "server_id": server })),
            )?;
            print_object(ctx.format, answer.get("credential").unwrap_or(&answer), DETAIL);
            Ok(())
        }
        SshKeysCmd::Uninstall { key, server } => {
            let found = SSH_KEYS.resolve(&client, key, None)?;
            let server = SERVERS.resolve_id(&client, server, found["project"]["id"].as_str())?;
            let answer = client.delete(&format!("/api/v1/credentials/{}/bindings/{server}", id_of(&found)?))?;
            print_object(ctx.format, answer.get("credential").unwrap_or(&answer), DETAIL);
            Ok(())
        }
    }
}

/// The public key from a file, or stdin for `-`.
fn read_key(file: &str) -> Result<String, CliError> {
    let mut text = String::new();
    if file == "-" {
        std::io::stdin().read_to_string(&mut text).map_err(|e| CliError::usage(format!("can't read stdin: {e}")))?;
    } else {
        let path = expand_home(file);
        text = std::fs::read_to_string(&path)
            .map_err(|e| CliError::usage(format!("can't read {}: {e}", path.display())))?;
    }
    if text.trim().is_empty() {
        return Err(CliError::usage("the key is empty"));
    }
    Ok(text)
}

/// `~/x` -> the home folder's x (a quoted path doesn't get the shell's expansion).
fn expand_home(path: &str) -> std::path::PathBuf {
    match (path.strip_prefix("~/"), dirs::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => std::path::PathBuf::from(path),
    }
}
