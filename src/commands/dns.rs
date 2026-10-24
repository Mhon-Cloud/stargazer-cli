//! `stargazer dns ...` (GET /api/v1/dns).

use serde_json::Value;

use crate::cli::DnsCmd;
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_list, Column};

use super::common::{in_project, items};

const COLUMNS: &[Column] = &[
    col("ID", "id"),
    col("NAME", "fqdn"),
    col("KIND", "kind"),
    col("STATUS", "status"),
    col("PROJECT", "project.name"),
];

pub fn run(cmd: &DnsCmd, ctx: &Context) -> Result<(), CliError> {
    match cmd {
        DnsCmd::List(filter) => {
            let records: Vec<Value> = items(ctx.client()?.get("/api/v1/dns")?, "records")
                .into_iter()
                .filter(|r| in_project(r, filter.project.as_deref()))
                .collect();
            print_list(ctx.format, &records, COLUMNS, "No DNS names.");
            Ok(())
        }
    }
}
