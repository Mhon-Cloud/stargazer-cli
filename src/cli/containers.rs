//! `stargazer containers ...`: containers, and ordering them (Market -> Containers).

use clap::{Args, Subcommand};

use super::{Confirm, ProjectFilter, Wait};

#[derive(Args, Debug, Clone)]
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
    /// One container in full
    Show(ContainerTarget),
    /// Order a container (free within 1 vCPU / 1 GB / 5 GB; else prepaid: needs --yes)
    Create(Box<ContainerCreate>),
    /// The images `create --image` knows by name, with their ports and variables
    Images,
    /// What an order can choose: free plan left, plans, rates, locations
    Options,
    /// Start a container
    Start {
        #[command(flatten)]
        target: ContainerTarget,
        #[command(flatten)]
        wait: Wait,
    },
    /// Stop a container
    Stop {
        #[command(flatten)]
        target: ContainerTarget,
        #[command(flatten)]
        wait: Wait,
    },
    /// Delete a container (or cancel one not created yet; needs --yes)
    Delete {
        #[command(flatten)]
        target: ContainerTarget,
        #[command(flatten)]
        confirm: Confirm,
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

#[derive(Args, Debug)]
pub struct ContainerCreate {
    /// The container's name (a DNS label: lower-case letters, digits, hyphens)
    pub name: String,
    /// An image by name (`containers images`: ubuntu, nginx, postgres, minecraft...) or any public image (nginx:1.27)
    #[arg(long)]
    pub image: String,
    /// Project id or name (default: your personal Default project)
    #[arg(long)]
    pub project: Option<String>,
    /// Location id or name (default: the only one that runs containers, if there's one)
    #[arg(long)]
    pub zone: Option<String>,
    /// A plan's name from `containers options` (sets CPU / RAM / storage; its discount applies)
    #[arg(long)]
    pub plan: Option<String>,
    /// vCPU cores (default: the image's minimum, else 1)
    #[arg(long)]
    pub cpu: Option<u32>,
    /// RAM in GB (default 1; Minecraft 2)
    #[arg(long)]
    pub ram: Option<u32>,
    /// Storage in GB (default 5)
    #[arg(long)]
    pub storage: Option<u32>,
    /// A port, PORT/PROTOCOL (80/http, 22/tcp, 19132/udp); repeat for more. Default: the image's.
    /// One HTTP port gets the DNS name (HTTPS); TCP / UDP ports get a public port
    #[arg(long = "port", value_name = "PORT/PROTO")]
    pub ports: Vec<String>,
    /// An environment variable, KEY=VALUE; repeat for more (overrides the image's defaults)
    #[arg(long = "env", short = 'e', value_name = "KEY=VALUE")]
    pub env: Vec<String>,
    /// Command to run instead of the image's
    #[arg(long)]
    pub command: Option<String>,
    /// DNS name <name>.in.mhon.cloud (default: the container's name)
    #[arg(long, conflicts_with = "no_dns")]
    pub dns_name: Option<String>,
    /// No DNS name
    #[arg(long)]
    pub no_dns: bool,
    /// Minecraft: an SRV record, so players type just the name (default: on for port 25565)
    #[arg(long)]
    pub minecraft_srv: Option<bool>,
    /// Ubuntu image: the login user (default: your username)
    #[arg(long)]
    pub username: Option<String>,
    /// Ubuntu image: read the login password from stdin (default: a random one, printed once)
    #[arg(long)]
    pub password_stdin: bool,
    /// Ubuntu image: a bash script run as root at every start (a file; up to 16 KB)
    #[arg(long, value_name = "FILE")]
    pub startup_script: Option<std::path::PathBuf>,
    /// Upload a folder into the container, LOCAL_DIR:/mount/path; up to 3, 1 GB each
    #[arg(long = "upload", value_name = "DIR:PATH")]
    pub uploads: Vec<String>,
    /// Months paid now: 1, 6 or 12 (the free plan is always 1)
    #[arg(long, default_value_t = 1, value_parser = super::parse_period)]
    pub period: u32,
    #[command(flatten)]
    pub confirm: Confirm,
    #[command(flatten)]
    pub wait: Wait,
}
