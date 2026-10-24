//! Helpers shared by the commands: reading lists out of API answers and finding one item by
//! id or name (agents and people both prefer names; the API wants ids).

use serde_json::Value;

use crate::error::CliError;

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
    item["id"]
        .as_str()
        .ok_or_else(|| CliError::new(crate::error::ErrorKind::Internal, "the API sent an item without an id"))
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
