//! Printing results and errors. Two formats:
//! - **table**: for people (the default on a terminal): aligned columns, `-` for empty values.
//! - **json**: for scripts and AI agents (the default when stdout isn't a terminal): the API's own
//!   objects, one JSON document on stdout, compact. Lists print as a JSON array.
//!
//! Errors go to stderr: `error: <message>` in table mode, `{"error": {...}}` in JSON mode.
//! Nothing else is ever printed in JSON mode (no colours, spinners or prompts).
//!
//! `--fields a,b.c` (set once by `lib::run`) trims JSON output to those paths (fewer tokens for
//! agents) and, in table mode, picks the columns.

use std::io::{IsTerminal, Write};
use std::sync::OnceLock;

use serde_json::{json, Value};

use crate::error::CliError;

/// The `--output` flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputArg {
    Auto,
    Json,
    Table,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Json,
    Table,
}

impl Format {
    pub fn resolve(arg: OutputArg) -> Format {
        match arg {
            OutputArg::Json => Format::Json,
            OutputArg::Table => Format::Table,
            OutputArg::Auto if std::io::stdout().is_terminal() => Format::Table,
            OutputArg::Auto => Format::Json,
        }
    }
}

/// A table column: its header and where its value is in each item (dot path, e.g. `project.name`).
pub struct Column {
    pub header: &'static str,
    pub path: &'static str,
}

pub const fn col(header: &'static str, path: &'static str) -> Column {
    Column { header, path }
}

static NULL: Value = Value::Null;

static FIELDS: OnceLock<Vec<String>> = OnceLock::new();

/// `--fields`: empty = everything.
pub fn set_fields(fields: Vec<String>) {
    let fields: Vec<String> = fields.into_iter().map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect();
    let _ = FIELDS.set(fields);
}

fn fields() -> &'static [String] {
    FIELDS.get().map(Vec::as_slice).unwrap_or(&[])
}

/// Only `paths` of an object (nested paths keep their nesting); arrays are trimmed item by item.
pub fn pick(value: &Value, paths: &[String]) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(|v| pick(v, paths)).collect()),
        Value::Object(_) => {
            let mut out = Value::Object(Default::default());
            for path in paths {
                let mut at = &mut out;
                let keys: Vec<&str> = path.split('.').collect();
                for (i, key) in keys.iter().enumerate() {
                    if i + 1 == keys.len() {
                        at[*key] = lookup(value, path).clone();
                    } else {
                        if !at[*key].is_object() {
                            at[*key] = Value::Object(Default::default());
                        }
                        at = &mut at[*key];
                    }
                }
            }
            out
        }
        other => other.clone(),
    }
}

/// The columns to show: `--fields` when given, else the command's own.
fn columns_for(columns: &[Column]) -> Vec<Column> {
    if fields().is_empty() {
        columns.iter().map(|c| col(c.header, c.path)).collect()
    } else {
        // Leaked: a handful of short strings, once per process.
        fields()
            .iter()
            .map(|f| col(Box::leak(f.to_uppercase().into_boxed_str()), Box::leak(f.clone().into_boxed_str())))
            .collect()
    }
}

/// The value at a dot path (`zone.name`), or null.
pub fn lookup<'a>(value: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(value, |v, key| v.get(key).unwrap_or(&NULL))
}

/// How one value reads in a table cell.
pub fn cell(value: &Value) -> String {
    match value {
        Value::Null => "-".to_string(),
        Value::String(s) if s.is_empty() => "-".to_string(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "yes" } else { "no" }.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(items) if items.is_empty() => "-".to_string(),
        Value::Array(items) => items.iter().map(cell).collect::<Vec<_>>().join(", "),
        Value::Object(map) => match map.get("name") {
            Some(name) => cell(name),
            None => value.to_string(),
        },
    }
}

fn out(text: &str) {
    // Ignore write errors: `stargazer ... | head` closing the pipe isn't a failure.
    let _ = writeln!(std::io::stdout().lock(), "{text}");
}

fn json_text(value: &Value) -> String {
    if std::io::stdout().is_terminal() {
        serde_json::to_string_pretty(value).unwrap_or_default()
    } else {
        value.to_string()
    }
}

pub fn print_json(value: &Value) {
    if fields().is_empty() {
        out(&json_text(value));
    } else {
        out(&json_text(&pick(value, fields())));
    }
}

/// A list: a table with `columns`, or the items as a JSON array. `empty` is shown (table only)
/// when there's nothing.
pub fn print_list(format: Format, items: &[Value], columns: &[Column], empty: &str) {
    if format == Format::Json {
        print_json(&Value::Array(items.to_vec()));
        return;
    }
    if items.is_empty() {
        out(empty);
        return;
    }
    let columns = columns_for(columns);
    let rows: Vec<Vec<String>> =
        items.iter().map(|item| columns.iter().map(|c| cell(lookup(item, c.path))).collect()).collect();
    out(&render_table(&columns, &rows));
}

/// One object: `Field  value` lines, or the object as JSON.
pub fn print_object(format: Format, value: &Value, fields: &[Column]) {
    if format == Format::Json {
        print_json(value);
        return;
    }
    let fields = columns_for(fields);
    let width = fields.iter().map(|f| f.header.chars().count()).max().unwrap_or(0);
    let lines: Vec<String> =
        fields.iter().map(|f| format!("{:width$}  {}", f.header, cell(lookup(value, f.path)))).collect();
    out(&lines.join("\n"));
}

/// A result with no natural table: `json` for JSON output, `text` for people.
pub fn print_message(format: Format, json: Value, text: &str) {
    match format {
        Format::Json => print_json(&json),
        Format::Table => out(text),
    }
}

pub fn print_error(format: Format, err: &CliError) {
    let mut stderr = std::io::stderr().lock();
    let _ = match format {
        Format::Json => writeln!(
            stderr,
            "{}",
            json!({ "error": { "code": err.kind.code(), "message": err.message, "status": err.status } })
        ),
        Format::Table => writeln!(stderr, "error: {}", err.message),
    };
}

pub fn render_table(columns: &[Column], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = columns.iter().map(|c| c.header.chars().count()).collect();
    for row in rows {
        for (i, value) in row.iter().enumerate() {
            widths[i] = widths[i].max(value.chars().count());
        }
    }
    let line = |values: Vec<&str>| {
        values
            .iter()
            .enumerate()
            .map(|(i, v)| if i + 1 == values.len() { v.to_string() } else { format!("{:w$}", v, w = widths[i]) })
            .collect::<Vec<_>>()
            .join("  ")
    };
    let mut lines = vec![line(columns.iter().map(|c| c.header).collect())];
    lines.extend(rows.iter().map(|r| line(r.iter().map(String::as_str).collect())));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_follows_paths() {
        let v = json!({ "project": { "name": "web" }, "zone": null });
        assert_eq!(lookup(&v, "project.name"), &json!("web"));
        assert_eq!(lookup(&v, "zone.name"), &Value::Null);
    }

    #[test]
    fn cells_read_well() {
        assert_eq!(cell(&Value::Null), "-");
        assert_eq!(cell(&json!("")), "-");
        assert_eq!(cell(&json!(true)), "yes");
        assert_eq!(cell(&json!(["a", "b"])), "a, b");
        assert_eq!(cell(&json!({ "name": "bkk-1" })), "bkk-1");
    }

    #[test]
    fn pick_keeps_only_the_paths() {
        let v = json!([{ "id": "1", "name": "web", "project": { "id": "p", "name": "Shop" }, "x": 1 }]);
        let paths = vec!["id".to_string(), "project.name".to_string(), "missing".to_string()];
        assert_eq!(pick(&v, &paths), json!([{ "id": "1", "project": { "name": "Shop" }, "missing": null }]));
    }

    #[test]
    fn tables_align() {
        let cols = [col("ID", "id"), col("NAME", "name")];
        let rows = vec![vec!["1".to_string(), "web".to_string()], vec!["22".to_string(), "db".to_string()]];
        assert_eq!(render_table(&cols, &rows), "ID  NAME\n1   web\n22  db");
    }
}
