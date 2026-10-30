//! The LOS bake (docs/specs/line-of-sight.md): `los-bake` (feature `bake`) turns a map
//! tile's terrain, opaque WMO batches and solid M2 doodads into one `.los` file, and
//! writes the eye table. The runtime (tiles, segment queries, eye table, bodies) is
//! `shared::los`, which the server and the client link.

pub use shared::los::*;
pub mod liquid;
pub mod wmo_liquid;

#[cfg(feature = "bake")]
pub mod bake;
