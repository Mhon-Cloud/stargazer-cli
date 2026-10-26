//! `stargazer public-ips ...`: GET /api/v1/public-ips, GET /api/v1/market/public-ip/quote,
//! POST /api/v1/market/public-ip (reserve), POST/DELETE /api/v1/public-ips/{id}/bind,
//! DELETE /api/v1/public-ips/{id} (release). Reserved addresses are paid monthly from the
//! buyer's credit and bind to one server of their project and location (applied at a restart).

use serde_json::{json, Value};

use crate::api::Client;
use crate::cli::PublicIpsCmd;
use crate::context::Context;
use crate::error::{CliError, ErrorKind};
use crate::output::{col, print_list, print_message, print_object, Column};

use super::common::{find, id_of, in_project, need_yes, order_project, order_zone, PUBLIC_IPS, SERVERS};

const COLUMNS: &[Column] = &[
    col("ID", "id"),
    col("ADDRESS", "address"),
    col("SOURCE", "source"),
    col("SERVER", "server.name"),
    col("BIND", "bind_status"),
    col("PAID_UNTIL", "reserved_until"),
    col("ZONE", "zone.name"),
    col("PROJECT", "project.name"),
];

const DETAIL: &[Column] = &[
    col("ID", "id"),
    col("Address", "address"),
    col("Source", "source"),
    col("Server", "server.name"),
    col("Bind status", "bind_status"),
    col("Bind error", "bind_error"),
    col("Paid until", "reserved_until"),
    col("Zone", "zone.name"),
    col("Project", "project.name"),
];

const QUOTE: &[Column] = &[
    col("Address (month)", "rate"),
    col("Reservation fee (month)", "fee"),
    col("Total (month)", "total"),
    col("Your credit", "credit"),
    col("Enough credit", "enough_credit"),
    col("Free addresses", "available"),
    col("Locations", "zones"),
];

pub fn run(cmd: &PublicIpsCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        PublicIpsCmd::List(filter) => {
            let ips: Vec<Value> =
                PUBLIC_IPS.list(&client)?.into_iter().filter(|i| in_project(i, filter.project.as_deref())).collect();
            print_list(ctx.format, &ips, COLUMNS, "No public IPs.");
            Ok(())
        }
        PublicIpsCmd::Quote => {
            print_object(ctx.format, &client.get("/api/v1/market/public-ip/quote")?, QUOTE);
            Ok(())
        }
        PublicIpsCmd::Reserve { project, zone, confirm } => {
            let quote = client.get("/api/v1/market/public-ip/quote")?;
            let project = order_project(&client, project.as_deref())?;
            // Only locations with a free address (the quote lists them).
            let open: Vec<Value> = quote["zones"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|z| z["available"].as_u64().unwrap_or(0) > 0)
                .cloned()
                .collect();
            let zone = match zone {
                Some(z) => order_zone(&client, Some(z), None)?,
                None => match open.as_slice() {
                    [only] => only.clone(),
                    [] => {
                        return Err(CliError::new(
                            ErrorKind::Conflict,
                            "no public IP is free in any location right now",
                        ))
                    }
                    _ => {
                        let names: Vec<&str> = open.iter().filter_map(|z| z["name"].as_str()).collect();
                        return Err(CliError::usage(format!("say which location with --zone: {}", names.join(", "))));
                    }
                },
            };
            if quote["enough_credit"] == false {
                return Err(CliError::new(
                    ErrorKind::PaymentRequired,
                    format!(
                        "not enough credit: an address costs ฿{} a month; top up first",
                        quote["total"].as_str().unwrap_or("?")
                    ),
                ));
            }
            need_yes(
                confirm.yes,
                &format!(
                    "this pays ฿{} now and every month from your credit until you release it (no refund)",
                    quote["total"].as_str().unwrap_or("?")
                ),
            )?;
            let answer = client.post(
                "/api/v1/market/public-ip",
                Some(json!({ "project_id": id_of(&project)?, "availability_zone_id": id_of(&zone)? })),
            )?;
            let text = format!(
                "Reserved {} (paid until {}).",
                answer["address"].as_str().unwrap_or("?"),
                answer["reserved_until"].as_str().unwrap_or("?")
            );
            print_message(ctx.format, answer, &text);
            Ok(())
        }
        PublicIpsCmd::Bind { ip, server } => {
            let ip = resolve(&client, ip)?;
            let server = SERVERS.resolve_id(&client, server, ip["project"]["id"].as_str())?;
            let answer = client
                .post(&format!("/api/v1/public-ips/{}/bind", id_of(&ip)?), Some(json!({ "server_id": server })))?;
            print_object(ctx.format, answer.get("public_ip").unwrap_or(&answer), DETAIL);
            Ok(())
        }
        PublicIpsCmd::Unbind { ip } => {
            let ip = resolve(&client, ip)?;
            let answer = client.delete(&format!("/api/v1/public-ips/{}/bind", id_of(&ip)?))?;
            print_object(ctx.format, answer.get("public_ip").unwrap_or(&answer), DETAIL);
            Ok(())
        }
        PublicIpsCmd::Release { ip, confirm } => {
            let ip = resolve(&client, ip)?;
            let address = ip["address"].as_str().unwrap_or("the address").to_string();
            need_yes(confirm.yes, &format!("this gives {address} up (no refund for the paid month)"))?;
            let answer = client.delete(&format!("/api/v1/public-ips/{}", id_of(&ip)?))?;
            print_message(
                ctx.format,
                json!({ "released": true, "address": address, "message": answer["message"] }),
                &format!("Released {address}."),
            );
            Ok(())
        }
    }
}

fn resolve(client: &Client, needle: &str) -> Result<Value, CliError> {
    let list = PUBLIC_IPS.list(client)?;
    Ok(find(&list, needle, None, PUBLIC_IPS.names, PUBLIC_IPS.noun)?.clone())
}
