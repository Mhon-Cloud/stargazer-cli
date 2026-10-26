//! Ordering containers (Market -> Containers): GET /api/v1/market/containers/options,
//! POST /api/v1/market/containers, plus folder uploads (upload.rs). Like the console, the CLI
//! fills in what an image needs (ports, variables, a DNS name, a login for Ubuntu) and
//! estimates the price from the published rates; the server prices the checkout.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::api::Client;
use crate::cli::ContainerCreate;
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_json, print_list, print_message, print_object, Column, Format};

use super::catalog::{self, Image};
use super::common::{id_of, my_username, need_yes, order_project, order_zone, random_password, read_stdin_line};
use super::compute_order::{login_name, pick_plan};
use super::containers::{wait_until_status, DETAIL};
use super::upload;

const IMAGE_COLUMNS: &[Column] = &[
    col("ALIAS", "alias"),
    col("IMAGE", "image"),
    col("PORTS", "ports"),
    col("NEEDS", "required_env"),
    col("ABOUT", "name"),
];

const RESULT: &[Column] = &[
    col("ID", "container.id"),
    col("Number", "container.number"),
    col("Name", "container.name"),
    col("Status", "container.status"),
    col("Free", "container.free"),
    col("Paid", "container.total"),
    col("Address", "container.address"),
    col("DNS name", "dns_name"),
    col("Login user", "login.username"),
    col("Password", "login.password"),
    col("Uploaded", "uploads"),
];

const STARTUP_SCRIPT_MAX: u64 = 16 << 10;

pub fn images(ctx: &Context) -> Result<(), CliError> {
    let list = catalog::as_json();
    print_list(ctx.format, list.as_array().map(Vec::as_slice).unwrap_or(&[]), IMAGE_COLUMNS, "No images.");
    Ok(())
}

pub fn options(ctx: &Context, client: &Client) -> Result<(), CliError> {
    let containers = client.get("/api/v1/market/containers/options")?;
    let compute = client.get("/api/v1/market/compute/options")?;
    let all = json!({
        "free": containers,
        "rates": compute["container_rates"],
        "packages": compute["container_packages"],
        "credit": compute["credit"],
    });
    if ctx.format == Format::Json {
        print_json(&all);
        return Ok(());
    }
    let f = &all["free"];
    let r = &all["rates"];
    let plans: Vec<String> = all["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            format!(
                "  {:<12} {} vCPU, {} GB RAM, {} GB  (-{}%)",
                p["name"].as_str().unwrap_or("?"),
                p["cpu_cores"],
                p["ram_gb"],
                p["storage_gb"],
                p["discount"].as_str().unwrap_or("0")
            )
        })
        .collect();
    let text = format!(
        "Free plan: {} vCPU / {} GB RAM / {} GB, {} of {} used{}\nPlans (--plan):\n{}\nMonthly rates (฿): vCPU {}, GB RAM {}, GB storage {}\nDiscounts: 6 months {}%, 12 months {}%\nYour credit: ฿{}",
        f["free"]["cpu_cores"], f["free"]["ram_gb"], f["free"]["storage_gb"], f["free_used"], f["free_limit"],
        if f["free_available"] == true { "" } else { " (none left)" },
        if plans.is_empty() { "  none".to_string() } else { plans.join("\n") },
        r["cpu"].as_str().unwrap_or("?"), r["ram"].as_str().unwrap_or("?"), r["storage"].as_str().unwrap_or("?"),
        r["discount_6_months"].as_str().unwrap_or("0"), r["discount_year"].as_str().unwrap_or("0"),
        all["credit"].as_str().unwrap_or("?"),
    );
    print_message(ctx.format, all, &text);
    Ok(())
}

pub fn create(args: &ContainerCreate, ctx: &Context, client: &Client) -> Result<(), CliError> {
    let known = catalog::find(&args.image);
    let image = known.map(|i| i.image.to_string()).unwrap_or_else(|| args.image.clone());
    if image.starts_with("mhon.local/") && known.is_none() {
        return Err(CliError::usage("mhon.local/ images are the platform's own; use --image ubuntu"));
    }
    if image.contains(char::is_whitespace) || image.is_empty() {
        return Err(CliError::usage(format!("\"{}\" isn't an image reference (e.g. nginx:1.27)", args.image)));
    }
    let is_dev = known.is_some_and(|i| i.login);

    let free_options = client.get("/api/v1/market/containers/options")?;
    let compute_options = client.get("/api/v1/market/compute/options")?;
    let project = order_project(client, args.project.as_deref())?;
    let zone_ids: Vec<String> = free_options["zone_ids"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|z| z.as_str().map(str::to_string))
        .collect();
    let zone = order_zone(client, args.zone.as_deref(), Some(&zone_ids))?;

    // Size: --plan, then flags, then the image's minimum / the free plan's size.
    let plan = match &args.plan {
        Some(name) => Some(pick_plan(&compute_options["container_packages"], name)?),
        None => None,
    };
    let from_plan = |key: &str| plan.as_ref().and_then(|p| p[key].as_u64()).map(|v| v as u32);
    let min_ram = known.map_or(1, |i| i.min_ram_gb);
    let cpu = args.cpu.or(from_plan("cpu_cores")).unwrap_or(1);
    let ram = args.ram.or(from_plan("ram_gb")).unwrap_or(min_ram);
    let storage = args.storage.or(from_plan("storage_gb")).unwrap_or(5);
    if ram < min_ram {
        return Err(CliError::usage(format!("{} needs at least {min_ram} GB of RAM", args.image)));
    }

    let ports = if args.ports.is_empty() {
        known.map(|i| i.ports.iter().map(|(p, proto)| (*p, proto.to_string())).collect()).unwrap_or_default()
    } else {
        args.ports.iter().map(|p| parse_port(p)).collect::<Result<Vec<_>, _>>()?
    };
    let env = environment(known, &args.env, client)?;

    let mut body = json!({
        "project_id": id_of(&project)?,
        "name": args.name,
        "image": image,
        "cpu_cores": cpu,
        "ram_gb": ram,
        "storage_gb": storage,
        "ports": ports.iter().map(|(p, proto)| json!({ "port": p, "protocol": proto })).collect::<Vec<_>>(),
        "env": env,
        "availability_zone_id": id_of(&zone)?,
        "period_months": args.period,
    });
    if let Some(c) = &args.command {
        body["command"] = json!(c);
    }

    // The DNS name: on the HTTP port (HTTPS), else straight to the first TCP / UDP port.
    let mut dns_name = Value::Null;
    if !args.no_dns {
        let name = args.dns_name.clone().unwrap_or_else(|| args.name.clone());
        if let Some((port, _)) = ports.iter().find(|(_, proto)| proto == "http") {
            body["dns_name"] = json!(name);
            body["dns_kind"] = json!("web");
            body["dns_port"] = json!(port);
            body["dns_protocol"] = json!("http");
        } else if let Some((port, proto)) = ports.first() {
            body["dns_name"] = json!(name);
            body["dns_kind"] = json!("direct");
            body["dns_port"] = json!(port);
            body["dns_protocol"] = json!(proto);
            if proto == "tcp" && args.minecraft_srv.unwrap_or(*port == 25565) {
                body["dns_srv"] = json!("minecraft");
            }
        }
        if body.get("dns_name").is_some() {
            dns_name = json!(format!("{name}.in.mhon.cloud"));
        }
    }

    let mut login = Value::Null;
    if is_dev {
        let username = args.username.clone().unwrap_or_else(|| login_name(my_username(client).ok().flatten()));
        let (password, generated) =
            if args.password_stdin { (read_stdin_line("password")?, false) } else { (random_password()?, true) };
        body["login_username"] = json!(username);
        body["login_password"] = json!(password);
        login = if generated {
            json!({ "username": username, "password": password })
        } else {
            json!({ "username": username })
        };
        if let Some(path) = &args.startup_script {
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            if size > STARTUP_SCRIPT_MAX {
                return Err(CliError::usage("a startup script can be at most 16 KB"));
            }
            let script = std::fs::read_to_string(path)
                .map_err(|e| CliError::usage(format!("can't read {}: {e}", path.display())))?;
            body["startup_script"] = json!(script);
        }
    } else if args.username.is_some() || args.password_stdin || args.startup_script.is_some() {
        return Err(CliError::usage("--username, --password-stdin and --startup-script are for --image ubuntu only"));
    }

    let uploads: Vec<(String, String)> =
        args.uploads.iter().map(|u| upload::parse_upload(u)).collect::<Result<_, _>>()?;
    if uploads.len() > 3 {
        return Err(CliError::usage("at most 3 folders can be uploaded"));
    }

    // Free within the free plan (1 month), else prepaid: --yes, with an estimate.
    let free = &free_options["free"];
    let is_free = free_options["free_available"] == true
        && u64::from(cpu) <= free["cpu_cores"].as_u64().unwrap_or(0)
        && u64::from(ram) <= free["ram_gb"].as_u64().unwrap_or(0)
        && u64::from(storage) <= free["storage_gb"].as_u64().unwrap_or(0);
    if is_free {
        body["period_months"] = json!(1);
    } else {
        let estimate = estimate(&compute_options, plan.as_ref(), cpu, ram, storage, args.period);
        need_yes(
            args.confirm.yes,
            &format!(
                "this is beyond the free plan and pays about ฿{estimate:.2} for {} month(s) from your credit (฿{} left)",
                args.period,
                compute_options["credit"].as_str().unwrap_or("?")
            ),
        )?;
    }

    let mut uploaded = Vec::new();
    for (dir, mount) in &uploads {
        let u = upload::upload_folder(client, std::path::Path::new(dir))?;
        uploaded.push(upload::describe(&u, mount));
    }
    if !uploaded.is_empty() {
        body["files"] = json!(uploaded
            .iter()
            .map(|u| json!({ "upload_id": u["upload_id"], "mount_path": u["mount_path"] }))
            .collect::<Vec<_>>());
    }

    let answer = client.post("/api/v1/market/containers", Some(body))?;
    let mut container = answer.get("container").cloned().unwrap_or(answer);
    if args.wait.wait {
        container = wait_until_status(client, id_of(&container)?, "running", args.wait.timeout)?;
    }
    let result = json!({ "container": container, "dns_name": dns_name, "login": login, "uploads": uploaded });
    if ctx.format == Format::Table && args.wait.wait {
        print_object(ctx.format, &result["container"], DETAIL);
        if !login.is_null() {
            print_object(ctx.format, &result, &RESULT[8..10]); // Login user, Password
        }
        return Ok(());
    }
    print_object(ctx.format, &result, RESULT);
    Ok(())
}

/// The image's default variables, the account's username as Minecraft's operator, then the
/// order's own; the image's required ones must be set and no `YOUR_...` placeholder may stay.
fn environment(known: Option<&Image>, given: &[String], client: &Client) -> Result<BTreeMap<String, String>, CliError> {
    let mut env: BTreeMap<String, String> = BTreeMap::new();
    if let Some(i) = known {
        for (k, v) in i.env {
            env.insert(k.to_string(), v.to_string());
        }
        if i.alias == "minecraft" && !given.iter().any(|g| g.starts_with("OPS=")) {
            if let Some(name) = my_username(client)? {
                env.insert("OPS".into(), name);
            }
        }
    }
    for pair in given {
        let (k, v) =
            pair.split_once('=').ok_or_else(|| CliError::usage(format!("--env is KEY=VALUE (got \"{pair}\")")))?;
        if k.starts_with("MHON_") {
            return Err(CliError::usage("MHON_ variables are the platform's own"));
        }
        env.insert(k.to_string(), v.to_string());
    }
    if let Some(i) = known {
        let missing: Vec<&str> =
            i.required.iter().copied().filter(|k| env.get(*k).is_none_or(|v| v.is_empty())).collect();
        if !missing.is_empty() {
            return Err(CliError::usage(format!("{} needs: {} (add --env KEY=VALUE)", i.alias, missing.join(", "))));
        }
    }
    if let Some((k, _)) = env.iter().find(|(_, v)| v.starts_with("YOUR_")) {
        return Err(CliError::usage(format!("set {k} (add --env {k}=...)")));
    }
    Ok(env)
}

/// `80/http` -> (80, "http"); a bare number is TCP.
fn parse_port(text: &str) -> Result<(u32, String), CliError> {
    let bad = || CliError::usage(format!("a port is PORT/PROTOCOL: 80/http, 22/tcp, 19132/udp (got \"{text}\")"));
    let (port, proto) = text.split_once('/').unwrap_or((text, "tcp"));
    let port: u32 = port.parse().map_err(|_| bad())?;
    let proto = proto.to_ascii_lowercase();
    if !(1..=65535).contains(&port) || !["http", "tcp", "udp"].contains(&proto.as_str()) {
        return Err(bad());
    }
    Ok((port, proto))
}

/// The console's estimate: rates x size x months, less the plan's discount when the size is
/// exactly the plan, then the period discount.
fn estimate(options: &Value, plan: Option<&Value>, cpu: u32, ram: u32, storage: u32, months: u32) -> f64 {
    let rate = |k: &str| options["container_rates"][k].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let pct = |v: &Value| v.as_str().and_then(|s| s.parse::<f64>().ok()).or(v.as_f64()).unwrap_or(0.0);
    let monthly = rate("cpu") * f64::from(cpu) + rate("ram") * f64::from(ram) + rate("storage") * f64::from(storage);
    let mut total = monthly * f64::from(months);
    if let Some(p) = plan {
        let exact = p["cpu_cores"] == json!(cpu) && p["ram_gb"] == json!(ram) && p["storage_gb"] == json!(storage);
        if exact {
            total *= 1.0 - pct(&p["discount"]) / 100.0;
        }
    }
    let period = match months {
        6 => rate("discount_6_months"),
        12 => rate("discount_year"),
        _ => 0.0,
    };
    total * (1.0 - period / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports_parse() {
        assert_eq!(parse_port("80/HTTP").unwrap(), (80, "http".to_string()));
        assert_eq!(parse_port("22").unwrap(), (22, "tcp".to_string()));
        assert!(parse_port("0/tcp").is_err());
        assert!(parse_port("80/sctp").is_err());
    }

    #[test]
    fn estimates_like_the_console() {
        let options = json!({ "container_rates": { "cpu": "100.00", "ram": "50.00", "storage": "2.00", "discount_6_months": "5.00", "discount_year": "10.00" } });
        // 2 cpu + 2 GB + 10 GB = 320 / month
        assert!((estimate(&options, None, 2, 2, 10, 1) - 320.0).abs() < 1e-9);
        assert!((estimate(&options, None, 2, 2, 10, 12) - 320.0 * 12.0 * 0.9).abs() < 1e-9);
        let plan = json!({ "cpu_cores": 2, "ram_gb": 2, "storage_gb": 10, "discount": "20.00" });
        assert!((estimate(&options, Some(&plan), 2, 2, 10, 1) - 256.0).abs() < 1e-9);
        assert!((estimate(&options, Some(&plan), 2, 4, 10, 1) - 420.0).abs() < 1e-9);
    }
}
