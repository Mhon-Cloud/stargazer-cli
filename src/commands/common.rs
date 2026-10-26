//! Helpers shared by the commands: reading lists out of API answers and finding one item by
//! id or name (agents and people both prefer names; the API wants ids).

use std::time::{Duration, Instant};

use serde_json::Value;

use crate::api::Client;
use crate::error::{CliError, ErrorKind};

/// The array under `key` in an API answer (`{"servers": [...]}`), or empty.
pub fn items(answer: Value, key: &str) -> Vec<Value> {
    match answer {
        Value::Object(mut map) => match map.remove(key) {
            Some(Value::Array(items)) => items,
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Whether `item` is in the project named or numbered `filter` (no filter = every project).
pub fn in_project(item: &Value, filter: Option<&str>) -> bool {
    let Some(filter) = filter else { return true };
    let project = &item["project"];
    project["id"].as_str() == Some(filter)
        || project["name"].as_str().is_some_and(|name| name.eq_ignore_ascii_case(filter))
}

/// The one item whose `id` (or one of `keys`, e.g. `name`, `number`) is `needle`, within the
/// project filter. Several items with that name → a usage error asking for the id.
pub fn find<'a>(
    list: &'a [Value],
    needle: &str,
    project: Option<&str>,
    keys: &[&str],
    noun: &str,
) -> Result<&'a Value, CliError> {
    let scoped: Vec<&Value> = list.iter().filter(|item| in_project(item, project)).collect();
    if let Some(item) = scoped.iter().find(|item| item["id"].as_str() == Some(needle)) {
        return Ok(item);
    }
    let matches: Vec<&&Value> = scoped
        .iter()
        .filter(|item| keys.iter().any(|k| item[*k].as_str().is_some_and(|v| v.eq_ignore_ascii_case(needle))))
        .collect();
    match matches.as_slice() {
        [one] => Ok(one),
        [] => Err(CliError::not_found(format!("no {noun} \"{needle}\" in your projects"))),
        _ => Err(CliError::usage(format!(
            "{} {noun}s are called \"{needle}\"; use the id, or narrow it with --project",
            matches.len()
        ))),
    }
}

/// The `id` of an item found with [`find`].
pub fn id_of(item: &Value) -> Result<&str, CliError> {
    item["id"].as_str().ok_or_else(|| CliError::new(ErrorKind::Internal, "the API sent an item without an id"))
}

/// A kind of thing the API lists, and how people name one of it. Every command that takes
/// "an id or a name" goes through [`Kind::resolve`]; a new resource (e.g. VPCs) is one more const.
pub struct Kind {
    /// For messages: "no server \"web\" in your projects".
    pub noun: &'static str,
    /// The list operation.
    pub path: &'static str,
    /// Where the list is in its answer.
    pub key: &'static str,
    /// Fields that also name an item, besides its id.
    pub names: &'static [&'static str],
}

pub const PROJECTS: Kind = Kind { noun: "project", path: "/api/v1/projects", key: "projects", names: &["name"] };
pub const ZONES: Kind =
    Kind { noun: "location", path: "/api/v1/availability-zones", key: "availability_zones", names: &["name"] };
pub const SERVERS: Kind = Kind { noun: "server", path: "/api/v1/servers", key: "servers", names: &["name"] };
pub const CONTAINERS: Kind =
    Kind { noun: "container", path: "/api/v1/containers", key: "containers", names: &["name", "number"] };
pub const SSH_KEYS: Kind =
    Kind { noun: "SSH key", path: "/api/v1/credentials", key: "credentials", names: &["name", "fingerprint"] };
pub const SECURITY_GROUPS: Kind =
    Kind { noun: "security group", path: "/api/v1/security-groups", key: "security_groups", names: &["name"] };
pub const PUBLIC_IPS: Kind =
    Kind { noun: "public IP", path: "/api/v1/public-ips", key: "public_ips", names: &["address"] };
pub const DNS_NAMES: Kind = Kind { noun: "DNS name", path: "/api/v1/dns", key: "records", names: &["name", "fqdn"] };
pub const ORDERS: Kind =
    Kind { noun: "order", path: "/api/v1/market/compute/orders", key: "orders", names: &["number", "name"] };

impl Kind {
    pub fn list(&self, client: &Client) -> Result<Vec<Value>, CliError> {
        Ok(items(client.get(self.path)?, self.key))
    }

    /// The one item named `needle` (id or a name), within the project filter.
    pub fn resolve(&self, client: &Client, needle: &str, project: Option<&str>) -> Result<Value, CliError> {
        let list = self.list(client)?;
        Ok(find(&list, needle, project, self.names, self.noun)?.clone())
    }

    pub fn resolve_id(&self, client: &Client, needle: &str, project: Option<&str>) -> Result<String, CliError> {
        Ok(id_of(&self.resolve(client, needle, project)?)?.to_string())
    }
}

/// The project an order goes in: `--project`, else the caller's personal Default project, else
/// their only project.
pub fn order_project(client: &Client, given: Option<&str>) -> Result<Value, CliError> {
    if let Some(p) = given {
        return PROJECTS.resolve(client, p, None);
    }
    let projects = PROJECTS.list(client)?;
    let default = projects.iter().find(|p| p["is_default"] == true && p["kind"] == "personal");
    match (default, projects.as_slice()) {
        (Some(p), _) => Ok(p.clone()),
        (None, [only]) => Ok(only.clone()),
        _ => Err(CliError::usage("say which project with --project (see `stargazer projects list`)")),
    }
}

/// The location an order goes to: `--zone`, else the only one of `allowed` (all zones when None).
pub fn order_zone(client: &Client, given: Option<&str>, allowed: Option<&[String]>) -> Result<Value, CliError> {
    let zones: Vec<Value> = ZONES
        .list(client)?
        .into_iter()
        .filter(|z| allowed.is_none_or(|ids| z["id"].as_str().is_some_and(|id| ids.iter().any(|a| a == id))))
        .collect();
    if let Some(z) = given {
        return Ok(find(&zones, z, None, ZONES.names, "location that offers this")?.clone());
    }
    match zones.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(CliError::new(ErrorKind::Conflict, "no location offers this right now")),
        _ => {
            let names: Vec<&str> = zones.iter().filter_map(|z| z["name"].as_str()).collect();
            Err(CliError::usage(format!("say which location with --zone: {}", names.join(", "))))
        }
    }
}

/// `--wait`: calls `check` every few seconds until it returns the finished item; a failure it
/// reports ends the wait with that error. Exit 9 (timeout) after `timeout` seconds.
pub fn wait_for(
    timeout: u64,
    what: &str,
    mut check: impl FnMut() -> Result<Option<Value>, CliError>,
) -> Result<Value, CliError> {
    let deadline = Instant::now() + Duration::from_secs(timeout);
    loop {
        if let Some(done) = check()? {
            return Ok(done);
        }
        if Instant::now() >= deadline {
            return Err(CliError::new(
                ErrorKind::Timeout,
                format!("gave up waiting for {what} after {timeout}s (it may still finish; check again later)"),
            ));
        }
        std::thread::sleep(POLL);
    }
}

const POLL: Duration = Duration::from_secs(4);

/// A random password like the console's: 16 characters, no look-alikes (0/O, 1/l/I).
pub fn random_password() -> Result<String, CliError> {
    const CHARS: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| CliError::new(ErrorKind::Internal, format!("no randomness for a password: {e}")))?;
    Ok(bytes.iter().map(|b| CHARS[*b as usize % CHARS.len()] as char).collect())
}

/// One line from stdin (a password or key piped in), without its newline.
pub fn read_stdin_line(what: &str) -> Result<String, CliError> {
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| CliError::usage(format!("can't read the {what} from stdin: {e}")))?;
    let line = line.trim_end_matches(['\r', '\n']).to_string();
    if line.is_empty() {
        return Err(CliError::usage(format!("no {what} on stdin")));
    }
    Ok(line)
}

/// `--yes` missing: a usage error saying what would happen.
pub fn need_yes(yes: bool, what: &str) -> Result<(), CliError> {
    if yes {
        Ok(())
    } else {
        Err(CliError::usage(format!("{what}; add --yes to confirm")))
    }
}

/// The caller's username, a starting point for login users (as the console does).
pub fn my_username(client: &Client) -> Result<Option<String>, CliError> {
    let me = client.get("/api/v1/auth/me")?;
    Ok(me["user"]["username"].as_str().map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn list() -> Vec<Value> {
        vec![
            json!({ "id": "a1", "name": "web", "project": { "id": "p1", "name": "Shop" } }),
            json!({ "id": "b2", "name": "web", "project": { "id": "p2", "name": "Blog" } }),
            json!({ "id": "c3", "name": "db", "project": { "id": "p1", "name": "Shop" } }),
        ]
    }

    #[test]
    fn finds_by_id_or_unique_name() {
        let l = list();
        assert_eq!(find(&l, "b2", None, &["name"], "server").unwrap()["id"], "b2");
        assert_eq!(find(&l, "DB", None, &["name"], "server").unwrap()["id"], "c3");
    }

    #[test]
    fn ambiguous_names_need_a_project() {
        let l = list();
        assert!(find(&l, "web", None, &["name"], "server").is_err());
        assert_eq!(find(&l, "web", Some("blog"), &["name"], "server").unwrap()["id"], "b2");
    }

    #[test]
    fn missing_is_not_found() {
        let l = list();
        let err = find(&l, "nope", None, &["name"], "server").unwrap_err();
        assert_eq!(err.kind, crate::error::ErrorKind::NotFound);
    }
}
