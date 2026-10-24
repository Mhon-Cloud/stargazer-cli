//! `stargazer servers ...`: GET /api/v1/servers and POST /api/v1/servers/{id}/actions.
//! The API queues actions (202); the printed server shows the queued `last_action`.

use serde_json::{json, Map, Value};

use crate::api::Client;
use crate::cli::{ServerTarget, ServersCmd};
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_list, print_object, Column};

use super::common::{find, id_of, in_project, items};

const COLUMNS: &[Column] = &[
    col("ID", "id"),
    col("NAME", "name"),
    col("STATUS", "status"),
    col("CPU", "cpu_cores"),
    col("PROJECT", "project.name"),
    col("ZONE", "zone.name"),
];

const DETAIL: &[Column] = &[
    col("ID", "id"),
    col("Name", "name"),
    col("Status", "status"),
    col("vCPU", "cpu_cores"),
    col("Memory (bytes)", "mem_total"),
    col("Disk (bytes)", "disk_total"),
    col("Project", "project.name"),
    col("Zone", "zone.name"),
    col("Last action", "last_action.action"),
    col("Action status", "last_action.status"),
    col("Pending reboot", "pending_reboot"),
];

pub fn run(cmd: &ServersCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        ServersCmd::List(filter) => {
            let servers: Vec<Value> =
                fetch(&client)?.into_iter().filter(|s| in_project(s, filter.project.as_deref())).collect();
            print_list(ctx.format, &servers, COLUMNS, "No servers.");
            Ok(())
        }
        ServersCmd::Start(target) => act(ctx, &client, target, json!({ "action": "start" })),
        ServersCmd::Stop(target) => act(ctx, &client, target, json!({ "action": "stop" })),
        ServersCmd::Rename { target, new_name } => {
            act(ctx, &client, target, json!({ "action": "edit", "name": new_name }))
        }
        ServersCmd::Resize { target, cores, memory_mb, disk_gb } => {
            let mut body = Map::new();
            body.insert("action".into(), json!("edit"));
            if let Some(v) = cores {
                body.insert("cores".into(), json!(v));
            }
            if let Some(v) = memory_mb {
                body.insert("memory_mb".into(), json!(v));
            }
            if let Some(v) = disk_gb {
                body.insert("disk_gb".into(), json!(v));
            }
            if body.len() == 1 {
                return Err(CliError::usage("give at least one of --cores, --memory-mb, --disk-gb"));
            }
            act(ctx, &client, target, Value::Object(body))
        }
        ServersCmd::Delete { target, yes } => {
            if !yes {
                return Err(CliError::usage("this destroys the server and its disks; add --yes to confirm"));
            }
            act(ctx, &client, target, json!({ "action": "delete" }))
        }
    }
}

fn fetch(client: &Client) -> Result<Vec<Value>, CliError> {
    Ok(items(client.get("/api/v1/servers")?, "servers"))
}

fn act(ctx: &Context, client: &Client, target: &ServerTarget, body: Value) -> Result<(), CliError> {
    let servers = fetch(client)?;
    let server = find(&servers, &target.server, target.project.project.as_deref(), &["name"], "server")?;
    let answer = client.post(&format!("/api/v1/servers/{}/actions", id_of(server)?), Some(body))?;
    print_object(ctx.format, answer.get("server").unwrap_or(&answer), DETAIL);
    Ok(())
}
