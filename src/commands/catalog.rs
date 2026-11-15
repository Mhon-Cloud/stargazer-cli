//! The container images `containers create --image` knows by name: the console's list
//! (management_page/frontend/src/lib/container-order.ts in the platform), with the ports and
//! variables each one needs. Keep the two in step; any other public image works by reference.

use serde_json::{json, Value};

pub struct Image {
    /// What people type: `--image nginx`.
    pub alias: &'static str,
    pub image: &'static str,
    pub name: &'static str,
    /// PORT, protocol (http | tcp | udp).
    pub ports: &'static [(u32, &'static str)],
    /// Variables set unless the order sets them.
    pub env: &'static [(&'static str, &'static str)],
    /// Variables the order must set.
    pub required: &'static [&'static str],
    /// The platform's dev image: asks for a login, takes a startup script.
    pub login: bool,
    pub min_ram_gb: u32,
}

/// MB Java needs on top of its heap. The container's RAM is a hard limit with no swap, so a heap as big as
/// the RAM gets a Java server (Minecraft) killed and restarted over and over.
pub const JAVA_OVERHEAD_MB: u32 = 512;

/// The Java heap (`MEMORY` / `MAX_MEMORY`) for `ram_gb` of RAM: three quarters of it, leaving at least
/// JAVA_OVERHEAD_MB (1 GB -> 512M, 2 -> 1536M, 4 -> 3G). The console's order form uses the same rule.
pub fn java_heap(ram_gb: u32) -> String {
    let ram = ram_gb * 1024;
    let mb = (ram * 3 / 4).min(ram.saturating_sub(JAVA_OVERHEAD_MB)).max(256);
    if mb.is_multiple_of(1024) {
        format!("{}G", mb / 1024)
    } else {
        format!("{mb}M")
    }
}

/// The platform's own Ubuntu image (SSH + code-server), built on every container node.
pub const DEV_IMAGE: &str = "mhon.local/ubuntu-dev:24.04";

pub const IMAGES: &[Image] = &[
    Image {
        alias: "ubuntu",
        image: DEV_IMAGE,
        name: "Ubuntu 24.04: SSH + VS Code in the browser (code-server)",
        ports: &[(22, "tcp"), (8080, "http")],
        env: &[],
        required: &[],
        login: true,
        min_ram_gb: 1,
    },
    Image {
        alias: "minecraft",
        image: "itzg/minecraft-server:latest",
        name: "Minecraft Java server (TYPE=VANILLA|FORGE|NEOFORGE; OPS = your Minecraft name)",
        ports: &[(25565, "tcp")],
        env: &[("EULA", "TRUE"), ("VERSION", "1.21.11"), ("TYPE", "VANILLA")],
        required: &[],
        login: false,
        min_ram_gb: 2,
    },
    Image {
        alias: "nginx",
        image: "nginx:1.27-alpine",
        name: "Nginx: a static website or a reverse proxy",
        ports: &[(80, "http")],
        env: &[],
        required: &[],
        login: false,
        min_ram_gb: 1,
    },
    Image {
        alias: "postgres",
        image: "postgres:17-alpine",
        name: "PostgreSQL 17",
        ports: &[(5432, "tcp")],
        env: &[],
        required: &["POSTGRES_PASSWORD"],
        login: false,
        min_ram_gb: 1,
    },
    Image {
        alias: "wordpress",
        image: "wordpress:6.7-apache",
        name: "WordPress (needs a MySQL database)",
        ports: &[(80, "http")],
        env: &[],
        required: &["WORDPRESS_DB_HOST", "WORDPRESS_DB_USER", "WORDPRESS_DB_PASSWORD", "WORDPRESS_DB_NAME"],
        login: false,
        min_ram_gb: 1,
    },
    Image {
        alias: "python",
        image: "python:3.13-slim",
        name: "Python 3.13 (your app listens on 8000)",
        ports: &[(8000, "http")],
        env: &[],
        required: &[],
        login: false,
        min_ram_gb: 1,
    },
    Image {
        alias: "node",
        image: "node:22-alpine",
        name: "Node.js 22 (your app listens on 3000)",
        ports: &[(3000, "http")],
        env: &[],
        required: &[],
        login: false,
        min_ram_gb: 1,
    },
    Image {
        alias: "uptime-kuma",
        image: "louislam/uptime-kuma:1",
        name: "Uptime Kuma: watch websites and servers",
        ports: &[(3001, "http")],
        env: &[],
        required: &[],
        login: false,
        min_ram_gb: 1,
    },
    Image {
        alias: "mysql",
        image: "mysql:8.4",
        name: "MySQL 8.4",
        ports: &[(3306, "tcp")],
        env: &[],
        required: &["MYSQL_ROOT_PASSWORD"],
        login: false,
        min_ram_gb: 1,
    },
    Image {
        alias: "redis",
        image: "redis:7.4-alpine",
        name: "Redis 7.4",
        ports: &[(6379, "tcp")],
        env: &[],
        required: &[],
        login: false,
        min_ram_gb: 1,
    },
    Image {
        alias: "mongo",
        image: "mongo:8.0",
        name: "MongoDB 8.0",
        ports: &[(27017, "tcp")],
        env: &[],
        required: &["MONGO_INITDB_ROOT_USERNAME", "MONGO_INITDB_ROOT_PASSWORD"],
        login: false,
        min_ram_gb: 1,
    },
];

/// The catalog entry for an alias (`nginx`) or an exact image (`nginx:1.27-alpine`).
pub fn find(wanted: &str) -> Option<&'static Image> {
    IMAGES.iter().find(|i| i.alias.eq_ignore_ascii_case(wanted) || i.image == wanted)
}

/// The catalog as JSON (`containers images`).
pub fn as_json() -> Value {
    Value::Array(
        IMAGES
            .iter()
            .map(|i| {
                json!({
                    "alias": i.alias,
                    "image": i.image,
                    "name": i.name,
                    "ports": i.ports.iter().map(|(p, proto)| format!("{p}/{proto}")).collect::<Vec<_>>(),
                    "required_env": i.required,
                    "min_ram_gb": i.min_ram_gb,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_by_alias_or_image() {
        assert_eq!(find("NGINX").unwrap().image, "nginx:1.27-alpine");
        assert_eq!(find(DEV_IMAGE).unwrap().alias, "ubuntu");
        assert!(find("nginx:latest").is_none());
    }

    #[test]
    fn java_heap_leaves_room_for_java() {
        let heaps: Vec<String> = [1, 2, 3, 4, 8].into_iter().map(java_heap).collect();
        assert_eq!(heaps, ["512M", "1536M", "2304M", "3G", "6G"]);
    }

    #[test]
    fn aliases_are_unique() {
        let mut aliases: Vec<&str> = IMAGES.iter().map(|i| i.alias).collect();
        aliases.sort();
        aliases.dedup();
        assert_eq!(aliases.len(), IMAGES.len());
    }
}
