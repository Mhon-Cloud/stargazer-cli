//! `stargazer containers ...`: GET /api/v1/containers, POST .../{id}/start|stop,
//! DELETE .../{id}, GET .../{id}/logs. Start / stop set what the container should be; the
//! cluster controller gets it there, and `--wait` follows until it is. Ordering is in
//! container_order.rs.

use serde_json::Value;

use crate::api::Client;
use crate::cli::{ContainerTarget, ContainersCmd, Wait};
use crate::context::Context;
use crate::error::{CliError, ErrorKind};
use crate::output::{col, print_list, print_message, print_object, Column};

use super::common::{in_project, need_yes, wait_for, CONTAINERS};
use super::container_order;

const COLUMNS: &[Column] = &[
    col("ID", "id"),
    col("NUMBER", "number"),
    col("NAME", "name"),
    col("IMAGE", "image"),
    col("STATUS", "status"),
    col("CPU", "cpu_cores"),
    col("RAM_GB", "ram_gb"),
    col("PROJECT", "project.name"),
];

pub const DETAIL: &[Column] = &[
    col("ID", "id"),
    col("Number", "number"),
    col("Name", "name"),
    col("Image", "image"),
    col("Status", "status"),
    col("Wanted", "desired_state"),
    col("Note", "status_note"),
    col("vCPU", "cpu_cores"),
    col("RAM (GB)", "ram_gb"),
    col("Storage (GB)", "storage_gb"),
    col("Address", "address"),
    col("Ports", "ports"),
    col("DNS names", "dns_names"),
    col("Login user", "login_user"),
    col("Mounts", "mounts"),
    col("Free", "free"),
    col("Paid until", "expires_at"),
    col("Project", "project.name"),
    col("Zone", "zone.name"),
];

pub fn run(cmd: &ContainersCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        ContainersCmd::List(filter) => {
            let containers: Vec<Value> =
                CONTAINERS.list(&client)?.into_iter().filter(|c| in_project(c, filter.project.as_deref())).collect();
            print_list(ctx.format, &containers, COLUMNS, "No containers.");
            Ok(())
        }
        ContainersCmd::Show(target) => {
            print_object(ctx.format, &find(&client, target)?, DETAIL);
            Ok(())
        }
        ContainersCmd::Create(args) => container_order::create(args, ctx, &client),
        ContainersCmd::Images => container_order::images(ctx),
        ContainersCmd::Options => container_order::options(ctx, &client),
        ContainersCmd::Start { target, wait } => {
            let id = id(&client, target)?;
            let answer = client.post(&format!("/api/v1/containers/{id}/start"), None)?;
            settle(ctx, &client, &id, answer, "running", wait)
        }
        ContainersCmd::Stop { target, wait } => {
            let id = id(&client, target)?;
            let answer = client.post(&format!("/api/v1/containers/{id}/stop"), None)?;
            settle(ctx, &client, &id, answer, "stopped", wait)
        }
        ContainersCmd::Delete { target, confirm } => {
            need_yes(confirm.yes, "this destroys the container and its volumes")?;
            let id = id(&client, target)?;
            show(ctx, client.delete(&format!("/api/v1/containers/{id}"))?)
        }
        ContainersCmd::Logs { target, tail } => {
            let id = id(&client, target)?;
            let answer = client.get_query(&format!("/api/v1/containers/{id}/logs"), &[("tail", &tail.to_string())])?;
            let text = answer["logs"].as_str().unwrap_or_default().trim_end().to_string();
            print_message(ctx.format, answer, &text);
            Ok(())
        }
    }
}

fn find(client: &Client, target: &ContainerTarget) -> Result<Value, CliError> {
    CONTAINERS.resolve(client, &target.container, target.project.project.as_deref())
}

fn id(client: &Client, target: &ContainerTarget) -> Result<String, CliError> {
    CONTAINERS.resolve_id(client, &target.container, target.project.project.as_deref())
}

/// Prints the answer, or with --wait the container once it's `until`.
fn settle(ctx: &Context, client: &Client, id: &str, answer: Value, until: &str, wait: &Wait) -> Result<(), CliError> {
    if !wait.wait {
        return show(ctx, answer);
    }
    let done = wait_until_status(client, id, until, wait.timeout)?;
    print_object(ctx.format, &done, DETAIL);
    Ok(())
}

/// Polls the container until its status is `until`; `failed` (or gone) ends the wait with an error.
pub fn wait_until_status(client: &Client, id: &str, until: &str, timeout: u64) -> Result<Value, CliError> {
    wait_for(timeout, &format!("the container to be {until}"), || {
        let Some(c) = CONTAINERS.list(client)?.into_iter().find(|c| c["id"] == id) else {
            return Err(CliError::not_found("the container is gone (deleted, or its order was cancelled)"));
        };
        match c["status"].as_str() {
            Some(s) if s == until => Ok(Some(c)),
            Some("failed") => Err(CliError::new(
                ErrorKind::Conflict,
                format!(
                    "the container failed: {} (a paid order is refunded)",
                    c["status_note"].as_str().unwrap_or("no reason given")
                ),
            )),
            _ => Ok(None),
        }
    })
}

fn show(ctx: &Context, answer: Value) -> Result<(), CliError> {
    print_object(ctx.format, answer.get("container").unwrap_or(&answer), DETAIL);
    Ok(())
}
