//! `stargazer projects ...` (GET /api/v1/projects).

use crate::cli::ProjectsCmd;
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_list, Column};

use super::common::items;

const COLUMNS: &[Column] = &[
    col("ID", "id"),
    col("NAME", "name"),
    col("KIND", "kind"),
    col("COMPANY", "company_name"),
    col("ROLE", "my_role"),
    col("MEMBERS", "member_count"),
];

pub fn run(cmd: &ProjectsCmd, ctx: &Context) -> Result<(), CliError> {
    match cmd {
        ProjectsCmd::List => {
            let projects = items(ctx.client()?.get("/api/v1/projects")?, "projects");
            print_list(ctx.format, &projects, COLUMNS, "No projects.");
            Ok(())
        }
    }
}
