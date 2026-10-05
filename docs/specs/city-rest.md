# Capital city rest

`crates/server/src/rest.rs` grants rested XP in friendly capital territory as well as inns. [Experience wiki](../wiki/systems/experience.md) describes the data and lifecycle.

## What it must do

- [x] A human in Stormwind's Trade District rests and gains rested XP at the existing resting rate.
- [x] A human in Orgrimmar does not rest or gain rested XP from city territory; an orc does.
- [x] A human in Elwynn Forest outside an inn does not rest or gain rested XP.
- [x] Leaving a friendly capital settles earned rest and stops further city accrual.
- [x] Rest near an innkeeper remains unchanged.

## How it works

- [Experience](../wiki/systems/experience.md#rested-xp)

## Implementation inventory

- `scripts/db2.py`, `scripts/db2_schema.sql` — normal AreaTable DB2 import.
- `scripts/import-area-table.py` — AreaTable-only import into an existing world database.
- `crates/server/src/area_table.rs` — area parent-zone and faction territory lookup.
- `crates/server/src/terrain_height.rs`, `ground.rs` — position-derived MCNK area ID.
- `crates/server/src/world_data.rs` — startup metadata load.
- `crates/server/src/rest.rs` — city detection, existing accrual and owner snapshot lifecycle.

## Tests asserting this spec

- `crates/server/src/rest.rs::tests` — real WDT/ADT positions, AreaTable rows and innkeeper data; 41 targeted tests pass at `39f31b2`.
- `scripts/tests/test_area_table.py` — import preservation, rollback and header rejection; three tests pass at `39f31b2`.

## Known gaps (current cycle)

None. Canonical data import remains a deployment prerequisite, not authorized worker activity.

## Out of scope

- War Mode and quest-forced PvP (not represented by the server).
- WMO group-specific area overrides; city detection uses authored ADT areas.
- Live runtime and canonical data import: delegated worker cannot mutate shared world.db or run the live server.
