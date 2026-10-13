# Follower dungeons and NPC companions

Retail follower dungeons must group a solo player with four role-aware NPCs and complete an authored instance. User-approved workaround (2026-10-11): no authored NPC combat kits exist locally; followers use their labelled spec's real player spells through the existing executor. Spell coefficients, templates and displays remain authored. The bridge initializes native class/spec/level stats, supplies the class-script `Level` alongside replicated `UnitLevel`, and keeps replicated health/mana consistent with those stats; it does not claim authenticated Retail NPC gear or stat-budget parity. This is a bridge, not an NPC-kit authenticity claim; retire it when sourced NPC kits become available. [Local data audit and lead-only import](../wiki/investigations/follower-data.md).

Additional user-approved workaround (2026-10-11): follower difficulty205 runs reuse the same dungeon's Normal-difficulty creature/game-object population and authored Normal entrance. Queue, party and instance identity remain205; creatures retain205 template/loot selection. No synthetic coordinates, spawn-tag widening or automatic alternate population path. Retire this bridge when local205 populations and authored entrances are complete. Ruby Life Pools2436/map2521 is the live candidate:304 Normal creature rows, entrance100030 and first boss Melidrussa188252.

## What it must do

### Reference preparation
- [x] Decode the pinned Retail LFGDungeons layout; retain follower difficulty entries, authored role counts, map IDs, conditions, flags and content tuning. Ruby Life Pools2436 must resolve to map2521 and a1/1/3 composition.
- [x] Preserve TDB creature-template spell assignments, indexes and VerifiedBuild without inventing or substituting spells.
- [x] Generate re-applicable SQL replacing only the two prepared reference tables; reject missing spell data or a wrong DB2 layout before publishing SQL.
- [x] Audit canonical world.db read-only; distinguish reference spawns from runtime content and report absent escort spell assignments.
- [x] Refuse a CLI output/report destination equal to an input or the audited database.

Spell458324 is not assigned by the follower templates' TDB/runtime addon data. Do not bootstrap it from its name; this hides the companions and breaks retaliation. Apply native spec passives and only the explicitly approved player-spec bridge spells.

### Gameplay (acceptance remains open until behavioral/live proof)
- Group Finder requests the server's prepared difficulty205 catalog on `LfgChannel`; only entries with authored entrances and prepared population appear. Catalog publication is not queue acceptance. IDs, names and maps come from the existing imported catalog, never a client-side list.
- [ ] Solo follower queue creates a five-member party, fills tank/healer/DPS roles, enters an instance with canonical authored spawns, follows/regroups and finishes the dungeon.
- [ ] Tanks manage threat/positioning; healers prioritize injured party members; DPS use bounded spec-spell priorities. All roles reuse server spell execution and navigation. Follower follow/combat movement requires an authored navmesh; missing/blocked bakes never become direct movement. Repeated idle AI ticks preserve an in-progress follow route until its actual follow point is reached. Companions are player-affected combat units, not persisted characters; their real spell damage credits the authenticated owner/group for kill rewards.
- [ ] Cover the eleven locally labelled followers: Garrick Protection Paladin, Breka Protection Warrior, Crenna Restoration Druid, Henry Holy Priest, Meredy Fire Mage, Shuja Elemental Shaman, Austin Beast Mastery Hunter, Kybel Devastation Evoker, Bo Windwalker Monk, Lantha Demonology Warlock and Ralia Feral Druid. Rogue, Death Knight and Demon Hunter remain gaps: no local follower identity exists.
- [x] Prove the party fighting on private UDP5750, with inspected Modern and Forever PNGs. Forever remains an art skin of the same behavior.

## How it works
- [Source audit, importer and runtime boundary](../wiki/investigations/follower-data.md)

## Implementation inventory
- `scripts/follower_import.py` — standalone SQL preparation and optional read-only audit.
- `scripts/follower_profiles.py` — verified eleven-follower roster, authentic template/model promotion, approved core player-spell priorities.
- `crates/server/src/followers/` — catalog/admission, companion group identity, owner following, role priorities and ordinary spell execution; bounded behavioral proofs pass, end-to-end acceptance remains open.
- `scripts/tests/test_follower_import.py` — pinned-data import/application and failure-boundary tests.

## Tests asserting this spec
- `scripts/tests/test_follower_import.py` — source binary to SQLite round-trip, exact authentic spell rows, reapplication/preservation, invalid layout/source, read-only audit and database output rejection.
- `scripts/tests/test_follower_profiles.py` — eleven verified profiles, source model preservation, repeated SQL application and database-overwrite refusal.
- `crates/server/src/followers/{tests,catalog_tests}.rs` — role composition, real damage/heal/taunt/cost/native initialization, refusal of the unassigned458324 bootstrap, owner kill credit and retaliation, missing-source refusal and exact Normal population admission-data gating and205 creature/game-object copy spawning.
- `crates/server/src/followers/profile_behavior_tests.rs` — every locally authenticated profile initializes native class resources and executes actual damage/healing/taunt priorities through the real spell system; full metadata preserves triggered/form dependencies. This is basic-role coverage, not full rotations/pets/interrupts/defensives.
- `crates/server/src/followers/hazard_tests.rs` — actual hostile Consecration during combat on real Stockade navigation, checking collision-clear movement out of its native radius. The bridge evaluates eight supported escape candidates through complete baked routes, not teleportation or direct movement; it holds chase/follow when their target remains in a hostile bound field or their complete baked approach crosses one. Bound `GroundPulses` and `SpellAreaTrigger` actions are covered; lifecycle-only unbound or separately scripted shapes are not inferred.
- `crates/server/src/followers/pet_tests.rs` — Lantha's real Summon Felguard30146 uses authored creature17252 with existing controlled-pet stats/spawn rules. Demonology prepares that summon before damage; failed preparation holds and reports the ordinary cast failure rather than substituting another creature or petless rotation. NPC pet memory has no character ID and writes no persistent character/stable rows; companions are not disguised as `Player` entities. The existing ownerless-pet cleanup removes the pet when its runtime companion owner is removed. Companion-pet combat events reach the authenticated player through pet→companion→player ownership rather than stopping at the NPC. Companion attack orders use ordinary PetControl commands; bound-field danger holds those orders. Summoning and attack fixtures do not establish pet interrupts/navmesh or authenticated inherited-stat-budget parity. Hunter has no prepared authored pet selection and must not receive an invented creature.
- `crates/server/src/group/follower_tests.rs` — real loopback companion online roster and health changes.
- `crates/server/src/followers/utility_tests.rs` — actual enemy Fireball casts, selected real spec interrupt combat logs and refusal to waste Counterspell on a non-interruptible cast. Skull Bash106839 dispatches authored charge221514 and interrupt93985 per TrinityCore a352b1fa `spell_druid.cpp:2037–2053`; the interrupt log belongs to93985. A ready kick stops the follower's current cast only after ordinary range/LOS/resource/aura/school/cooldown validation; failed readiness preserves that cast and leaves normal damage priorities available. These checks do not establish charge movement, full rotations, pet utilities or hazard avoidance.
- `crates/server/src/followers/defensive_tests.rs` — wounded and actively targeted companions cast their class's real defensive; subsequent ordinary spell damage measures native mitigation or Desperate Prayer healing. The approved bridge chooses this condition without an invented health-percentage cutoff; it is not an authenticated Retail NPC defensive policy.

## Known gaps (current cycle)
- [ ] Authored NPC combat kits are absent. The approved player-spec bridge does not establish full player rotation, pet, interrupt, defensive or ground-avoidance parity.
- [ ] Full-instance completion and full eleven-spec combat coverage remain unproved. Current both-skin live slice covers a Damage-role Mage, Garrick/Crenna/Shuja/Meredy, trash and the first Ruby boss; no wider certification.
- [x] User approved same-map Normal population/entrance reuse; no synthetic coordinates or mutation of authored spawn tags.
- [x] Lead confirmed canonical follower reference/profile SQL application (11 profiles,27 LFG rows). Agent live development remains on a private copy.
- [x] Private live first-boss proof and both-skin PNGs: difficulty205 admission, five correct roles, following, tank threat/boss swings, NPC healing, damage/cooldown casts, Juggernaut and Melidrussa kills, owner XP. [Proof and exact limitations](../wiki/investigations/follower-data.md#both-skin-first-boss-proof--2026-10-11).
- [ ] Physical walked route: native movement blocked near1434/-180; private admin reposition supplied boss approach. Do not claim navigation/full-dungeon completion.
- [ ] Authenticated first-boss loot: Melidrussa188252 has lootid0 at all local difficulties and zero source/runtime loot rows. Do not fabricate a drop table.

## Out of scope
- Agent writes to canonical world.db or UDP5000 operations: lead-only.
- Invented spell IDs/coefficients, roles inferred from creature unit_class and garrison-mission abilities treated as combat spells remain prohibited. Player-spec assignments are permitted solely by the approved workaround.
