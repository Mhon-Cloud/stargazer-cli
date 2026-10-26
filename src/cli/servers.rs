//! `stargazer servers ...`: virtual machines, and ordering them (Market -> Compute Instances).

use clap::{Args, Subcommand, ValueEnum};

use super::{Confirm, ProjectFilter, Wait};

#[derive(Args, Debug, Clone)]
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
    /// One server in full
    Show(ServerTarget),
    /// Order a new server (prepaid from your credit; --dry-run prices it, --yes pays)
    Create(Box<ServerCreate>),
    /// What an order can choose: OS images, plans, rates, limits
    Options,
    /// Your server orders (pending, active, suspended...)
    #[command(subcommand)]
    Orders(OrdersCmd),
    /// Start a server
    Start {
        #[command(flatten)]
        target: ServerTarget,
        #[command(flatten)]
        wait: Wait,
    },
    /// Stop a server
    Stop {
        #[command(flatten)]
        target: ServerTarget,
        #[command(flatten)]
        wait: Wait,
    },
    /// Rename a server
    Rename {
        #[command(flatten)]
        target: ServerTarget,
        /// The new name (a DNS name: letters, digits, hyphens)
        new_name: String,
        #[command(flatten)]
        wait: Wait,
    },
    /// Change CPU, memory or disk (owner / admin; a Market server is charged a month at the new size: needs --yes)
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
        #[command(flatten)]
        confirm: Confirm,
        #[command(flatten)]
        wait: Wait,
    },
    /// Delete a stopped server and its disks (needs --yes; unused paid time is refunded)
    Delete {
        #[command(flatten)]
        target: ServerTarget,
        #[command(flatten)]
        confirm: Confirm,
        #[command(flatten)]
        wait: Wait,
    },
    /// Pay one more month for a suspended server and start it (the buyer only; needs --yes)
    Renew {
        #[command(flatten)]
        target: ServerTarget,
        #[command(flatten)]
        confirm: Confirm,
    },
    /// Turn monthly auto-renew on or off
    AutoRenew {
        #[command(flatten)]
        target: ServerTarget,
        #[arg(value_enum)]
        state: OnOff,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum OnOff {
    On,
    Off,
}

#[derive(Args, Debug)]
pub struct ServerCreate {
    /// The server's name (a DNS label: lower-case letters, digits, hyphens)
    pub name: String,
    /// Project id or name (default: your personal Default project)
    #[arg(long)]
    pub project: Option<String>,
    /// Location id or name (default: the only one, if there's one)
    #[arg(long)]
    pub zone: Option<String>,
    /// OS image, as `servers options` lists it (e.g. ubuntu24.04)
    #[arg(long)]
    pub os: String,
    /// A plan's name from `servers options` (sets CPU / RAM / disk / public IP; its discount applies)
    #[arg(long)]
    pub plan: Option<String>,
    /// vCPU cores
    #[arg(long)]
    pub cpu: Option<u32>,
    /// RAM in GB
    #[arg(long)]
    pub ram: Option<u32>,
    /// A disk, TYPE:GB (ssd:50, hdd:200); repeat for more disks. The first is the boot disk
    #[arg(long = "disk", value_name = "TYPE:GB")]
    pub disks: Vec<String>,
    /// Give it a public IPv4 address
    #[arg(long, conflicts_with = "dns_label")]
    pub public_ip: bool,
    /// Without a public IP: a free <label>.mhon.cloud name (default: the server's name)
    #[arg(long)]
    pub dns_label: Option<String>,
    /// The login user (cloud-init; default: your username)
    #[arg(long)]
    pub username: Option<String>,
    /// Read the login password from stdin (default: a random one, printed once and sent to your inbox)
    #[arg(long, conflicts_with = "no_password")]
    pub password_stdin: bool,
    /// No password: SSH keys only (needs --ssh-key)
    #[arg(long)]
    pub no_password: bool,
    /// An SSH key to install (id or name from `ssh-keys list`); repeat for more
    #[arg(long = "ssh-key", value_name = "KEY")]
    pub ssh_keys: Vec<String>,
    /// Automatic backups: daily or a weekday (mon...sun), at a time: daily@03:00, sun@02:30
    #[arg(long, value_name = "WHEN")]
    pub backup: Option<String>,
    /// Months paid now: 1, 6 (discount) or 12 (bigger discount); renewals are monthly
    #[arg(long, default_value_t = 1, value_parser = super::parse_period)]
    pub period: u32,
    /// A coupon code
    #[arg(long)]
    pub coupon: Option<String>,
    /// Only price it: show the quote, order nothing
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub confirm: Confirm,
    #[command(flatten)]
    pub wait: Wait,
}

#[derive(Subcommand, Debug)]
pub enum OrdersCmd {
    /// List your server orders
    #[command(visible_alias = "ls")]
    List,
    /// Cancel an order still waiting for a host (full refund)
    Cancel {
        /// Order id or number (ORD-000001)
        order: String,
        #[command(flatten)]
        confirm: Confirm,
    },
}
