//! openapi/contract.json is what the CLI needs from the API; the platform checks new specs against it before
//! deploying. These tests keep it honest: every API call in src/ is listed, and everything listed exists in
//! openapi/openapi.json (so refreshing the spec copy fails here when the platform removed something we use).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

fn load(path: &str) -> Value {
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).expect(path);
    serde_json::from_str(&text).expect(path)
}

/// `/api/v1/servers/{id}/actions` and `/api/v1/servers/{}/actions` are the same route.
fn normalize(path: &str) -> String {
    let mut out = String::new();
    let mut in_param = false;
    for c in path.chars() {
        match c {
            '{' => {
                in_param = true;
                out.push_str("{}");
            }
            '}' => in_param = false,
            _ if !in_param => out.push(c),
            _ => {}
        }
    }
    out
}

fn contract_ops() -> Vec<Value> {
    load("openapi/contract.json")["operations"].as_array().expect("operations").clone()
}

/// (METHOD, path) of every API call in src/: a `"/api/v1...` literal after `.get(`, `.get_query(`, `.post(`,
/// `.put(` or `.delete(` on the same line.
fn calls_in_code() -> BTreeSet<(String, String)> {
    let mut calls = BTreeSet::new();
    let mut dirs = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).expect("src") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            for line in fs::read_to_string(&path).expect("source").lines() {
                let Some(start) = line.find("\"/api/v1") else { continue };
                let before = &line[..start];
                let method = [
                    (".get_query(", "GET"),
                    (".get(", "GET"),
                    (".post(", "POST"),
                    (".put(", "PUT"),
                    (".delete(", "DELETE"),
                ]
                .iter()
                .filter_map(|(call, m)| before.rfind(call).map(|at| (at, *m)))
                .max_by_key(|(at, _)| *at)
                .map(|(_, m)| m);
                let Some(method) = method else { continue }; // a doc comment or a message, not a call
                let literal = &line[start + 1..];
                let end = literal.find('"').expect("closing quote");
                calls.insert((method.to_string(), normalize(&literal[..end])));
            }
        }
    }
    calls
}

#[test]
fn every_api_call_is_in_the_contract() {
    let listed: BTreeSet<(String, String)> = contract_ops()
        .iter()
        .map(|op| (op["method"].as_str().unwrap().to_string(), normalize(op["path"].as_str().unwrap())))
        .collect();
    let used = calls_in_code();
    assert!(used.len() >= 10, "found only {} calls in src/: is the scan broken?", used.len());
    let missing: Vec<_> = used.difference(&listed).collect();
    let unused: Vec<_> = listed.difference(&used).collect();
    assert!(missing.is_empty(), "called in src/ but not in openapi/contract.json: {missing:?}");
    assert!(unused.is_empty(), "in openapi/contract.json but no longer called: {unused:?}");
}

fn resolve<'a>(spec: &'a Value, schema: &'a Value) -> &'a Value {
    match schema.get("$ref").and_then(Value::as_str) {
        Some(r) => {
            let name = r.rsplit('/').next().unwrap();
            resolve(spec, &spec["components"]["schemas"][name])
        }
        None => schema,
    }
}

/// Every variant a value may take: the schema itself, or each of its oneOf / anyOf / allOf.
fn variants<'a>(spec: &'a Value, schema: &'a Value) -> Vec<&'a Value> {
    let schema = resolve(spec, schema);
    for key in ["oneOf", "anyOf", "allOf"] {
        if let Some(list) = schema.get(key).and_then(Value::as_array) {
            return list.iter().flat_map(|s| variants(spec, s)).collect();
        }
    }
    vec![schema]
}

/// Whether `path` (`servers[].project.name`) exists in `schema`.
fn has_field(spec: &Value, schema: &Value, path: &str) -> bool {
    let (head, rest) = match path.split_once('.') {
        Some((h, r)) => (h, Some(r)),
        None => (path, None),
    };
    let (name, each) = match head.strip_suffix("[]") {
        Some(n) => (n, true),
        None => (head, false),
    };
    variants(spec, schema).into_iter().any(|s| {
        let Some(field) = s["properties"].get(name) else { return false };
        let field = if each { resolve(spec, field).get("items").unwrap_or(&Value::Null) } else { field };
        match rest {
            Some(rest) => has_field(spec, field, rest),
            None => !field.is_null(),
        }
    })
}

fn success_schema(op: &Value) -> Option<&Value> {
    let responses = op["responses"].as_object()?;
    responses.iter().find(|(code, _)| code.starts_with('2')).map(|(_, r)| &r["content"]["application/json"]["schema"])
}

#[test]
fn the_spec_has_everything_the_contract_needs() {
    let spec = load("openapi/openapi.json");
    let mut problems = Vec::new();
    for op in contract_ops() {
        let method = op["method"].as_str().unwrap();
        let path = op["path"].as_str().unwrap();
        let found = spec["paths"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(p, _)| normalize(p) == normalize(path))
            .and_then(|(_, item)| item.get(method.to_lowercase()));
        let Some(spec_op) = found else {
            problems.push(format!("{method} {path}: not in the spec"));
            continue;
        };
        for field in op["request"].as_array().into_iter().flatten() {
            let field = field.as_str().unwrap();
            let body = &spec_op["requestBody"]["content"]["application/json"]["schema"];
            if !has_field(&spec, body, field) {
                problems.push(format!("{method} {path}: request field `{field}` is gone"));
            }
        }
        for name in op["query"].as_array().into_iter().flatten() {
            let params = spec_op["parameters"].as_array().cloned().unwrap_or_default();
            if !params.iter().any(|p| p["in"] == "query" && p["name"] == *name) {
                problems.push(format!("{method} {path}: query parameter `{name}` is gone"));
            }
        }
        let response = success_schema(spec_op);
        for field in op["response"].as_array().into_iter().flatten() {
            let field = field.as_str().unwrap();
            if !response.is_some_and(|s| has_field(&spec, s, field)) {
                problems.push(format!("{method} {path}: response field `{field}` is gone"));
            }
        }
    }
    assert!(problems.is_empty(), "openapi/openapi.json no longer has what the CLI uses:\n  {}", problems.join("\n  "));
}

#[test]
fn field_lookup_works() {
    let spec: Value = serde_json::from_str(
        r##"{"components": {"schemas": {"P": {"type": "object", "properties": {"name": {"type": "string"}}}}},
            "s": {"type": "object", "properties": {"items": {"type": "array", "items": {"properties": {"project": {"$ref": "#/components/schemas/P"}}}}}}}"##,
    )
    .unwrap();
    assert!(has_field(&spec, &spec["s"], "items[].project.name"));
    assert!(!has_field(&spec, &spec["s"], "items[].project.id"));
    assert!(!has_field(&spec, &spec["s"], "items.project"));
    assert_eq!(normalize("/a/{id}/b/{}"), "/a/{}/b/{}");
}
