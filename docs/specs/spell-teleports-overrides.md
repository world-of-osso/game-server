# Spell teleports and overrides

Data-driven server displacement and matrix aura overrides in `crates/server/src/spell_cast` and `auras`. [Implementation](../wiki/systems/spell-casting.md).

## What it must do

- [x] Blink 1953 moves 20 yards forward from target55's radius and snaps to ground.
- [x] Blink stops before real map0 wall geometry and bumps the movement teleport epoch.
- [x] Hearthstone 8690 returns to a persisted homebind, including NewWorld/WorldPortAck across maps.
- [x] New characters persist actual creation binds, preserved after logout-position changes and reopening redb.
- [ ] Existing characters without a stored bind must fail explicitly; no inferred migration location.
- [x] Target17 teleports consume authored spell_target_position rows, never infer destination coordinates from spell IDs. Import the established TDB1210.26091 source as tdb_spell_target_position, preserving OrderIndex and nullable Orientation. Target17 consumes point0; NULL facing uses Retail SpellEffect facing (degrees above 2π convert to radians). Mage teleport3567 must reach the authored Orgrimmar row through NewWorld/WorldPortAck.
- [x] Harpoon190925's child186260 and Feral Lunge196884 move the caster, not the enemy target. Infernal Strike189110 accepts a finite, in-range explicit ground point.
- [x] Target148 moves the current destination toward the caster by the active slot's native radius before the next selector, including destination-area target selection. Actual434712 movement in Normal1/Follower205 and native mixed selectors are covered by eight enabled tests. Use existing baked/live direct collision and creature ground; no spell-ID offset or fabricated terrain. `SPELL_DEST_DETOUR_RAYCAST_ABSENT` (TC Object.cpp2853–2866) remains unproved; `SPELL_DEST_FORCE_LOCATION_MAX_HEIGHT_ABSENT` (Spell.cpp9286–9292, Object.cpp2802–2826) must reject native force-location spells explicitly.
- [x] Aura332 selects its data-specified replacement spell while active and restores the base after removal/expiry.
- [x] Aura312 exposes its animation replacement set through aura replication; client consumption is a separate task.

## How it works

- [Spell casting](../wiki/systems/spell-casting.md)
- [Map transfer](../wiki/systems/zone-architecture.md)

## Implementation inventory

- `spell_cast/teleport.rs`: LOS displacement and destination validation.
- `spell_cast/teleport_positions.rs`: authored destination loader.
- `spell_cast/homebind.rs`: separate persisted homebind ABI and login initialization.
- `spell_cast/effects.rs`, `targets.rs`: dispatch and target selection.
- `auth_character.rs`, `persistence.rs`: homebind login/table lifecycle.
- `auras/overrides.rs`, `auras/mod.rs`: live spell substitution and aura wire values.
- `scripts/tdb_schema.sql`, `scripts/import-azerothcore.sh`: prefixed authored destination import.

## Tests asserting this spec

- `crates/server/src/spell_cast/teleport_tests.rs`
- `crates/server/src/maps/tests.rs`: Hearthstone cross-map handshake.
- `crates/server/src/auras/override_tests.rs`: substitution, removal and expiry.
- `crates/server/src/spell_cast/override_queue_tests.rs`: re-resolve on queued release.
- Companion `shared-protocol/src/protocol_spell_tests.rs`: aura overrides and ground-point wire round trips.

## Known gaps (current cycle)

- Ordered nearby-destination selection (target106) is outside this fixed-destination import; all ordered rows remain stored.
- [ ] Client must consume AuraView.overrides and supply SpellCastIntent.destination for ground-point leaps. Companion shared-protocol branch required; existing clients need protocol literal initializers updated.

## Out of scope

Client animation/actionbar consumption, innkeeper UI, unrelated dummy spell scripts and full rotations.
