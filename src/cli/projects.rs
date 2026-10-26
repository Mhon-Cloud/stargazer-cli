//! `stargazer projects ...` and `stargazer zones ...`.

use clap::{Subcommand, ValueEnum};

use super::Confirm;

#[derive(Subcommand, Debug)]
pub enum ProjectsCmd {
    /// List your projects
    #[command(visible_alias = "ls")]
    List,
    /// One project
    Show {
        /// Project id or name
        project: String,
    },
    /// Create a project (free)
    Create {
        /// Its name (unique; no "Mhon Cloud")
        name: String,
        #[arg(long, default_value = "")]
        description: String,
        /// Personal, or a company's (needs --company)
        #[arg(long, value_enum, default_value_t = ProjectKind::Personal)]
        kind: ProjectKind,
        /// The company's id (kind company)
        #[arg(long)]
        company: Option<String>,
    },
    /// Rename a project, or change its description
    Rename {
        /// Project id or name
        project: String,
        /// The new name
        new_name: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a project and everything in it (needs --confirm <its name> and --yes)
    Delete {
        /// Project id or name
        project: String,
        /// The project's name, typed again
        #[arg(long, value_name = "NAME")]
        confirm: String,
        #[command(flatten)]
        confirm_yes: Confirm,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ProjectKind {
    Personal,
    Company,
}

#[derive(Subcommand, Debug)]
pub enum ZonesCmd {
    /// List the locations
    #[command(visible_alias = "ls")]
    List,
}
