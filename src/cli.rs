//! Every command, flag and help text (clap derive). `--help` is what people and AI agents read
//! to learn the CLI: keep each line short and say what the command does, not how.
//! A new command goes here, then in `commands::dispatch`, then in docs/commands.md.

use clap::{Args, Parser, Subcommand};

use crate::output::OutputArg;

const AFTER_HELP: &str = "\
Sign in once with `stargazer login` (or set STARGAZER_API_KEY). Create a key in the console: Overview > API.
Output is a table on a terminal and JSON otherwise; force one with --output json|table.
Exit codes: 0 ok, 1 internal, 2 usage, 3 auth, 4 not found, 5 not enough credit, 6 conflict, 7 rate limited, 8 network.";

#[derive(Parser, Debug)]
#[command(
    name = "stargazer",
    version,
    about = "Control Stargazer (Mhon Cloud) from the command line: servers, containers, DNS names and credit.",
    after_help = AFTER_HELP,
    propagate_version = true
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug)]
pub struct GlobalArgs {
    /// Output format (auto = table on a terminal, JSON otherwise)
    #[arg(long, short = 'o', global = true, value_enum, default_value_t = OutputArg::Auto, env = "STARGAZER_OUTPUT")]
    pub output: OutputArg,

    /// API base URL [default: https://app.mhon.cloud]
    #[arg(long, global = true, env = "STARGAZER_API_URL", value_name = "URL")]
    pub api_url: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Show the CLI's version
    Version,
    /// Save an API key (read from stdin, or asked for on a terminal)
    Login(LoginArgs),
    /// Remove the saved API key
    Logout,
    /// Show the account the API key belongs to
    Whoami,
    /// Show your credit
    Credit,
    /// Your projects
    #[command(subcommand)]
    Projects(ProjectsCmd),
    /// Locations you can order in
    #[command(subcommand)]
    Zones(ZonesCmd),
    /// Virtual machines: list, start, stop, rename, resize, delete
    #[command(subcommand)]
    Servers(ServersCmd),
    /// Containers: list, start, stop, delete, logs
    #[command(subcommand)]
    Containers(ContainersCmd),
    /// DNS names (<name>.in.mhon.cloud)
    #[command(subcommand)]
    Dns(DnsCmd),
    /// Print a shell completion script (bash, zsh, fish, powershell, elvish)
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Read the key from stdin even on a terminal (e.g. `echo "$KEY" | stargazer login --api-key-stdin`)
    #[arg(long)]
    pub api_key_stdin: bool,
}

/// `--project` on lists and actions: a project id or name.
#[derive(Args, Debug)]
pub struct ProjectFilter {
    /// Only this project (id or name)
    #[arg(long, value_name = "PROJECT")]
    pub project: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum ProjectsCmd {
    /// List your projects
    #[command(visible_alias = "ls")]
    List,
}

#[derive(Subcommand, Debug)]
pub enum ZonesCmd {
    /// List the locations
    #[command(visible_alias = "ls")]
    List,
}

#[derive(Args, Debug)]
pub struct ServerTarget {
    /// Server id or name
    pub server: String,
    #[command(flatten)]
    pub project: ProjectFilter,
}

#[derive(Subcommand, Debug)]
pub enum ServersCmd {
    /// List your servers
    #[command(visible_alias = "ls")]
    List(ProjectFilter),
    /// Start a server
    Start(ServerTarget),
    /// Stop a server
    Stop(ServerTarget),
    /// Rename a server
    Rename {
        #[command(flatten)]
        target: ServerTarget,
        /// The new name (a DNS name: letters, digits, hyphens)
        new_name: String,
    },
    /// Change CPU, memory or disk (owner / admin; a Market server is charged a month at the new size)
    Resize {
        #[command(flatten)]
        target: ServerTarget,
        /// vCPU cores
        #[arg(long)]
        cores: Option<u32>,
        /// Memory in MB
        #[arg(long)]
        memory_mb: Option<u32>,
        /// Boot disk size in GB (grow only)
        #[arg(long)]
        disk_gb: Option<u32>,
    },
    /// Delete a stopped server and its disks (needs --yes)
    Delete {
        #[command(flatten)]
        target: ServerTarget,
        /// Confirm: the server and its disks are destroyed
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Args, Debug)]
pub struct ContainerTarget {
    /// Container id, number (CT-000001) or name
    pub container: String,
    #[command(flatten)]
    pub project: ProjectFilter,
}

#[derive(Subcommand, Debug)]
pub enum ContainersCmd {
    /// List your containers
    #[command(visible_alias = "ls")]
    List(ProjectFilter),
    /// Start a container
    Start(ContainerTarget),
    /// Stop a container
    Stop(ContainerTarget),
    /// Delete a container (or cancel one not created yet; needs --yes)
    Delete {
        #[command(flatten)]
        target: ContainerTarget,
        /// Confirm: the container and its volumes are destroyed
        #[arg(long)]
        yes: bool,
    },
    /// Print a container's latest logs
    Logs {
        #[command(flatten)]
        target: ContainerTarget,
        /// How many lines from the end
        #[arg(long, default_value_t = 200)]
        tail: u32,
    },
}

#[derive(Subcommand, Debug)]
pub enum DnsCmd {
    /// List your DNS names
    #[command(visible_alias = "ls")]
    List(ProjectFilter),
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    #[test]
    fn cli_is_well_formed() {
        super::Cli::command().debug_assert();
    }
}
