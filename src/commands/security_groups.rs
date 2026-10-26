//! `stargazer security-groups ...`: GET/POST /api/v1/security-groups, PUT/DELETE .../{id},
//! POST/DELETE .../{id}/rules[/{ruleId}], POST/DELETE .../{id}/attachments[/{serverId}].
//! Allow-only rules, written to the servers' firewalls at once (no restart).

use serde_json::{json, Value};

use crate::cli::{Direction, Protocol, SecurityGroupsCmd};
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_list, print_message, print_object, Column, Format};

use super::common::{id_of, in_project, need_yes, order_project, SECURITY_GROUPS, SERVERS};

const COLUMNS: &[Column] = &[
    col("ID", "id"),
    col("NAME", "name"),
    col("RULES", "rules_count"),
    col("SERVERS", "servers"),
    col("PROJECT", "project.name"),
];

const DETAIL: &[Column] = &[
    col("ID", "id"),
    col("Name", "name"),
    col("Description", "description"),
    col("Rules", "rules_text"),
    col("Servers", "servers"),
    col("Project", "project.name"),
];

pub fn run(cmd: &SecurityGroupsCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        SecurityGroupsCmd::List(filter) => {
            let groups: Vec<Value> = SECURITY_GROUPS
                .list(&client)?
                .into_iter()
                .filter(|g| in_project(g, filter.project.as_deref()))
                .map(|g| with_summary(g, false, ctx.format))
                .collect();
            print_list(ctx.format, &groups, COLUMNS, "No security groups.");
            Ok(())
        }
        SecurityGroupsCmd::Show { group } => show(ctx, SECURITY_GROUPS.resolve(&client, group, None)?),
        SecurityGroupsCmd::Create { name, description, project } => {
            let project = order_project(&client, project.as_deref())?;
            let answer = client.post(
                "/api/v1/security-groups",
                Some(json!({ "project_id": id_of(&project)?, "name": name, "description": description })),
            )?;
            show(ctx, answer.get("security_group").cloned().unwrap_or(answer))
        }
        SecurityGroupsCmd::Rename { group, new_name, description } => {
            let found = SECURITY_GROUPS.resolve(&client, group, None)?;
            let description =
                description.clone().unwrap_or_else(|| found["description"].as_str().unwrap_or("").to_string());
            let answer = client.put(
                &format!("/api/v1/security-groups/{}", id_of(&found)?),
                json!({ "name": new_name, "description": description }),
            )?;
            show(ctx, answer.get("security_group").cloned().unwrap_or(answer))
        }
        SecurityGroupsCmd::Delete { group, confirm } => {
            let found = SECURITY_GROUPS.resolve(&client, group, None)?;
            need_yes(confirm.yes, "this deletes the group and its rules")?;
            let answer = client.delete(&format!("/api/v1/security-groups/{}", id_of(&found)?))?;
            print_message(
                ctx.format,
                json!({ "deleted": true, "id": found["id"], "message": answer["message"] }),
                "Deleted.",
            );
            Ok(())
        }
        SecurityGroupsCmd::Allow { group, direction, protocol, port, cidr, peer, description } => {
            let found = SECURITY_GROUPS.resolve(&client, group, None)?;
            let mut body = json!({
                "direction": match direction { Direction::Ingress => "ingress", Direction::Egress => "egress" },
                "protocol": match protocol { Protocol::All => "all", Protocol::Tcp => "tcp", Protocol::Udp => "udp", Protocol::Icmp => "icmp" },
                "description": description,
            });
            match (port, protocol) {
                (Some(p), Protocol::Tcp | Protocol::Udp) => {
                    let (from, to) = parse_ports(p)?;
                    body["from_port"] = json!(from);
                    body["to_port"] = json!(to);
                }
                (Some(_), _) => return Err(CliError::usage("--port is for tcp / udp rules")),
                (None, _) => {}
            }
            match peer {
                Some(p) => {
                    body["peer_group_id"] =
                        json!(SECURITY_GROUPS.resolve_id(&client, p, found["project"]["id"].as_str())?)
                }
                None => body["cidr"] = json!(cidr.clone().unwrap_or_else(|| "0.0.0.0/0".to_string())),
            }
            let answer = client.post(&format!("/api/v1/security-groups/{}/rules", id_of(&found)?), Some(body))?;
            show(ctx, answer.get("security_group").cloned().unwrap_or(answer))
        }
        SecurityGroupsCmd::Revoke { group, rule } => {
            let found = SECURITY_GROUPS.resolve(&client, group, None)?;
            let answer = client.delete(&format!("/api/v1/security-groups/{}/rules/{rule}", id_of(&found)?))?;
            show(ctx, answer.get("security_group").cloned().unwrap_or(answer))
        }
        SecurityGroupsCmd::Attach { group, server } => {
            let found = SECURITY_GROUPS.resolve(&client, group, None)?;
            let server = SERVERS.resolve_id(&client, server, found["project"]["id"].as_str())?;
            let answer = client.post(
                &format!("/api/v1/security-groups/{}/attachments", id_of(&found)?),
                Some(json!({ "server_id": server })),
            )?;
            show(ctx, answer.get("security_group").cloned().unwrap_or(answer))
        }
        SecurityGroupsCmd::Detach { group, server } => {
            let found = SECURITY_GROUPS.resolve(&client, group, None)?;
            let server = SERVERS.resolve_id(&client, server, found["project"]["id"].as_str())?;
            let answer = client.delete(&format!("/api/v1/security-groups/{}/attachments/{server}", id_of(&found)?))?;
            show(ctx, answer.get("security_group").cloned().unwrap_or(answer))
        }
    }
}

fn show(ctx: &Context, group: Value) -> Result<(), CliError> {
    print_object(ctx.format, &with_summary(group, true, ctx.format), DETAIL);
    Ok(())
}

/// Adds table-only summaries (`rules_count`, `rules_text`, `servers`) in table mode; JSON stays
/// the API's object.
fn with_summary(mut group: Value, detail: bool, format: Format) -> Value {
    if format == Format::Json {
        return group;
    }
    let rules = group["rules"].as_array().cloned().unwrap_or_default();
    group["rules_count"] = json!(rules.len());
    if detail {
        let lines: Vec<String> = rules.iter().map(rule_text).collect();
        group["rules_text"] = json!(if lines.is_empty() { "none".to_string() } else { lines.join("\n  ") });
    }
    let servers: Vec<Value> = group["attachments"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|a| a["server"]["name"].clone())
        .filter(|n| !n.is_null())
        .collect();
    group["servers"] = json!(servers);
    group
}

/// `ingress tcp 22 from 0.0.0.0/0 (rule-id)`.
fn rule_text(r: &Value) -> String {
    let ports = match (r["from_port"].as_u64(), r["to_port"].as_u64()) {
        (Some(a), Some(b)) if a == b => format!(" {a}"),
        (Some(a), Some(b)) => format!(" {a}-{b}"),
        _ => String::new(),
    };
    let other =
        r["cidr"].as_str().map(str::to_string).or(r["peer_group"]["name"].as_str().map(|n| format!("group {n}")));
    let dir = if r["direction"] == "egress" { "to" } else { "from" };
    format!(
        "{} {}{ports} {dir} {}  ({})",
        r["direction"].as_str().unwrap_or("?"),
        r["protocol"].as_str().unwrap_or("?"),
        other.unwrap_or_else(|| "anywhere".into()),
        r["id"].as_str().unwrap_or("?")
    )
}

/// `22` -> (22, 22); `8000-8100` -> (8000, 8100).
fn parse_ports(text: &str) -> Result<(u32, u32), CliError> {
    let bad = || CliError::usage(format!("--port is a port or a range: 22, 8000-8100 (got \"{text}\")"));
    let (a, b) = text.split_once('-').unwrap_or((text, text));
    let (a, b): (u32, u32) = (a.trim().parse().map_err(|_| bad())?, b.trim().parse().map_err(|_| bad())?);
    if a == 0 || b > 65535 || a > b {
        return Err(bad());
    }
    Ok((a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports_parse() {
        assert_eq!(parse_ports("22").unwrap(), (22, 22));
        assert_eq!(parse_ports("8000-8100").unwrap(), (8000, 8100));
        assert!(parse_ports("90-80").is_err());
        assert!(parse_ports("ssh").is_err());
    }

    #[test]
    fn rules_read_well() {
        let r = json!({ "id": "r1", "direction": "ingress", "protocol": "tcp", "from_port": 22, "to_port": 22, "cidr": "0.0.0.0/0" });
        assert_eq!(rule_text(&r), "ingress tcp 22 from 0.0.0.0/0  (r1)");
    }
}
