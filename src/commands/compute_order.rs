//! Ordering servers (Market -> Compute Instances): GET /api/v1/market/compute/options,
//! GET .../images (a project's own OS: its Proxmox templates), POST .../quote (priced, nothing charged),
//! POST/GET .../orders, POST .../orders/{id}/cancel.
//! The server prices everything; `create` shows its quote and pays only with --yes.

use serde_json::{json, Value};

use crate::api::Client;
use crate::cli::{OrdersCmd, ServerCreate};
use crate::context::Context;
use crate::error::{CliError, ErrorKind};
use crate::output::{col, print_json, print_list, print_message, print_object, Column, Format};

use super::common::{
    id_of, my_username, need_yes, order_project, order_zone, random_password, read_stdin_line, wait_for, ORDERS,
    SSH_KEYS,
};

const ORDER_COLUMNS: &[Column] = &[
    col("NUMBER", "number"),
    col("NAME", "name"),
    col("STATUS", "status"),
    col("CPU", "cpu_cores"),
    col("RAM_GB", "ram_gb"),
    col("OS", "os"),
    col("ZONE", "zone"),
    col("TOTAL", "total"),
    col("PAID_UNTIL", "expires_at"),
];

const ORDER_DETAIL: &[Column] = &[
    col("Order", "order.number"),
    col("Status", "order.status"),
    col("Name", "order.name"),
    col("vCPU", "order.cpu_cores"),
    col("RAM (GB)", "order.ram_gb"),
    col("OS", "order.os"),
    col("Location", "order.zone"),
    col("Public IP", "order.public_ip_address"),
    col("DNS name", "order.dns_name"),
    col("Paid", "order.total"),
    col("Paid until", "order.expires_at"),
    col("Note", "order.provision_note"),
    col("Username", "login.username"),
    col("Password", "login.password"),
];

const IMAGE_COLUMNS: &[Column] = &[
    col("NAME", "name"),
    col("OS", "os"),
    col("KIND", "kind"),
    col("STATE", "state"),
    col("SHARED", "visibility"),
    col("MIN_DISK_GB", "min_disk_gb"),
    col("NOTE", "note"),
];

const QUOTE: &[Column] = &[
    col("Monthly", "monthly"),
    col("Months", "period_months"),
    col("Subtotal", "subtotal"),
    col("Plan", "package.name"),
    col("Plan discount", "package.discount"),
    col("Period discount", "period_discount"),
    col("Coupon", "coupon.code"),
    col("Coupon discount", "coupon.discount"),
    col("Coupon problem", "coupon_error"),
    col("Total", "total"),
    col("Your credit", "credit"),
    col("Enough credit", "enough_credit"),
];

pub fn options(ctx: &Context, client: &Client) -> Result<(), CliError> {
    let options = client.get("/api/v1/market/compute/options")?;
    if ctx.format == Format::Json {
        print_json(&options);
        return Ok(());
    }
    let os: Vec<&str> = options["os"].as_array().into_iter().flatten().filter_map(|o| o["name"].as_str()).collect();
    let plans: Vec<String> = options["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            format!(
                "  {:<12} {} vCPU, {} GB RAM, {} GB {}{}  (-{}%)",
                p["name"].as_str().unwrap_or("?"),
                p["cpu_cores"],
                p["ram_gb"],
                p["storage_gb"],
                p["storage_type"].as_str().unwrap_or("ssd").to_uppercase(),
                if p["public_ip"] == true { ", public IP" } else { "" },
                p["discount"].as_str().unwrap_or("0"),
            )
        })
        .collect();
    let r = &options["rates"];
    let l = &options["limits"];
    let text = format!(
        "OS images: {}\nPlans (--plan):\n{}\nMonthly rates (฿): vCPU {}, GB RAM {}, GB SSD {}, GB HDD {}, public IP {}\nDiscounts: 6 months {}%, 12 months {}%\nLimits: {}-{} vCPU, {}-{} GB RAM, {} disks of {}-{} GB\nYour credit: ฿{}",
        if os.is_empty() { "none available right now".to_string() } else { os.join(", ") },
        if plans.is_empty() { "  none".to_string() } else { plans.join("\n") },
        r["cpu"].as_str().unwrap_or("?"),
        r["ram"].as_str().unwrap_or("?"),
        r["ssd"].as_str().unwrap_or("?"),
        r["hdd"].as_str().unwrap_or("?"),
        r["public_ip"].as_str().unwrap_or("?"),
        r["discount_6_months"].as_str().unwrap_or("0"),
        r["discount_year"].as_str().unwrap_or("0"),
        l["min_cores"], l["max_cores"], l["min_ram_gb"], l["max_ram_gb"], l["max_disks"], l["min_disk_gb"], l["max_disk_gb"],
        options["credit"].as_str().unwrap_or("?"),
    );
    print_message(ctx.format, options, &text);
    Ok(())
}

/// The locations VPS orders can go to (`vps_zone_ids`; None from a gateway that doesn't say).
fn vps_zones(options: &Value) -> Option<Vec<String>> {
    options["vps_zone_ids"].as_array().map(|ids| ids.iter().filter_map(|v| v.as_str().map(String::from)).collect())
}

/// GB needed to hold `bytes` (rounded up); None when unknown.
fn min_disk_gb(bytes: &Value) -> Option<u64> {
    bytes.as_u64().filter(|b| *b > 0).map(|b| b.div_ceil(1 << 30))
}

/// The project's OS a server (no VPC) can be made from in a location: its own ready Proxmox templates, then other
/// projects' public ones. Each item: os, name, kind, state, visibility, min_disk_gb, note.
fn own_os(images: &Value, zone_id: &str, ready_only: bool) -> Vec<Value> {
    let in_zone = |v: &Value| v["availability_zone_id"].as_str() == Some(zone_id) || zone_id.is_empty();
    let mut out = Vec::new();
    for i in images["images"].as_array().into_iter().flatten() {
        if i["source"] != "proxmox" || !in_zone(i) || (ready_only && i["state"] != "ready") {
            continue;
        }
        out.push(json!({
            "os": i["os"], "name": i["name"], "kind": "project template", "state": i["state"],
            "visibility": i["visibility"], "min_disk_gb": min_disk_gb(&i["virtual_size_bytes"]), "note": i["note"],
        }));
    }
    for c in images["community"].as_array().into_iter().flatten() {
        if c["source"] != "proxmox" || !in_zone(c) {
            continue;
        }
        out.push(json!({
            "os": c["os"], "name": c["name"], "kind": "community template", "state": "ready",
            "visibility": "public (another project's)", "min_disk_gb": min_disk_gb(&c["virtual_size_bytes"]), "note": Value::Null,
        }));
    }
    out
}

fn images_of(client: &Client, project_id: &str, zone_id: &str) -> Result<Value, CliError> {
    client.get_query("/api/v1/market/compute/images", &[("project_id", project_id), ("availability_zone_id", zone_id)])
}

/// `servers images`: the project's own OS for servers (its Proxmox templates) and other projects' public ones.
pub fn images(ctx: &Context, client: &Client, project: Option<&str>, zone: Option<&str>) -> Result<(), CliError> {
    let options = client.get("/api/v1/market/compute/options")?;
    let project = order_project(client, project)?;
    let zone = order_zone(client, zone, vps_zones(&options).as_deref())?;
    let zone_id = id_of(&zone)?.to_string();
    let answer = images_of(client, id_of(&project)?, &zone_id)?;
    if ctx.format == Format::Json {
        print_json(&answer);
        return Ok(());
    }
    print_list(
        ctx.format,
        &own_os(&answer, &zone_id, false),
        IMAGE_COLUMNS,
        "No own OS images here: an administrator puts a Proxmox template (with a cloud-init drive) in the project's pool.",
    );
    Ok(())
}

pub fn orders(cmd: &OrdersCmd, ctx: &Context, client: &Client) -> Result<(), CliError> {
    match cmd {
        OrdersCmd::List => {
            print_list(ctx.format, &ORDERS.list(client)?, ORDER_COLUMNS, "No orders.");
            Ok(())
        }
        OrdersCmd::Cancel { order, confirm } => {
            let order = ORDERS.resolve(client, order, None)?;
            need_yes(confirm.yes, "this cancels the order (the full price comes back to your credit)")?;
            let answer = client.post(&format!("/api/v1/market/compute/orders/{}/cancel", id_of(&order)?), None)?;
            print_object(ctx.format, &answer, ORDER_DETAIL);
            Ok(())
        }
    }
}

pub fn create(args: &ServerCreate, ctx: &Context, client: &Client) -> Result<(), CliError> {
    let options = client.get("/api/v1/market/compute/options")?;
    let project = order_project(client, args.project.as_deref())?;
    let project_id = id_of(&project)?.to_string();
    let zone = order_zone(client, args.zone.as_deref(), vps_zones(&options).as_deref())?;
    let zone_id = id_of(&zone)?.to_string();
    let (os, os_min_gb) = pick_os(client, &options, &project_id, &zone_id, &args.os)?;

    let plan = match &args.plan {
        Some(name) => Some(pick_plan(&options["packages"], name)?),
        None => None,
    };
    let cpu = args.cpu.or(plan.as_ref().and_then(|p| p["cpu_cores"].as_u64().map(|v| v as u32)));
    let ram = args.ram.or(plan.as_ref().and_then(|p| p["ram_gb"].as_u64().map(|v| v as u32)));
    let (Some(cpu), Some(ram)) = (cpu, ram) else {
        return Err(CliError::usage("give --plan, or --cpu and --ram (see `stargazer servers options`)"));
    };
    let disks = if !args.disks.is_empty() {
        args.disks.iter().map(|d| parse_disk(d)).collect::<Result<Vec<_>, _>>()?
    } else if let Some(p) = &plan {
        vec![json!({ "type": p["storage_type"].as_str().unwrap_or("ssd"), "size_gb": p["storage_gb"] })]
    } else {
        // At least the template's disk (the server refuses a smaller boot disk).
        let min = options["limits"]["min_disk_gb"].as_u64().unwrap_or(10).max(20).max(os_min_gb.unwrap_or(0));
        vec![json!({ "type": "ssd", "size_gb": min })]
    };
    let public_ip = args.public_ip || plan.as_ref().is_some_and(|p| p["public_ip"] == true);

    let username = match &args.username {
        Some(u) => u.clone(),
        None => login_name(my_username(client)?),
    };
    let ssh_keys: Vec<String> =
        args.ssh_keys.iter().map(|k| SSH_KEYS.resolve_id(client, k, Some(&project_id))).collect::<Result<_, _>>()?;
    let (password, generated) = if args.no_password {
        if ssh_keys.is_empty() {
            return Err(CliError::usage("--no-password needs at least one --ssh-key to log in with"));
        }
        (String::new(), false)
    } else if args.password_stdin {
        (read_stdin_line("password")?, false)
    } else {
        (random_password()?, true)
    };

    let mut body = json!({
        "project_id": project_id,
        "name": args.name,
        "cpu_cores": cpu,
        "ram_gb": ram,
        "disks": disks,
        "os": os,
        "availability_zone_id": zone_id,
        "public_ip": public_ip,
        "period_months": args.period,
        "username": username,
        "password": password,
        "credential_ids": ssh_keys,
    });
    if !public_ip {
        body["dns_label"] = json!(args.dns_label.clone().unwrap_or_else(|| args.name.clone()));
    }
    if let Some(c) = &args.coupon {
        body["coupon_code"] = json!(c);
    }
    if let Some(when) = &args.backup {
        let (day, time) = parse_backup(when)?;
        body["auto_backup"] = json!(true);
        body["backup_day"] = json!(day);
        body["backup_time"] = json!(time);
    }

    let quote = client.post("/api/v1/market/compute/quote", Some(body.clone()))?;
    if args.dry_run {
        print_object(ctx.format, &quote, QUOTE);
        return Ok(());
    }
    // The gateway answers "" (not null) when the coupon is fine or none was given.
    if let Some(problem) = quote["coupon_error"].as_str().filter(|p| !p.is_empty()) {
        return Err(CliError::usage(format!("the coupon can't be used: {problem}")));
    }
    if quote["enough_credit"] == false {
        return Err(CliError::new(
            ErrorKind::PaymentRequired,
            format!(
                "not enough credit: this costs ฿{} and you have ฿{}; top up in the console (Overview > Billing)",
                quote["total"].as_str().unwrap_or("?"),
                quote["credit"].as_str().unwrap_or("?")
            ),
        ));
    }
    need_yes(
        args.confirm.yes,
        &format!(
            "this pays ฿{} for {} month(s) from your credit (then ฿{}/month); --dry-run shows the quote",
            quote["total"].as_str().unwrap_or("?"),
            args.period,
            quote["monthly"].as_str().unwrap_or("?")
        ),
    )?;

    let answer = client.post("/api/v1/market/compute/orders", Some(body))?;
    let mut order = answer.get("order").cloned().unwrap_or(answer);
    if args.wait.wait {
        let id = id_of(&order)?.to_string();
        order = wait_for(args.wait.timeout, "the server to be created", || {
            let Some(o) = ORDERS.list(client)?.into_iter().find(|o| o["id"] == id) else { return Ok(None) };
            match o["status"].as_str() {
                Some("active") => Ok(Some(o)),
                Some("failed") | Some("cancelled") => Err(CliError::new(
                    ErrorKind::Conflict,
                    format!(
                        "the order couldn't be completed ({}); anything you paid is back in your credit",
                        o["provision_note"].as_str().unwrap_or("no reason given")
                    ),
                )),
                _ => Ok(None),
            }
        })?;
    }
    let login = if generated {
        json!({ "username": username, "password": password, "note": "also sent to your inbox (Overview > Inbox)" })
    } else {
        json!({ "username": username })
    };
    print_object(ctx.format, &json!({ "order": order, "login": login }), ORDER_DETAIL);
    Ok(())
}

/// The OS id for a name or id the user typed (case-insensitive): the platform's images, else the project's own ready
/// ones in the location (`servers images`), with the smallest boot disk they need (GB); or a usage error listing them.
fn pick_os(
    client: &Client,
    options: &Value,
    project_id: &str,
    zone_id: &str,
    wanted: &str,
) -> Result<(String, Option<u64>), CliError> {
    let matches = |o: &Value| {
        [&o["id"], &o["os"], &o["name"]].iter().any(|v| v.as_str().is_some_and(|s| s.eq_ignore_ascii_case(wanted)))
    };
    let list = options["os"].as_array().cloned().unwrap_or_default();
    if let Some(o) = list.iter().find(|o| matches(o)) {
        return Ok((o["id"].as_str().or(o["name"].as_str()).unwrap_or(wanted).to_string(), None));
    }
    // A gateway without the images list (older, 404) just has the platform's; any other failure is the user's to see.
    let own = match images_of(client, project_id, zone_id) {
        Ok(answer) => own_os(&answer, zone_id, true),
        Err(e) if e.kind == ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e),
    };
    if let Some(o) = own.iter().find(|o| matches(o)) {
        return Ok((o["os"].as_str().unwrap_or(wanted).to_string(), o["min_disk_gb"].as_u64()));
    }
    let names: Vec<&str> =
        list.iter().filter_map(|o| o["name"].as_str()).chain(own.iter().filter_map(|o| o["name"].as_str())).collect();
    Err(CliError::usage(format!(
        "no OS image \"{wanted}\" in this project and location; choose one of: {}",
        names.join(", ")
    )))
}

/// A plan by name (case-insensitive) from options' `packages` / `container_packages`.
pub fn pick_plan(plans: &Value, name: &str) -> Result<Value, CliError> {
    let list = plans.as_array().cloned().unwrap_or_default();
    list.iter().find(|p| p["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(name))).cloned().ok_or_else(|| {
        let names: Vec<&str> = list.iter().filter_map(|p| p["name"].as_str()).collect();
        CliError::usage(format!("no plan \"{name}\"; choose one of: {}", names.join(", ")))
    })
}

/// `ssd:50` -> {"type": "ssd", "size_gb": 50}.
fn parse_disk(text: &str) -> Result<Value, CliError> {
    let bad = || CliError::usage(format!("a disk is TYPE:GB, e.g. ssd:50 or hdd:200 (got \"{text}\")"));
    let (kind, size) = text.split_once(':').ok_or_else(bad)?;
    let kind = kind.to_ascii_lowercase();
    let size: u32 = size.parse().map_err(|_| bad())?;
    if kind != "ssd" && kind != "hdd" {
        return Err(bad());
    }
    Ok(json!({ "type": kind, "size_gb": size }))
}

/// `daily@03:00` / `sun@02:30` -> ("daily", "03:00").
fn parse_backup(text: &str) -> Result<(String, String), CliError> {
    let bad = || CliError::usage(format!("--backup is DAY@HH:MM, DAY = daily or mon...sun (got \"{text}\")"));
    let (day, time) = text.split_once('@').ok_or_else(bad)?;
    let day = day.to_ascii_lowercase();
    let days = ["daily", "mon", "tue", "wed", "thu", "fri", "sat", "sun"];
    let time_ok = time.len() == 5
        && time.as_bytes()[2] == b':'
        && time[..2].parse::<u8>().is_ok_and(|h| h < 24)
        && time[3..].parse::<u8>().is_ok_and(|m| m < 60);
    if !days.contains(&day.as_str()) || !time_ok {
        return Err(bad());
    }
    Ok((day, time.to_string()))
}

/// A login name from the account's username (the console's suggestion): lower-case letters,
/// digits, _ and -, starting with a letter; "user" when nothing usable is left.
pub fn login_name(username: Option<String>) -> String {
    let cleaned: String = username
        .unwrap_or_default()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .skip_while(|c| !c.is_ascii_lowercase())
        .take(32)
        .collect();
    const SYSTEM: &[&str] = &["root", "admin", "ubuntu", "daemon", "bin", "sys", "nobody", "sshd"];
    if cleaned.is_empty() || SYSTEM.contains(&cleaned.as_str()) {
        "user".to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disks_parse() {
        assert_eq!(parse_disk("SSD:50").unwrap(), json!({ "type": "ssd", "size_gb": 50 }));
        assert!(parse_disk("nvme:50").is_err());
        assert!(parse_disk("ssd").is_err());
    }

    #[test]
    fn backups_parse() {
        assert_eq!(parse_backup("daily@03:00").unwrap(), ("daily".to_string(), "03:00".to_string()));
        assert_eq!(parse_backup("Sun@23:59").unwrap().0, "sun");
        assert!(parse_backup("sunday@03:00").is_err());
        assert!(parse_backup("daily@25:00").is_err());
    }

    #[test]
    fn login_names_are_safe() {
        assert_eq!(login_name(Some("Mhon.Nathadon".into())), "mhonnathadon");
        assert_eq!(login_name(Some("42web".into())), "web");
        assert_eq!(login_name(Some("42admin".into())), "user");
        assert_eq!(login_name(None), "user");
        assert_eq!(login_name(Some("root".into())), "user");
    }
}
