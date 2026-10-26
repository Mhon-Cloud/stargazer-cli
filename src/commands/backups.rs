//! `stargazer backups ...`: GET /api/v1/backups (read live from the servers' backup storages),
//! POST /api/v1/servers/{id}/backups | /backups/restore | /backups/delete (queued actions).

use serde_json::{json, Value};

use crate::cli::BackupsCmd;
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_list, print_message, Column};

use super::common::{id_of, need_yes, SERVERS};

const COLUMNS: &[Column] = &[
    col("VOLID", "volid"),
    col("SERVER", "server.name"),
    col("STORAGE", "storage"),
    col("SIZE", "size"),
    col("CREATED", "created_at"),
    col("NOTES", "notes"),
];

pub fn run(cmd: &BackupsCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        BackupsCmd::List { server } => {
            let answer = client.get("/api/v1/backups")?;
            let id = match server {
                Some(s) => Some(SERVERS.resolve_id(&client, s, None)?),
                None => None,
            };
            let backups: Vec<Value> = answer["backups"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|b| id.as_ref().is_none_or(|id| b["server"]["id"] == json!(id)))
                .cloned()
                .collect();
            print_list(ctx.format, &backups, COLUMNS, "No backups.");
            Ok(())
        }
        BackupsCmd::Create { server, storage, notes } => {
            let id = SERVERS.resolve_id(&client, server, None)?;
            let storage = match storage {
                Some(s) => s.clone(),
                None => {
                    let answer = client.get("/api/v1/backups")?;
                    answer["storages"][&id][0]
                        .as_str()
                        .map(str::to_string)
                        .ok_or_else(|| CliError::usage("this server has no backup storage; ask the platform's admin"))?
                }
            };
            let answer = client
                .post(&format!("/api/v1/servers/{id}/backups"), Some(json!({ "storage": storage, "notes": notes })))?;
            queued(ctx, answer, "Backup queued.")
        }
        BackupsCmd::Restore { server, volid, confirm } => {
            let found = SERVERS.resolve(&client, server, None)?;
            need_yes(confirm.yes, "this overwrites the server's disks with the backup (the server must be stopped)")?;
            let answer = client.post(
                &format!("/api/v1/servers/{}/backups/restore", id_of(&found)?),
                Some(json!({ "volid": volid })),
            )?;
            queued(ctx, answer, "Restore queued.")
        }
        BackupsCmd::Delete { server, volid, confirm } => {
            let found = SERVERS.resolve(&client, server, None)?;
            need_yes(confirm.yes, "this deletes the backup for good")?;
            let answer = client
                .post(&format!("/api/v1/servers/{}/backups/delete", id_of(&found)?), Some(json!({ "volid": volid })))?;
            queued(ctx, answer, "Delete queued.")
        }
    }
}

fn queued(ctx: &Context, answer: Value, text: &str) -> Result<(), CliError> {
    print_message(ctx.format, json!({ "queued": true, "message": answer["message"] }), text);
    Ok(())
}
