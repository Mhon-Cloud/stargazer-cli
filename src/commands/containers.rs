//! `stargazer containers ...`: GET /api/v1/containers, POST .../{id}/start|stop,
//! DELETE .../{id}, GET .../{id}/logs.

use serde_json::Value;

use crate::api::Client;
use crate::cli::{ContainerTarget, ContainersCmd};
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_list, print_message, print_object, Column};

use super::common::{find, id_of, in_project, items};

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

const DETAIL: &[Column] = &[
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
    col("DNS names", "dns_names"),
    col("Project", "project.name"),
];

pub fn run(cmd: &ContainersCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        ContainersCmd::List(filter) => {
            let containers: Vec<Value> =
                fetch(&client)?.into_iter().filter(|c| in_project(c, filter.project.as_deref())).collect();
            print_list(ctx.format, &containers, COLUMNS, "No containers.");
            Ok(())
        }
        ContainersCmd::Start(target) => {
            let id = resolve(&client, target)?;
            show(ctx, client.post(&format!("/api/v1/containers/{id}/start"), None)?)
        }
        ContainersCmd::Stop(target) => {
            let id = resolve(&client, target)?;
            show(ctx, client.post(&format!("/api/v1/containers/{id}/stop"), None)?)
        }
        ContainersCmd::Delete { target, yes } => {
            if !yes {
                return Err(CliError::usage("this destroys the container and its volumes; add --yes to confirm"));
            }
            let id = resolve(&client, target)?;
            show(ctx, client.delete(&format!("/api/v1/containers/{id}"))?)
        }
        ContainersCmd::Logs { target, tail } => {
            let id = resolve(&client, target)?;
            let answer = client.get_query(&format!("/api/v1/containers/{id}/logs"), &[("tail", &tail.to_string())])?;
            let text = answer["logs"].as_str().unwrap_or_default().trim_end().to_string();
            print_message(ctx.format, answer, &text);
            Ok(())
        }
    }
}

fn fetch(client: &Client) -> Result<Vec<Value>, CliError> {
    Ok(items(client.get("/api/v1/containers")?, "containers"))
}

fn resolve(client: &Client, target: &ContainerTarget) -> Result<String, CliError> {
    let containers = fetch(client)?;
    let container =
        find(&containers, &target.container, target.project.project.as_deref(), &["name", "number"], "container")?;
    Ok(id_of(container)?.to_string())
}

fn show(ctx: &Context, answer: Value) -> Result<(), CliError> {
    print_object(ctx.format, answer.get("container").unwrap_or(&answer), DETAIL);
    Ok(())
}
