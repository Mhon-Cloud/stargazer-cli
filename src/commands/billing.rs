//! Account-wide reads: `bills` (GET /api/v1/billings[/{id}|/summary|/invoices[/{id}]]),
//! `inbox` (GET /api/v1/announcements, POST .../{id}/read, DELETE .../{id}), `usage`
//! (GET /api/v1/usage), `quotas` (GET /api/v1/quotas) and `pricing` (GET /api/v1/public/pricing,
//! no key needed).

use serde_json::{json, Value};

use crate::api::Client;
use crate::cli::{BillsCmd, InboxCmd, UsageArgs, UsageScope};
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_json, print_list, print_message, print_object, Column, Format};

use super::common::{items, CONTAINERS, SERVERS};

const BILLS: &[Column] = &[
    col("ID", "id"),
    col("NAME", "name"),
    col("KIND", "kind"),
    col("STATUS", "status"),
    col("AMOUNT", "amount"),
    col("ISSUED", "issued_at"),
    col("PAID", "paid_at"),
];

const ITEMS: &[Column] = &[
    col("CATEGORY", "category"),
    col("DESCRIPTION", "description"),
    col("QTY", "quantity"),
    col("UNIT", "unit"),
    col("AMOUNT", "amount"),
];

const INVOICES: &[Column] = &[
    col("ID", "id"),
    col("NUMBER", "number"),
    col("ISSUED", "issued_at"),
    col("NET", "net"),
    col("BILL_TO", "bill_to"),
];

const INBOX: &[Column] =
    &[col("ID", "id"), col("KIND", "kind"), col("TITLE", "title"), col("PUBLISHED", "publish_at"), col("READ", "read")];

const USAGE: &[Column] = &[
    col("Reporting", "summary.hosts"),
    col("CPU busy (cores)", "summary.cpu_used"),
    col("CPU total", "summary.cpu_total"),
    col("RAM used (bytes)", "summary.mem_used"),
    col("RAM total (bytes)", "summary.mem_total"),
    col("Disk (bytes)", "summary.disk_total"),
    col("Net in (B/s)", "summary.net_in_bps"),
    col("Net out (B/s)", "summary.net_out_bps"),
    col("Updated", "summary.updated_at"),
];

pub fn bills(cmd: &BillsCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        BillsCmd::List { from, to } => {
            let mut query = Vec::new();
            if let Some(f) = from {
                query.push(("from", f.as_str()));
            }
            if let Some(t) = to {
                query.push(("to", t.as_str()));
            }
            let answer = client.get_query("/api/v1/billings", &query)?;
            print_list(ctx.format, &items(answer, "billings"), BILLS, "No bills.");
            Ok(())
        }
        BillsCmd::Show { bill } => {
            let answer = client.get(&format!("/api/v1/billings/{bill}"))?;
            if ctx.format == Format::Json {
                print_json(&answer);
            } else {
                print_object(ctx.format, &answer["billing"], BILLS);
                print_list(
                    ctx.format,
                    answer["items"].as_array().map(Vec::as_slice).unwrap_or(&[]),
                    ITEMS,
                    "No items.",
                );
            }
            Ok(())
        }
        BillsCmd::Summary { range } => {
            let answer = client.get_query("/api/v1/billings/summary", &[("range", range.as_str())])?;
            let text = format!(
                "{} bill(s) paid from {} to {}: ฿{} in all (refunds subtracted).",
                answer["bills"].as_array().map_or(0, Vec::len),
                answer["from"].as_str().unwrap_or("?"),
                answer["to"].as_str().unwrap_or("?"),
                answer["total"]
            );
            print_message(ctx.format, answer, &text);
            Ok(())
        }
        BillsCmd::Invoices => {
            print_list(
                ctx.format,
                &items(client.get("/api/v1/billings/invoices")?, "invoices"),
                INVOICES,
                "No invoices.",
            );
            Ok(())
        }
        BillsCmd::Invoice { invoice } => {
            let answer = client.get(&format!("/api/v1/billings/invoices/{invoice}"))?;
            print_json(answer.get("invoice").unwrap_or(&answer));
            Ok(())
        }
    }
}

pub fn inbox(cmd: &InboxCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        InboxCmd::List { unread } => {
            let list: Vec<Value> = items(client.get("/api/v1/announcements")?, "announcements")
                .into_iter()
                .filter(|a| !unread || a["read"] != true)
                .collect();
            print_list(ctx.format, &list, INBOX, "Nothing in your inbox.");
            Ok(())
        }
        InboxCmd::Read { id } => {
            let item = items(client.get("/api/v1/announcements")?, "announcements")
                .into_iter()
                .find(|a| a["id"] == json!(id))
                .ok_or_else(|| CliError::not_found(format!("nothing \"{id}\" in your inbox")))?;
            client.post(&format!("/api/v1/announcements/{id}/read"), None)?;
            let text = format!(
                "{}\n{}\n\n{}",
                item["title"].as_str().unwrap_or(""),
                item["publish_at"].as_str().unwrap_or(""),
                item["description"].as_str().unwrap_or("")
            );
            print_message(ctx.format, item, &text);
            Ok(())
        }
        InboxCmd::Dismiss { id } => {
            let answer = client.delete(&format!("/api/v1/announcements/{id}"))?;
            print_message(ctx.format, json!({ "dismissed": true, "id": id, "message": answer["message"] }), "Removed.");
            Ok(())
        }
    }
}

pub fn usage(args: &UsageArgs, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    let scope = if args.scope == UsageScope::Containers { "containers" } else { "servers" };
    let mut query = vec![("scope", scope.to_string()), ("range", args.range.as_str().to_string())];
    if let Some(name) = &args.of {
        let (key, id) = match args.scope {
            UsageScope::Servers => ("server", SERVERS.resolve_id(&client, name, None)?),
            UsageScope::Containers => ("container", CONTAINERS.resolve_id(&client, name, None)?),
        };
        query.push((key, id));
    }
    let query: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let answer = client.get_query("/api/v1/usage", &query)?;
    print_object(ctx.format, &answer, USAGE);
    Ok(())
}

pub fn quotas(ctx: &Context) -> Result<(), CliError> {
    let answer = ctx.client()?.get("/api/v1/quotas")?;
    let quotas = answer.get("quotas").cloned().unwrap_or(answer);
    let text = quotas
        .as_object()
        .map(|m| m.iter().map(|(k, v)| format!("{k:<28} {v}")).collect::<Vec<_>>().join("\n"))
        .unwrap_or_default();
    print_message(ctx.format, quotas, &text);
    Ok(())
}

/// Public: works without a key (`Client::anonymous`).
pub fn pricing(ctx: &Context) -> Result<(), CliError> {
    let answer = Client::anonymous(&ctx.api_url).get("/api/v1/public/pricing")?;
    print_json(&answer);
    Ok(())
}
