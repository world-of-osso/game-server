# Follower dungeons and NPC companions

Retail follower dungeons must group a solo player with four role-aware NPCs and complete the instance using authentic NPC spell kits. This cycle prepares reference data in [`scripts/follower_import.py`](../../scripts/follower_import.py); it does not implement the queue or combat AI. [Local data audit and lead-only import](../wiki/investigations/follower-data.md).

## What it must do

### Reference preparation
- [x] Decode the pinned Retail LFGDungeons layout; retain follower difficulty entries, authored role counts, map IDs, conditions, flags and content tuning. Ruby Life Pools2436 must resolve to map2521 and a1/1/3 composition.
- [x] Preserve TDB creature-template spell assignments, indexes and VerifiedBuild without inventing or substituting spells.
- [x] Generate re-applicable SQL replacing only the two prepared reference tables; reject missing spell data or a wrong DB2 layout before publishing SQL.
- [x] Audit canonical world.db read-only; distinguish reference spawns from runtime content and report absent escort spell assignments.
- [x] Refuse a CLI output/report destination equal to an input or the audited database.

### Gameplay (user requirement; not implemented)
- [ ] Solo follower queue creates a five-member party, fills tank/healer/DPS roles, enters an instance with canonical authored spawns, follows/regroups and finishes the dungeon.
- [ ] Tanks manage threat/positioning; healers prioritize injured party members; DPS use authored rotations. All roles reuse server spell execution, navigation and relevant interrupts/defensives/ground-effect avoidance.
- [ ] Extend to all13 classes/specs only where authentic companion data exists; missing class/spec kits remain explicit gaps, never guessed player-spell substitutions.
- [ ] Prove the party fighting on private UDP5750, with inspected Modern and Forever PNGs. Forever remains an art skin of the same behavior.

## How it works
- [Source audit, importer and runtime boundary](../wiki/investigations/follower-data.md)

## Implementation inventory
- `scripts/follower_import.py` — standalone SQL preparation and optional read-only audit; no runtime consumer yet.
- `scripts/tests/test_follower_import.py` — pinned-data import/application and failure-boundary tests.

## Tests asserting this spec
- `scripts/tests/test_follower_import.py` — source binary to SQLite round-trip, exact authentic spell rows, reapplication/preservation, invalid layout/source, read-only audit and database output rejection.

## Known gaps (current cycle)
- [ ] Authentic dungeon NPC→class/spec→combat spell assignments: none proved for the37 local Expedition Escort templates. All13 class/spec dungeon kits remain unproved.
- [ ] Queue/group NPC identity and instance entry; combat AI and class coverage.
- [ ] Promote selected TDB dungeon spawns/templates into runtime content through the content policy, then apply via the lead. Reference-table presence is not runtime spawn proof.
- [ ] Private live fighting proof and both-skin PNGs; no captures claimed.

## Out of scope
- Agent writes to canonical world.db or UDP5000 operations: lead-only.
- Guessed spells, NPC IDs, roles from unit_class, coefficients or garrison-mission ability IDs treated as combat spells: explicitly prohibited.
