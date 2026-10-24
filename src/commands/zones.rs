//! `stargazer zones ...` (GET /api/v1/availability-zones).

use crate::cli::ZonesCmd;
use crate::context::Context;
use crate::error::CliError;
use crate::output::{col, print_list, Column};

use super::common::items;

const COLUMNS: &[Column] = &[col("ID", "id"), col("NAME", "name"), col("REGION", "region")];

pub fn run(cmd: &ZonesCmd, ctx: &Context) -> Result<(), CliError> {
    match cmd {
        ZonesCmd::List => {
            let zones = items(ctx.client()?.get("/api/v1/availability-zones")?, "availability_zones");
            print_list(ctx.format, &zones, COLUMNS, "No locations yet.");
            Ok(())
        }
    }
}
