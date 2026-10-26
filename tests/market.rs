//! Ordering against a mock API: what spends credit needs --yes, the bodies the CLI builds,
//! uploads, DNS defaults, --fields and --wait.

mod common;

use mockito::Matcher;
use serde_json::{json, Value};

use common::Env;

const PROJECTS: &str = r#"{"projects": [
  {"id": "p1", "name": "Default", "kind": "personal", "is_default": true, "my_role": "owner"},
  {"id": "p2", "name": "Shop", "kind": "personal", "is_default": false, "my_role": "owner"}
]}"#;
const ZONES: &str = r#"{"availability_zones": [{"id": "z1", "name": "BKK-1", "region": "th"}]}"#;
const ME: &str = r#"{"user": {"id": "u1", "email": "a@b.c", "username": "Nathadon"}}"#;
const COMPUTE_OPTIONS: &str = r#"{
  "os": [{"id": "ubuntu24.04", "name": "ubuntu24.04"}],
  "packages": [{"name": "Starter", "cpu_cores": 2, "ram_gb": 4, "storage_gb": 40, "storage_type": "ssd", "public_ip": true, "discount": "10.00"}],
  "container_rates": {"cpu": "100.00", "ram": "50.00", "storage": "2.00", "discount_6_months": "5.00", "discount_year": "10.00"},
  "container_packages": [],
  "limits": {"min_cores": 1, "max_cores": 32, "min_ram_gb": 1, "max_ram_gb": 128, "max_disks": 4, "min_disk_gb": 10, "max_disk_gb": 4096},
  "credit": "1000.00"
}"#;
const CONTAINER_OPTIONS: &str = r#"{"free_available": true, "free_used": 0, "free_limit": 5,
  "free": {"cpu_cores": 1, "ram_gb": 1, "storage_gb": 5}, "zone_ids": ["z1"]}"#;
const QUOTE: &str = r#"{"monthly": "450.00", "period_months": 1, "subtotal": "450.00", "total": "405.00",
  "credit": "1000.00", "enough_credit": true, "coupon_error": null}"#;

fn stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("stdout is JSON: {}", String::from_utf8_lossy(&output.stdout)))
}

fn stderr_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stderr).expect("stderr is JSON")
}

/// The reads every order makes.
fn order_mocks(api: &mut mockito::ServerGuard) {
    api.mock("GET", "/api/v1/projects").with_body(PROJECTS).create();
    api.mock("GET", "/api/v1/availability-zones").with_body(ZONES).create();
    api.mock("GET", "/api/v1/auth/me").with_body(ME).create();
    api.mock("GET", "/api/v1/market/compute/options").with_body(COMPUTE_OPTIONS).create();
    api.mock("GET", "/api/v1/market/containers/options").with_body(CONTAINER_OPTIONS).create();
}

#[test]
fn server_orders_need_yes_and_never_pay_without_it() {
    let mut api = mockito::Server::new();
    order_mocks(&mut api);
    api.mock("POST", "/api/v1/market/compute/quote").with_body(QUOTE).create();
    let order = api.mock("POST", "/api/v1/market/compute/orders").expect(0).create();

    let out = Env::new()
        .signed_in(&api.url())
        .args(["servers", "create", "web-1", "--os", "ubuntu24.04", "--plan", "starter"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err = stderr_json(&out);
    assert!(err["error"]["message"].as_str().unwrap().contains("405.00"), "{err}");
    order.assert();
}

#[test]
fn dry_run_prints_the_quote() {
    let mut api = mockito::Server::new();
    order_mocks(&mut api);
    api.mock("POST", "/api/v1/market/compute/quote").with_body(QUOTE).create();
    let out = Env::new()
        .signed_in(&api.url())
        .args(["servers", "create", "web-1", "--os", "ubuntu24.04", "--cpu", "2", "--ram", "4", "--dry-run"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(stdout_json(&out)["total"], "405.00");
}

#[test]
fn server_orders_send_the_plan_and_print_the_password_once() {
    let mut api = mockito::Server::new();
    order_mocks(&mut api);
    api.mock("POST", "/api/v1/market/compute/quote").with_body(QUOTE).create();
    let order = api
        .mock("POST", "/api/v1/market/compute/orders")
        .match_body(Matcher::PartialJson(json!({
            "project_id": "p1",
            "name": "web-1",
            "cpu_cores": 2,
            "ram_gb": 4,
            "disks": [{ "type": "ssd", "size_gb": 40 }],
            "os": "ubuntu24.04",
            "availability_zone_id": "z1",
            "public_ip": true,
            "period_months": 12,
            "username": "nathadon",
            "auto_backup": true,
            "backup_day": "sun",
            "backup_time": "02:30",
        })))
        .with_status(201)
        .with_body(r#"{"order": {"id": "o1", "number": "ORD-000001", "status": "pending", "name": "web-1"}}"#)
        .create();

    let out = Env::new()
        .signed_in(&api.url())
        .args([
            "servers",
            "create",
            "web-1",
            "--os",
            "ubuntu24.04",
            "--plan",
            "Starter",
            "--period",
            "12",
            "--backup",
            "sun@02:30",
            "--yes",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    order.assert();
    let result = stdout_json(&out);
    assert_eq!(result["order"]["number"], "ORD-000001");
    assert_eq!(result["login"]["password"].as_str().map(str::len), Some(16));
}

#[test]
fn not_enough_credit_stops_before_ordering() {
    let mut api = mockito::Server::new();
    order_mocks(&mut api);
    api.mock("POST", "/api/v1/market/compute/quote")
        .with_body(r#"{"total": "405.00", "credit": "10.00", "enough_credit": false, "coupon_error": null}"#)
        .create();
    let out = Env::new()
        .signed_in(&api.url())
        .args(["servers", "create", "web-1", "--os", "ubuntu24.04", "--plan", "starter", "--yes"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
}

#[test]
fn a_free_nginx_container_needs_no_yes_and_gets_a_web_name() {
    let mut api = mockito::Server::new();
    order_mocks(&mut api);
    let order = api
        .mock("POST", "/api/v1/market/containers")
        .match_body(Matcher::PartialJson(json!({
            "project_id": "p1",
            "name": "site",
            "image": "nginx:1.27-alpine",
            "cpu_cores": 1,
            "ram_gb": 1,
            "storage_gb": 5,
            "ports": [{ "port": 80, "protocol": "http" }],
            "dns_name": "site",
            "dns_kind": "web",
            "dns_port": 80,
            "period_months": 1,
        })))
        .with_status(201)
        .with_body(
            r#"{"container": {"id": "c1", "number": "CT-000001", "name": "site", "status": "pending", "free": true}}"#,
        )
        .create();
    let out =
        Env::new().signed_in(&api.url()).args(["containers", "create", "site", "--image", "nginx"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    order.assert();
    assert_eq!(stdout_json(&out)["dns_name"], "site.in.mhon.cloud");
}

#[test]
fn a_paid_container_needs_yes() {
    let mut api = mockito::Server::new();
    order_mocks(&mut api);
    let order = api.mock("POST", "/api/v1/market/containers").expect(0).create();
    let out = Env::new()
        .signed_in(&api.url())
        .args(["containers", "create", "big", "--image", "nginx", "--cpu", "2", "--ram", "2", "--storage", "10"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    // 2 x 100 + 2 x 50 + 10 x 2 = 320 a month
    assert!(stderr_json(&out)["error"]["message"].as_str().unwrap().contains("320.00"));
    order.assert();
}

#[test]
fn required_variables_are_asked_for() {
    let mut api = mockito::Server::new();
    order_mocks(&mut api);
    let out =
        Env::new().signed_in(&api.url()).args(["containers", "create", "db", "--image", "postgres"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr_json(&out)["error"]["message"].as_str().unwrap().contains("POSTGRES_PASSWORD"));
}

#[test]
fn minecraft_gets_a_direct_name_with_srv_and_the_username_as_operator() {
    let mut api = mockito::Server::new();
    order_mocks(&mut api);
    let order = api
        .mock("POST", "/api/v1/market/containers")
        .match_body(Matcher::PartialJson(json!({
            "image": "itzg/minecraft-server:latest",
            "ram_gb": 2,
            "dns_kind": "direct",
            "dns_port": 25565,
            "dns_protocol": "tcp",
            "dns_srv": "minecraft",
            "env": { "EULA": "TRUE", "OPS": "Nathadon", "TYPE": "FORGE" },
        })))
        .with_status(201)
        .with_body(r#"{"container": {"id": "c2", "name": "mc", "status": "pending"}}"#)
        .create();
    let out = Env::new()
        .signed_in(&api.url())
        .args(["containers", "create", "mc", "--image", "minecraft", "--env", "TYPE=FORGE", "--yes"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    order.assert();
}

#[test]
fn uploads_go_in_chunks_then_the_order_names_them() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "<h1>hi</h1>").unwrap();
    let upload_arg = format!("{}:/usr/share/nginx/html", dir.path().display());

    let mut api = mockito::Server::new();
    order_mocks(&mut api);
    let start = api
        .mock("POST", "/api/v1/market/containers/uploads")
        .with_status(201)
        .with_body(r#"{"id": "up1", "chunk_bytes": 1024}"#)
        .create();
    // One small file packs to a few KiB, so 1 KiB chunks make several PUTs.
    let chunks = api
        .mock("PUT", "/api/v1/market/containers/uploads/up1")
        .match_query(Matcher::Regex("offset=\\d+".into()))
        .match_header("content-type", "application/octet-stream")
        .with_body_from_request(|req| {
            let offset: u64 = req.path_and_query().split("offset=").nth(1).and_then(|o| o.parse().ok()).unwrap_or(0);
            let len = req.body().map(|b| b.len() as u64).unwrap_or(0);
            json!({ "received_bytes": offset + len, "size_bytes": 0 }).to_string().into()
        })
        .expect_at_least(2)
        .create();
    let complete = api
        .mock("POST", "/api/v1/market/containers/uploads/up1/complete")
        .with_body(r#"{"id": "up1", "files": 1, "size_bytes": 2048}"#)
        .create();
    let order = api
        .mock("POST", "/api/v1/market/containers")
        .match_body(Matcher::PartialJson(
            json!({ "files": [{ "upload_id": "up1", "mount_path": "/usr/share/nginx/html" }] }),
        ))
        .with_status(201)
        .with_body(r#"{"container": {"id": "c3", "name": "site", "status": "pending"}}"#)
        .create();

    let out = Env::new()
        .signed_in(&api.url())
        .args(["containers", "create", "site", "--image", "nginx", "--upload", &upload_arg])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    start.assert();
    chunks.assert();
    complete.assert();
    order.assert();
    assert_eq!(stdout_json(&out)["uploads"][0]["files"], 1);
}

#[test]
fn dns_create_points_at_the_http_port() {
    let mut api = mockito::Server::new();
    api.mock("GET", "/api/v1/containers")
        .with_body(r#"{"containers": [{"id": "c1", "name": "site", "number": "CT-000001", "project": {"id": "p1", "name": "Default"},
            "ports": [{"port": 22, "protocol": "tcp"}, {"port": 8080, "protocol": "http"}]}]}"#)
        .create();
    let create = api
        .mock("POST", "/api/v1/dns")
        .match_body(Matcher::Json(json!({
            "project_id": "p1", "name": "blog", "target_kind": "container", "container_id": "c1",
            "port": 8080, "kind": "web", "protocol": "http", "srv": "",
        })))
        .with_status(201)
        .with_body(r#"{"record": {"id": "d1", "fqdn": "blog.in.mhon.cloud", "status": "waiting"}}"#)
        .create();
    let out =
        Env::new().signed_in(&api.url()).args(["dns", "create", "blog", "--container", "CT-000001"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    create.assert();
}

#[test]
fn reserving_an_ip_needs_yes() {
    let mut api = mockito::Server::new();
    api.mock("GET", "/api/v1/projects").with_body(PROJECTS).create();
    api.mock("GET", "/api/v1/market/public-ip/quote")
        .with_body(
            r#"{"total": "250.00", "enough_credit": true, "zones": [{"id": "z1", "name": "BKK-1", "available": 3}]}"#,
        )
        .create();
    let reserve = api
        .mock("POST", "/api/v1/market/public-ip")
        .match_body(Matcher::Json(json!({ "project_id": "p1", "availability_zone_id": "z1" })))
        .with_status(201)
        .with_body(r#"{"address": "203.0.113.7", "reserved_until": "2026-11-26T00:00:00Z"}"#)
        .expect(1)
        .create();
    let env = Env::new();
    let no = env.signed_in(&api.url()).args(["public-ips", "reserve"]).output().unwrap();
    assert_eq!(no.status.code(), Some(2));
    let yes = env.signed_in(&api.url()).args(["public-ips", "reserve", "--yes"]).output().unwrap();
    assert!(yes.status.success(), "{}", String::from_utf8_lossy(&yes.stderr));
    reserve.assert();
    assert_eq!(stdout_json(&yes)["address"], "203.0.113.7");
}

#[test]
fn fields_trim_the_json() {
    let mut api = mockito::Server::new();
    api.mock("GET", "/api/v1/projects").with_body(PROJECTS).create();
    let out = Env::new().signed_in(&api.url()).args(["projects", "list", "--fields", "name,kind"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(stdout_json(&out)[1], json!({ "name": "Shop", "kind": "personal" }));
}

#[test]
fn wait_follows_a_container_until_it_runs() {
    let mut api = mockito::Server::new();
    let list = r#"{"containers": [{"id": "c1", "name": "site", "status": "running", "project": {"id": "p1", "name": "Default"}}]}"#;
    api.mock("GET", "/api/v1/containers").with_body(list).create();
    api.mock("POST", "/api/v1/containers/c1/start")
        .with_body(r#"{"container": {"id": "c1", "status": "stopped", "desired_state": "running"}}"#)
        .create();
    let out = Env::new().signed_in(&api.url()).args(["containers", "start", "site", "--wait"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(stdout_json(&out)["status"], "running");
}

#[test]
fn pricing_needs_no_key() {
    let mut api = mockito::Server::new();
    let pricing = api
        .mock("GET", "/api/v1/public/pricing")
        .match_header("authorization", Matcher::Missing)
        .with_body(r#"{"compute": {"cpu": "100.00"}}"#)
        .create();
    let out = Env::new().cmd().env("STARGAZER_API_URL", api.url()).arg("pricing").output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    pricing.assert();
}

#[test]
fn deleting_a_project_needs_its_name_and_yes() {
    let mut api = mockito::Server::new();
    api.mock("GET", "/api/v1/projects").with_body(PROJECTS).create();
    let delete = api.mock("DELETE", "/api/v1/projects/p2").expect(0).create();
    let out = Env::new()
        .signed_in(&api.url())
        .args(["projects", "delete", "shop", "--confirm", "shop", "--yes"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "the name is case-sensitive, like the console");
    delete.assert();
}
