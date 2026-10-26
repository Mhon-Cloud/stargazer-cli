//! Security: SSH keys, security groups, backups (the console's Inventory -> Security).

use clap::{Subcommand, ValueEnum};

use super::{Confirm, ProjectFilter};

#[derive(Subcommand, Debug)]
pub enum SshKeysCmd {
    /// List your SSH keys
    #[command(visible_alias = "ls")]
    List(ProjectFilter),
    /// Add an SSH public key to a project
    Add {
        /// A name for it
        name: String,
        /// The public key file (e.g. ~/.ssh/id_ed25519.pub); `-` reads stdin
        #[arg(long, value_name = "FILE")]
        file: String,
        /// Project id or name (default: your personal Default project)
        #[arg(long)]
        project: Option<String>,
    },
    /// Rename an SSH key
    Rename {
        /// Key id or name
        key: String,
        new_name: String,
    },
    /// Delete an SSH key (remove it from every server first; needs --yes)
    Delete {
        /// Key id or name
        key: String,
        #[command(flatten)]
        confirm: Confirm,
    },
    /// Install an SSH key on a server (applied at its next boot)
    Install {
        /// Key id or name
        key: String,
        /// Server id or name
        server: String,
    },
    /// Remove an SSH key from a server
    Uninstall {
        /// Key id or name
        key: String,
        /// Server id or name
        server: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum SecurityGroupsCmd {
    /// List your security groups
    #[command(visible_alias = "ls")]
    List(ProjectFilter),
    /// One group with its rules and servers
    Show {
        /// Group id or name
        group: String,
    },
    /// Create a group (allow-all outbound, no inbound)
    Create {
        name: String,
        #[arg(long, default_value = "")]
        description: String,
        /// Project id or name (default: your personal Default project)
        #[arg(long)]
        project: Option<String>,
    },
    /// Rename a group, or change its description
    Rename {
        /// Group id or name
        group: String,
        new_name: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a group (detached and unreferenced; needs --yes)
    Delete {
        /// Group id or name
        group: String,
        #[command(flatten)]
        confirm: Confirm,
    },
    /// Allow traffic: add a rule
    Allow {
        /// Group id or name
        group: String,
        /// Inbound or outbound
        #[arg(long, value_enum, default_value_t = Direction::Ingress)]
        direction: Direction,
        #[arg(long, value_enum, default_value_t = Protocol::Tcp)]
        protocol: Protocol,
        /// A port or range: 22, 8000-8100 (tcp / udp)
        #[arg(long)]
        port: Option<String>,
        /// From / to this network (default 0.0.0.0/0, or --peer)
        #[arg(long, conflicts_with = "peer")]
        cidr: Option<String>,
        /// From / to another group of the same project (id or name)
        #[arg(long)]
        peer: Option<String>,
        #[arg(long, default_value = "")]
        description: String,
    },
    /// Remove a rule
    Revoke {
        /// Group id or name
        group: String,
        /// Rule id (from `security-groups show`)
        rule: String,
    },
    /// Attach a group to a server (up to 5 per server; applied at once)
    Attach {
        /// Group id or name
        group: String,
        /// Server id or name
        server: String,
    },
    /// Detach a group from a server
    Detach {
        /// Group id or name
        group: String,
        /// Server id or name
        server: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Direction {
    Ingress,
    Egress,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Protocol {
    All,
    Tcp,
    Udp,
    Icmp,
}

#[derive(Subcommand, Debug)]
pub enum BackupsCmd {
    /// List the backups of your servers
    #[command(visible_alias = "ls")]
    List {
        /// Only this server (id or name)
        #[arg(long)]
        server: Option<String>,
    },
    /// Back a server up now
    Create {
        /// Server id or name
        server: String,
        /// Backup storage (default: the first one the server can use)
        #[arg(long)]
        storage: Option<String>,
        #[arg(long, default_value = "")]
        notes: String,
    },
    /// Restore a backup over its server (the server must be stopped; needs --yes)
    Restore {
        /// Server id or name
        server: String,
        /// The backup's volid (from `backups list`)
        volid: String,
        #[command(flatten)]
        confirm: Confirm,
    },
    /// Delete a backup (needs --yes)
    Delete {
        /// Server id or name
        server: String,
        /// The backup's volid (from `backups list`)
        volid: String,
        #[command(flatten)]
        confirm: Confirm,
    },
}
