//! `stargazer servers ...`: GET /api/v1/servers, POST /api/v1/servers/{id}/actions | renew,
//! PUT .../auto-renew. The API queues actions (202); the printed server shows the queued
//! `last_action`, and `--wait` follows it until it's done. Ordering is in compute_order.rs.

use serde_json::{json, Map, Value};

use crate::api::Client;
use crate::cli::{OnOff, ServerTarget, ServersCmd, Wait};
use crate::context::Context;
use crate::error::{CliError, ErrorKind};
use crate::output::{col, print_list, print_object, Column};

use super::common::{id_of, in_project, need_yes, wait_for, SERVERS};
use super::compute_order;

const COLUMNS: &[Column] = &[
    col("ID", "id"),
    col("NAME", "name"),
    col("STATUS", "status"),
    col("CPU", "cpu_cores"),
    col("PROJECT", "project.name"),
    col("ZONE", "zone.name"),
];

pub const DETAIL: &[Column] = &[
    col("ID", "id"),
    col("Name", "name"),
    col("Status", "status"),
    col("vCPU", "cpu_cores"),
    col("Memory (bytes)", "mem_total"),
    col("Disk (bytes)", "disk_total"),
    col("Username", "username"),
    col("Interfaces", "interfaces"),
    col("Project", "project.name"),
    col("Zone", "zone.name"),
    col("Last action", "last_action.action"),
    col("Action status", "last_action.status"),
    col("Action error", "last_action.error"),
    col("Pending reboot", "pending_reboot"),
    col("Order", "order.number"),
    col("Order status", "order.status"),
    col("Paid until", "order.expires_at"),
    col("Auto-renew", "order.auto_renew"),
    col("Renewal price", "order.renewal_price"),
];

pub fn run(cmd: &ServersCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        ServersCmd::List(filter) => {
            let servers: Vec<Value> =
                SERVERS.list(&client)?.into_iter().filter(|s| in_project(s, filter.project.as_deref())).collect();
            print_list(ctx.format, &servers, COLUMNS, "No servers.");
            Ok(())
        }
        ServersCmd::Show(target) => {
            print_object(ctx.format, &find(&client, target)?, DETAIL);
            Ok(())
        }
        ServersCmd::Create(args) => compute_order::create(args, ctx, &client),
        ServersCmd::Options => compute_order::options(ctx, &client),
        ServersCmd::Orders(cmd) => compute_order::orders(cmd, ctx, &client),
        ServersCmd::Start { target, wait } => act(ctx, &client, target, json!({ "action": "start" }), wait),
        ServersCmd::Stop { target, wait } => act(ctx, &client, target, json!({ "action": "stop" }), wait),
        ServersCmd::Rename { target, new_name, wait } => {
            act(ctx, &client, target, json!({ "action": "edit", "name": new_name }), wait)
        }
        ServersCmd::Resize { target, cores, memory_mb, disk_gb, confirm, wait } => {
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
            let server = find(&client, target)?;
            if !server["order"].is_null() {
                need_yes(
                    confirm.yes,
                    &format!(
                        "resizing a Market server charges one month at the new size (unused time is refunded; now ฿{}/month)",
                        server["order"]["renewal_price"].as_str().unwrap_or("?")
                    ),
                )?;
            }
            act_on(ctx, &client, &server, Value::Object(body), wait)
        }
        ServersCmd::Delete { target, confirm, wait } => {
            need_yes(confirm.yes, "this destroys the server and its disks")?;
            act(ctx, &client, target, json!({ "action": "delete" }), wait)
        }
        ServersCmd::Renew { target, confirm } => {
            let server = find(&client, target)?;
            need_yes(
                confirm.yes,
                &format!(
                    "renewing pays one month (฿{}) from your credit",
                    server["order"]["renewal_price"].as_str().unwrap_or("?")
                ),
            )?;
            let answer = client.post(&format!("/api/v1/servers/{}/renew", id_of(&server)?), None)?;
            print_object(ctx.format, answer.get("server").unwrap_or(&answer), DETAIL);
            Ok(())
        }
        ServersCmd::AutoRenew { target, state } => {
            let id = SERVERS.resolve_id(&client, &target.server, target.project.project.as_deref())?;
            let answer = client
                .put(&format!("/api/v1/servers/{id}/auto-renew"), json!({ "auto_renew": *state == OnOff::On }))?;
            print_object(ctx.format, answer.get("server").unwrap_or(&answer), DETAIL);
            Ok(())
        }
    }
}

fn find(client: &Client, target: &ServerTarget) -> Result<Value, CliError> {
    SERVERS.resolve(client, &target.server, target.project.project.as_deref())
}

fn act(ctx: &Context, client: &Client, target: &ServerTarget, body: Value, wait: &Wait) -> Result<(), CliError> {
    let server = find(client, target)?;
    act_on(ctx, client, &server, body, wait)
}

fn act_on(ctx: &Context, client: &Client, server: &Value, body: Value, wait: &Wait) -> Result<(), CliError> {
    let id = id_of(server)?.to_string();
    let deleting = body["action"] == "delete";
    let answer = client.post(&format!("/api/v1/servers/{id}/actions"), Some(body))?;
    let queued = answer.get("server").cloned().unwrap_or(answer);
    let result = if wait.wait { wait_action(client, &id, &queued, deleting, wait.timeout)? } else { queued };
    print_object(ctx.format, &result, DETAIL);
    Ok(())
}

/// Follows the action just queued (its `last_action.created_at`) until it's done or failed. A
/// deleted server is done when it's gone from the list.
fn wait_action(client: &Client, id: &str, queued: &Value, deleting: bool, timeout: u64) -> Result<Value, CliError> {
    let started = queued["last_action"]["created_at"].clone();
    wait_for(timeout, "the server's action", || {
        let Some(server) = SERVERS.list(client)?.into_iter().find(|s| s["id"] == id) else {
            return Ok(deleting.then(|| json!({ "id": id, "status": "deleted" })));
        };
        let action = &server["last_action"];
        if action["created_at"] != started {
            return Ok(None);
        }
        match action["status"].as_str() {
            Some("done") => Ok(Some(server.clone())),
            Some("failed") => Err(CliError::new(
                ErrorKind::Conflict,
                format!("the action failed: {}", action["error"].as_str().unwrap_or("no reason given")),
            )),
            _ => Ok(None),
        }
    })
}
