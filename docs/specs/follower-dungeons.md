# Follower dungeons and NPC companions

Retail follower dungeons must group a solo player with four role-aware NPCs and complete an authored instance. User-approved workaround (2026-10-11): no authored NPC combat kits exist locally; followers use their labelled spec's real player spells through the existing executor. Spell coefficients, templates and displays remain authored. This is a bridge, not an NPC-kit authenticity claim; retire it when sourced NPC kits become available. [Local data audit and lead-only import](../wiki/investigations/follower-data.md).

## What it must do

### Reference preparation
- [x] Decode the pinned Retail LFGDungeons layout; retain follower difficulty entries, authored role counts, map IDs, conditions, flags and content tuning. Ruby Life Pools2436 must resolve to map2521 and a1/1/3 composition.
- [x] Preserve TDB creature-template spell assignments, indexes and VerifiedBuild without inventing or substituting spells.
- [x] Generate re-applicable SQL replacing only the two prepared reference tables; reject missing spell data or a wrong DB2 layout before publishing SQL.
- [x] Audit canonical world.db read-only; distinguish reference spawns from runtime content and report absent escort spell assignments.
- [x] Refuse a CLI output/report destination equal to an input or the audited database.

### Gameplay (user requirement; not implemented)
- [ ] Solo follower queue creates a five-member party, fills tank/healer/DPS roles, enters an instance with canonical authored spawns, follows/regroups and finishes the dungeon.
- [ ] Tanks manage threat/positioning; healers prioritize injured party members; DPS use bounded spec-spell priorities. All roles reuse server spell execution and navigation.
- [ ] Cover the eleven locally labelled followers: Garrick Protection Paladin, Breka Protection Warrior, Crenna Restoration Druid, Henry Holy Priest, Meredy Fire Mage, Shuja Elemental Shaman, Austin Beast Mastery Hunter, Kybel Devastation Evoker, Bo Windwalker Monk, Lantha Demonology Warlock and Ralia Feral Druid. Rogue, Death Knight and Demon Hunter remain gaps: no local follower identity exists.
- [ ] Prove the party fighting on private UDP5750, with inspected Modern and Forever PNGs. Forever remains an art skin of the same behavior.

## How it works
- [Source audit, importer and runtime boundary](../wiki/investigations/follower-data.md)

## Implementation inventory
- `scripts/follower_import.py` — standalone SQL preparation and optional read-only audit; no runtime consumer yet.
- `scripts/tests/test_follower_import.py` — pinned-data import/application and failure-boundary tests.

## Tests asserting this spec
- `scripts/tests/test_follower_import.py` — source binary to SQLite round-trip, exact authentic spell rows, reapplication/preservation, invalid layout/source, read-only audit and database output rejection.

## Known gaps (current cycle)
- [ ] Authored NPC combat kits are absent. The approved player-spec bridge does not establish full player rotation, pet, interrupt, defensive or ground-avoidance parity.
- [ ] Queue/group NPC identity and instance entry; combat AI and class coverage.
- [ ] Promote selected TDB dungeon spawns/templates into runtime content through the content policy, then apply via the lead. Reference-table presence is not runtime spawn proof.
- [ ] Private live fighting proof and both-skin PNGs; no captures claimed.

## Out of scope
- Agent writes to canonical world.db or UDP5000 operations: lead-only.
- Invented spell IDs/coefficients, roles inferred from creature unit_class and garrison-mission abilities treated as combat spells remain prohibited. Player-spec assignments are permitted solely by the approved workaround.
