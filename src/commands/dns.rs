//! `stargazer dns ...`: GET/POST /api/v1/dns, GET /api/v1/dns/check, PUT/DELETE /api/v1/dns/{id}.
//! A name points at a port of a container in the same project: `web` (HTTPS through the
//! platform's proxy) for an HTTP port, `direct` (A + optional Minecraft SRV records) for any
//! TCP / UDP port. Users never give an address: the API takes it from the container.

use serde_json::{json, Value};

use crate::api::Client;
use crate::cli::{DnsCmd, DnsKind, DnsTarget};
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_list, print_message, print_object, Column};

use super::common::{id_of, in_project, need_yes, CONTAINERS, DNS_NAMES};

const COLUMNS: &[Column] = &[
    col("ID", "id"),
    col("NAME", "fqdn"),
    col("KIND", "kind"),
    col("STATUS", "status"),
    col("TARGET", "target"),
    col("PROJECT", "project.name"),
];

const DETAIL: &[Column] = &[
    col("ID", "id"),
    col("Name", "fqdn"),
    col("Kind", "kind"),
    col("SRV", "srv"),
    col("Status", "status"),
    col("Note", "status_note"),
    col("Target", "target"),
    col("Project", "project.name"),
];

pub fn run(cmd: &DnsCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        DnsCmd::List(filter) => {
            let records: Vec<Value> =
                DNS_NAMES.list(&client)?.into_iter().filter(|r| in_project(r, filter.project.as_deref())).collect();
            print_list(ctx.format, &records, COLUMNS, "No DNS names.");
            Ok(())
        }
        DnsCmd::Check { name } => {
            let answer = client.get_query("/api/v1/dns/check", &[("name", name)])?;
            let text = if answer["available"] == true {
                format!("{name}.in.mhon.cloud is free.")
            } else {
                format!("{name}.in.mhon.cloud isn't available: {}", answer["reason"].as_str().unwrap_or("taken"))
            };
            print_message(ctx.format, answer, &text);
            Ok(())
        }
        DnsCmd::Create { name, target } => {
            let body = request(&client, name, target)?;
            let answer = client.post("/api/v1/dns", Some(body))?;
            print_object(ctx.format, answer.get("record").unwrap_or(&answer), DETAIL);
            Ok(())
        }
        DnsCmd::Update { record, target } => {
            let existing = DNS_NAMES.resolve(&client, record, target.project.project.as_deref())?;
            let name = existing["name"].as_str().unwrap_or_default().to_string();
            let body = request(&client, &name, target)?;
            let answer = client.put(&format!("/api/v1/dns/{}", id_of(&existing)?), body)?;
            print_object(ctx.format, answer.get("record").unwrap_or(&answer), DETAIL);
            Ok(())
        }
        DnsCmd::Delete { record, confirm } => {
            let existing = DNS_NAMES.resolve(&client, record, None)?;
            let fqdn = existing["fqdn"].as_str().unwrap_or("the name").to_string();
            need_yes(confirm.yes, &format!("this frees {fqdn} (anyone may take it)"))?;
            client.delete(&format!("/api/v1/dns/{}", id_of(&existing)?))?;
            print_message(
                ctx.format,
                json!({ "deleted": true, "id": existing["id"], "fqdn": fqdn }),
                &format!("Deleted {fqdn}."),
            );
            Ok(())
        }
    }
}

/// The body for create / update: the container's project, the port (its HTTP port by default,
/// else its first), and the kind that port allows.
fn request(client: &Client, name: &str, target: &DnsTarget) -> Result<Value, CliError> {
    let container = CONTAINERS.resolve(client, &target.container, target.project.project.as_deref())?;
    let ports = container["ports"].as_array().cloned().unwrap_or_default();
    let port = match target.port {
        Some(p) => ports
            .iter()
            .find(|x| x["port"] == json!(p))
            .cloned()
            .ok_or_else(|| CliError::usage(format!("the container has no port {p}")))?,
        None => ports
            .iter()
            .find(|x| x["protocol"] == "http")
            .or(ports.first())
            .cloned()
            .ok_or_else(|| CliError::usage("the container has no ports to point a name at"))?,
    };
    let protocol = port["protocol"].as_str().unwrap_or("tcp").to_string();
    let kind = target.kind.unwrap_or(if protocol == "http" { DnsKind::Web } else { DnsKind::Direct });
    if kind == DnsKind::Web && protocol != "http" {
        return Err(CliError::usage(format!(
            "port {} is {protocol}: web names need an HTTP port (use --kind direct)",
            port["port"]
        )));
    }
    if target.minecraft_srv && (kind != DnsKind::Direct || protocol != "tcp") {
        return Err(CliError::usage("--minecraft-srv needs a direct name on a TCP port"));
    }
    Ok(json!({
        "project_id": container["project"]["id"],
        "name": name,
        "target_kind": "container",
        "container_id": container["id"],
        "port": port["port"],
        "kind": if kind == DnsKind::Web { "web" } else { "direct" },
        "protocol": protocol,
        "srv": if target.minecraft_srv { "minecraft" } else { "" },
    }))
}
