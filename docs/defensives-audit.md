# Class defensives audit

Verified: 2026-10-09. Audit base: `5b04aa931720d441aaea1f99d3c11dc2a69bba40`; reproduction branch tip: `41fe1be7c`. Worktree: `/home/osso/.worktrees/game-server-rest`, branch `defensives`. Fixes are committed on this branch; final combined gate is pending. No merge, runtime or deployment changes.

## Results

All 13 supported classes' candidates exist in local `data/world.db` and load through `SpellStore`. Catalog presence does not prove learning, ownership or complete ability behavior. Evidence: [auras/defensives_tests.rs](../crates/server/src/auras/defensives_tests.rs). Test suffixes below omit `defensives_`; reduction tests exercise incoming fire and actual melee.

| Class | Spell ID / name | Implemented defensive behavior | Evidence suffix |
|---|---|---|---|
| Warrior | 871 Shield Wall | 100 → 60 fire/melee | `warrior_shield_wall_reduces_fire_40_percent` |
| Paladin | 642 Divine Shield | Fire/melee immunity; `Immune` log | `paladin_divine_shield_blocks_fire`, `paladin_divine_shield_blocks_melee` |
| Hunter | 186265 Aspect of the Turtle | 100 → 70 fire/melee; other mechanics remain incomplete | `hunter_turtle_reduces_fire_30_percent` |
| Rogue | 5277 Evasion | +200 unscaled catalog dodge points; guaranteed ordinary dodge, unchanged fire damage; dodge removed with aura | `rogue_evasion_increases_dodge` |
| Priest | 17 Power Word: Shield | Fire/melee consume shield; overflow damages health; depletion removes aura; full melee absorb logs `Absorb` | `priest_power_word_shield_absorbs_and_is_consumed`, `priest_power_word_shield_absorbs_melee` |
| Death Knight | 48792 Icebound Fortitude | 100 → 70 fire/melee | `death_knight_icebound_fortitude_reduces_fire_30_percent` |
| Shaman | 108271 Astral Shift | 100 → 60 fire/melee | `shaman_astral_shift_reduces_fire_40_percent` |
| Mage | 45438 Ice Block | Fire/melee immunity; `Immune` log | `mage_ice_block_blocks_fire`, `mage_ice_block_blocks_melee` |
| Warlock | 104773 Unending Resolve | 100 → 75 fire/melee; interrupt/silence protection not proven | `warlock_unending_resolve_reduces_fire_25_percent` |
| Monk | 115203 Fortifying Brew → 120954 | 100 → 80 fire/melee; fixed +20% of pre-buff max health added to current/max and removed with buff | `monk_fortifying_brew_reduces_fire_20_percent`, `monk_brew_health_survives_stat_updates` |
| Druid | 22812 Barkskin | 100 → 80 fire/melee | `druid_barkskin_reduces_fire_20_percent` |
| Demon Hunter | 198589 Blur → 212800 | 100 → 75 fire/melee; dodge/deflection remain incomplete | `demon_hunter_blur_trigger_reduces_fire_25_percent` |
| Evoker | 363916 Obsidian Scales | 100 → 70 fire/melee; interrupt/silence protection not proven | `evoker_obsidian_scales_reduces_fire_30_percent` |

All five original gap tests are enabled. Combined focused and full-crate validation is pending; the table describes implementation, not complete Retail parity.

## Reproduced causes and fixes

- **Divine Shield / Ice Block melee immunity:** melee previously applied 100 health loss without school immunity. [combat.rs](../crates/server/src/combat.rs) now shares `auras::immune_to_school` with the spell `is_immune` path, prevents health loss and logs `Miss(Immune)`. Both individual RED runs observed 100 instead of 0; combined immunity GREEN passed both tests.
- **Power Word: Shield melee absorb:** melee previously applied 100 health loss without consuming capacity. [auras/mod.rs](../crates/server/src/auras/mod.rs) extracts `consume_absorbs` and `finish_absorbs` from `absorb_damage`; melee uses the same logic. Capacity changes immediately, before subsequent swings; depleted aura removal and meter recording are queued through existing Commands. `ParamSet` releases attacker aura reads before target aura mutation. Tests check full `Absorb`, remaining capacity, partial damage/logging and depletion. RED observed 100 instead of 0.
- **Evasion:** aura 49 was unhandled and player defense used rating only. [handlers.rs](../crates/server/src/auras/handlers.rs), [stat_mods.rs](../crates/server/src/auras/stat_mods.rs) and [stats.rs](../crates/server/src/stats.rs) now add MOD_DODGE_PERCENT outside rating diminishing returns. RED observed delta 0 instead of 200; initial GREEN observed +200. The extended test also checks actual dodge/logging and aura removal.
- **Fortifying Brew:** cast 115203 contains DUMMY effects 0/1 (+20/-20), not defensive auras. [spell_cast/scripts.rs](../crates/server/src/spell_cast/scripts.rs) now applies catalog buff 120954's health and damage effects with custom amounts. Aura 34 adds/removes a fixed health bonus from both current and maximum health; stats retain that fixed bonus on recomputation. A wounded 700/1000 unit gains 200 health, takes 80 fire and 80 melee, then loses the same 200 bonus when the buff ends. Removal cannot kill a living unit. RED observed 100 instead of 80. Stagger, talent/spec variants and cosmetic effects are excluded.

### Evasion amount and sources

[TrinityCore Player::UpdateDodgePercentage](https://github.com/TrinityCore/TrinityCore/blob/master/src/server/game/Entities/Unit/StatSystem.cpp) adds `GetTotalAuraModifier(SPELL_AURA_MOD_DODGE_PERCENT)` to nondiminishing dodge. There is no division by two. Local 5277 effect 0 has BasePointsF **200**, Coefficient **0**, Variance **0**, RealPointsPerLevel **0**, PvP multiplier **1**. The loader reads BasePointsF directly; the raw +200 test expectation is retained, not silently changed to nominal Retail +100. Both exhaust the ordinary melee hit table; this is not a claim of a meaningful 200% probability.

[SimulationCraft Retail monk buffs::fortifying_brew_t](https://github.com/simulationcraft/simc/blob/midnight/engine/class_modules/monk/sc_monk.cpp) snapshots max health × cast health percentage, adds equal current/max gains and removes the same gains on expiry. [TrinityCore AuraEffect::HandleAuraModIncreaseHealth](https://github.com/TrinityCore/TrinityCore/blob/master/src/server/game/Spells/Auras/SpellAuraEffects.cpp) supplies aura 34's equal current/max and nonlethal-removal semantics. Upstream TrinityCore master currently has no Brew script; the DUMMY-to-120954 bridge is present in [TrinityCore-derived RoleplayCore spell_monk_fortifying_brew](https://github.com/KamiliaBlow/RoleplayCore/blob/master/src/server/scripts/Spells/spell_monk.cpp). Sources are cited in implementation comments. Local parent +20/-20 values drive this bounded implementation; other expansions/specs are not substituted.

## Blur and Turtle still need work

- **Blur:** generic aura 49 handling is now available, but catalog 212800's dodge amount is **0**, so this does not implement a nonzero Blur dodge bonus. Aura 288 deflection also lacks a handler. A spell-specific amount/source reconciliation and deflection behavior are needed before claiming complete Blur.
- **Turtle:** attacker melee/ranged/spell hit modifiers 184/185/186 (-200 catalog points), deflection 288 (+100), and mechanic immunity 77 still lack handlers. These are source-inspection gaps, not separate end-to-end failures reproduced here. Aura 28 reflection **does** have a handler (the original audit incorrectly grouped it with unhandled types), but Turtle's raw reflection amount is 0; it does not establish functioning Turtle deflection. Attack suppression and directional behavior are not proven. No new systems for these ancillary mechanics were added.

## Proof boundary

The real-data Bevy/Lightyear [harness](../crates/server/src/spell_cast/test_harness.rs) authenticates HostClient loopback casts through known-spell validation, costs, resolution, AuraPlugin and CombatPlugin. Spells are granted as fixture known spells; normal progression is not proven. No DB/catalog rows or defensive aura state are injected by tests. Brew customization happens in the production spell script.

Incoming fire uses Fireball 133 metadata and deterministic pre-mitigation damage through production `deal_damage`; it does not prove incoming spell hit/deflect/reflect selection. Melee uses actual scheduled NPC AutoAttack, a fixed 100-point weapon, zero AP/crit and +100 hit to remove random misses. Victims have zero avoidance except the computed Evasion fixture. An unprotected control takes 100 on both paths. Brew has separate wounded-health and StatsPlugin apply/remove proofs; explicit Expired removal exercises cleanup, not timer expiry.

Client presentation, live realm networking, all schools, natural expiry timers, dispels, secondary mechanics and talent/spec variants remain outside this proof. No other worktree or runtime was changed.

## Verification ledger

All Cargo commands use `/home/osso/.worktrees/build-lock.sh` and the project's `agent-run defensives` wrapper. Logs live in `target/`; none are committed.

- Original reproduction: 17 focused tests, 12 passed / 5 failed, subsequently ignored at `41fe1be7c`. Previous complete crate: 1557 passed / 0 failed / 65 ignored (60 pre-existing plus five gaps).
- RED, each individual enabled gap test: Paladin, Mage and Shield each observed 100 instead of 0; Evasion delta 0 instead of 200; Brew 100 instead of 80. Logs: `defensives-{paladin,mage,absorb,evasion,brew}-red.log`.
- Immunity GREEN at `ab9619b`: 2 passed / 0 failed / 0 ignored. `defensives-immunity-green.log`.
- Initial Evasion GREEN at `3d8f555`: 1 passed / 0 failed / 0 ignored. `defensives-evasion-green.log`. Later test assertions extend this proof and await the combined gate.
- Absorb/Brew GREEN and combined 18-test focused gate: pending. Intermediate compile failures (missing DefenseInputs field and wrong aura API name) were fixed in subsequent commits; they are not passing proof.
- Full server crate, cargo check and formatting gates: pending. Independent Claude verification is unavailable: authentication expired; no independent-model verification claim.
