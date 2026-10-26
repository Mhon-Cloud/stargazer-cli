//! `stargazer projects ...`: GET/POST /api/v1/projects, GET/PUT/DELETE /api/v1/projects/{id}.
//! Deleting a project deletes everything in it (servers, containers, DNS names...), so it
//! needs the name typed again (`--confirm`) and `--yes`, like the console.

use serde_json::json;

use crate::cli::{ProjectKind, ProjectsCmd};
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_list, print_object, Column};

use super::common::{id_of, PROJECTS};

const COLUMNS: &[Column] = &[
    col("ID", "id"),
    col("NAME", "name"),
    col("KIND", "kind"),
    col("COMPANY", "company_name"),
    col("ROLE", "my_role"),
    col("MEMBERS", "member_count"),
];

const DETAIL: &[Column] = &[
    col("ID", "id"),
    col("Name", "name"),
    col("Description", "description"),
    col("Kind", "kind"),
    col("Default", "is_default"),
    col("Company", "company_name"),
    col("Your role", "my_role"),
    col("Members", "member_count"),
    col("Deleting since", "deleting_at"),
    col("Created", "created_at"),
];

pub fn run(cmd: &ProjectsCmd, ctx: &Context) -> Result<(), CliError> {
    let client = ctx.client()?;
    match cmd {
        ProjectsCmd::List => {
            print_list(ctx.format, &PROJECTS.list(&client)?, COLUMNS, "No projects.");
            Ok(())
        }
        ProjectsCmd::Show { project } => {
            print_object(ctx.format, &PROJECTS.resolve(&client, project, None)?, DETAIL);
            Ok(())
        }
        ProjectsCmd::Create { name, description, kind, company } => {
            let mut body = json!({ "name": name, "description": description });
            match kind {
                ProjectKind::Personal => body["kind"] = json!("personal"),
                ProjectKind::Company => {
                    let company =
                        company.as_ref().ok_or_else(|| CliError::usage("a company project needs --company <id>"))?;
                    body["kind"] = json!("company");
                    body["company_id"] = json!(company);
                }
            }
            let answer = client.post("/api/v1/projects", Some(body))?;
            print_object(ctx.format, answer.get("project").unwrap_or(&answer), DETAIL);
            Ok(())
        }
        ProjectsCmd::Rename { project, new_name, description } => {
            let found = PROJECTS.resolve(&client, project, None)?;
            let description =
                description.clone().unwrap_or_else(|| found["description"].as_str().unwrap_or("").to_string());
            let answer = client.put(
                &format!("/api/v1/projects/{}", id_of(&found)?),
                json!({ "name": new_name, "description": description }),
            )?;
            print_object(ctx.format, answer.get("project").unwrap_or(&answer), DETAIL);
            Ok(())
        }
        ProjectsCmd::Delete { project, confirm, confirm_yes } => {
            let found = PROJECTS.resolve(&client, project, None)?;
            let name = found["name"].as_str().unwrap_or_default();
            if confirm != name {
                return Err(CliError::usage(format!("--confirm must be the project's name exactly: \"{name}\"")));
            }
            if !confirm_yes.yes {
                return Err(CliError::usage(
                    "this stops and deletes every server, container and DNS name in the project; add --yes to confirm",
                ));
            }
            let answer =
                client.delete_with(&format!("/api/v1/projects/{}", id_of(&found)?), json!({ "confirm_name": name }))?;
            print_object(ctx.format, answer.get("project").unwrap_or(&answer), DETAIL);
            Ok(())
        }
    }
}
