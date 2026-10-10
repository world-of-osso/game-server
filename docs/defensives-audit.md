# Class defensives audit

Blur/Turtle follow-up: branch `blur-turtle`, base `c5432a4`. Batched RED: 18 passed / 6 failed; initial GREEN: 24 passed / 0 failed. See follow-up ledger below. Blur's catalog dodge amount remains zero, so nonzero Retail Blur dodge is not claimed. Final suite pending.

Updated: 2026-10-09. Original audit base: `5b04aa931720d441aaea1f99d3c11dc2a69bba40`; reproduction tip: `41fe1be7c`. The five earlier fixes passed focused/full tests at `e86c038` and landed in master `c5432a4`. Follow-up worktree: `/home/osso/.worktrees/game-server-rest`, branch `blur-turtle`; no runtime or deployment changes.

## Results

All 13 supported classes' candidates exist in local `data/world.db` and load through `SpellStore`. Catalog presence does not prove learning, ownership or complete ability behavior. Evidence: [auras/defensives_tests.rs](../crates/server/src/auras/defensives_tests.rs). Test suffixes below omit `defensives_`; reduction tests exercise incoming fire and actual melee. Table amounts describe normal 100-point swings; melee assertions also validate the doubled damage of reported NPC critical hits.

| Class | Spell ID / name | Implemented defensive behavior | Evidence suffix |
|---|---|---|---|
| Warrior | 871 Shield Wall | 100 → 60 fire/melee | `warrior_shield_wall_reduces_fire_40_percent` |
| Paladin | 642 Divine Shield | Fire/melee immunity; `Immune` log | `paladin_divine_shield_blocks_fire`, `paladin_divine_shield_blocks_melee` |
| Hunter | 186265 Aspect of the Turtle | 100 → 70 already-landed damage; new melee/ranged/magic attacks miss; silence immunity; 205769 pacifies attacks until parent expiry | `hunter_turtle_reduces_fire_30_percent`, `turtle_*` |
| Rogue | 5277 Evasion | +200 unscaled catalog dodge points; guaranteed ordinary dodge, unchanged fire damage; dodge removed with aura | `rogue_evasion_increases_dodge` |
| Priest | 17 Power Word: Shield | Fire/melee consume shield; overflow damages health; depletion removes aura; full melee absorb logs `Absorb` | `priest_power_word_shield_absorbs_and_is_consumed`, `priest_power_word_shield_absorbs_melee` |
| Death Knight | 48792 Icebound Fortitude | 100 → 70 fire/melee | `death_knight_icebound_fortitude_reduces_fire_30_percent` |
| Shaman | 108271 Astral Shift | 100 → 60 fire/melee | `shaman_astral_shift_reduces_fire_40_percent` |
| Mage | 45438 Ice Block | Fire/melee immunity; `Immune` log | `mage_ice_block_blocks_fire`, `mage_ice_block_blocks_melee` |
| Warlock | 104773 Unending Resolve | 100 → 75 fire/melee; interrupt/silence protection not proven | `warlock_unending_resolve_reduces_fire_25_percent` |
| Monk | 115203 Fortifying Brew → 120954 | 100 → 80 fire/melee; fixed +20% of pre-buff max health added to current/max and removed with buff | `monk_fortifying_brew_reduces_fire_20_percent`, `monk_brew_health_survives_stat_updates` |
| Druid | 22812 Barkskin | 100 → 80 fire/melee | `druid_barkskin_reduces_fire_20_percent` |
| Demon Hunter | 198589 Blur → 212800 | 100 → 75 fire/melee; data-driven dodge modifier and expiry, but catalog dodge amount is 0; aura 288 ignores direction, not deflection | `demon_hunter_blur_trigger_reduces_fire_25_percent`, `blur_uses_catalog_dodge_amount_and_expires` |
| Evoker | 363916 Obsidian Scales | 100 → 70 fire/melee; interrupt/silence protection not proven | `evoker_obsidian_scales_reduces_fire_30_percent` |

All five original gap tests are enabled and passing. Focused: **18 passed / 0 failed / 0 ignored**. Complete server crate: **1563 passed / 0 failed / 60 pre-existing ignored**. All 13 candidates demonstrate at least one defensive effect; this does not mean complete Retail ability parity.

## Reproduced causes and fixes

- **Divine Shield / Ice Block melee immunity:** melee previously applied 100 health loss without school immunity. [combat.rs](../crates/server/src/combat.rs) now shares `auras::immune_to_school` with the spell `is_immune` path, prevents health loss and logs `Miss(Immune)`. Both individual RED runs observed 100 instead of 0; combined immunity GREEN passed both tests.
- **Power Word: Shield melee absorb:** melee previously applied 100 health loss without consuming capacity. [auras/mod.rs](../crates/server/src/auras/mod.rs) extracts `consume_absorbs` and `finish_absorbs` from `absorb_damage`; melee uses the same logic. Capacity changes immediately, before subsequent swings; depleted aura removal and meter recording are queued through existing Commands. `ParamSet` releases attacker aura reads before target aura mutation. Tests check full `Absorb`, remaining capacity, partial damage/logging and depletion. RED observed 100 instead of 0.
- **Evasion:** aura 49 was unhandled and player defense used rating only. [handlers.rs](../crates/server/src/auras/handlers.rs), [stat_mods.rs](../crates/server/src/auras/stat_mods.rs) and [stats.rs](../crates/server/src/stats.rs) now add MOD_DODGE_PERCENT outside rating diminishing returns. RED observed delta 0 instead of 200; initial GREEN observed +200. The extended test also checks actual dodge/logging and aura removal.
- **Fortifying Brew:** cast 115203 contains DUMMY effects 0/1 (+20/-20), not defensive auras. [spell_cast/scripts.rs](../crates/server/src/spell_cast/scripts.rs) now applies catalog buff 120954's health and damage effects with custom amounts. Aura 34 adds/removes a fixed health bonus from both current and maximum health; stats retain that fixed bonus on recomputation. A wounded 700/1000 unit gains 200 health, takes 80 fire and 80 normal melee (160 on a critical), then loses the same 200 bonus when the buff ends. Removal cannot kill a living unit. RED observed 100 instead of 80. Stagger, talent/spec variants and cosmetic effects are excluded.

### Evasion amount and sources

[TrinityCore Player::UpdateDodgePercentage](https://github.com/TrinityCore/TrinityCore/blob/master/src/server/game/Entities/Unit/StatSystem.cpp) adds `GetTotalAuraModifier(SPELL_AURA_MOD_DODGE_PERCENT)` to nondiminishing dodge. There is no division by two. Local 5277 effect 0 has BasePointsF **200**, Coefficient **0**, Variance **0**, RealPointsPerLevel **0**, PvP multiplier **1**. The loader reads BasePointsF directly; the raw +200 test expectation is retained, not silently changed to nominal Retail +100. Both exhaust the ordinary melee hit table; this is not a claim of a meaningful 200% probability.

[SimulationCraft Retail monk buffs::fortifying_brew_t](https://github.com/simulationcraft/simc/blob/midnight/engine/class_modules/monk/sc_monk.cpp) snapshots max health × cast health percentage, adds equal current/max gains and removes the same gains on expiry. [TrinityCore AuraEffect::HandleAuraModIncreaseHealth](https://github.com/TrinityCore/TrinityCore/blob/master/src/server/game/Spells/Auras/SpellAuraEffects.cpp) supplies aura 34's equal current/max and nonlethal-removal semantics. Upstream TrinityCore master currently has no Brew script; the DUMMY-to-120954 bridge is present in [TrinityCore-derived RoleplayCore spell_monk_fortifying_brew](https://github.com/KamiliaBlow/RoleplayCore/blob/master/src/server/scripts/Spells/spell_monk.cpp). Sources are cited in implementation comments. Local parent +20/-20 values drive this bounded implementation; other expansions/specs are not substituted.

## Blur/Turtle follow-up

Pinned [TrinityCore aura definitions](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Spells/Auras/SpellAuraDefines.h) correct the original audit: **287 = DEFLECT_SPELLS; 288 = IGNORE_HIT_DIRECTION**. A +100 amount on 288 is not a deflection probability. Neither Blur nor Turtle has aura 287 in this catalog; Turtle prevents new attacks through -200 attacker hit modifiers, not an invented school immunity or deflection chance.

- **General incoming defenses:** aura 184 modifies melee miss selection; 185 modifies ranged-spell misses; 186 modifies matching-school magic misses. Aura 77 prevents incoming matching spell/effect mechanics. Aura 287 deflects magic/ranged spells from the front, or any direction under 288, unless controlled. Existing NoAvoidance/NoAttackMiss/NoImmunities attributes are respected in their applicable paths. Aura 47 adds nondiminishing player parry with the existing weapon/parry-capability requirements. Damage reduction remains aura 87; already-landed and periodic damage bypasses new-hit avoidance. References: [Unit melee hit selection](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Unit/Unit.cpp), [magic hit selection](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Object/Object.cpp).
- **Turtle pacify:** pinned [spell_hun_aspect_of_the_turtle](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/scripts/Spells/spell_hunter.cpp) applies child 205769 on parent application and removes it with the parent. Its catalog aura 25 blocks PreventionType PACIFY abilities and, generally, auto-attacks. No defensive amounts are hard-coded. Natural timer expiry removes miss modifiers, reduction and the child pacify.
- **Blur data boundary:** 212800 effect 1 remains aura 49 with BasePointsF **0**, no coefficient or per-level term. Generic dodge handling already works (Evasion's real-data guaranteed-dodge test). The follow-up proves Blur consumes the catalog amount and removes it on timer expiry; it does **not** claim a nonzero Retail Blur dodge bonus. No replacement amount or catalog row is fabricated. Auto-attack front/back avoidance remains the existing engine boundary; 288's directional exemption is consumed by the new spell-deflection path.
- **Additional real-data fixtures:** Deterrence 19263 demonstrates aura 287 deflection without reflecting damage to the caster; Die by the Sword 118038, Parry 3127 and equipped Worn Shortsword 25 demonstrate aura 47 preventing actual NPC melee swings. Both end through natural timers. Turtle 186265 blocks Fireball 133, Auto Shot 75 and silence 1330, suppresses scheduled player swings and Steady Shot 56641, then permits attacks after expiry. These are combat/runtime assertions, not source-shape tests.

### Follow-up ledger

- RED: `target/blur-turtle-red2-tests.log`, base c5432a4 plus behavioral tests: **18 passed / 6 failed / 0 ignored**, exit 101. Failures: Turtle melee, magic/ranged miss logs, pacify, silence immunity, generic deflection, generic parry. An earlier RED attempt had one test enum-construction compile error, corrected before this behavioral run.
- Initial GREEN at `350924f`: `target/blur-turtle-green-tests.log`, **24 passed / 0 failed / 0 ignored**, exit 0. One unused import warning was subsequently removed; parry expiry assertion now checks restored baseline chance rather than a probabilistic single swing. The no-stats attacker path retains its prior avoidance behavior.
- Final full server suite: pending, once after follow-up fixes, skipping `query_300k_listings_under_200ms` as requested. Logs remain untracked under `target/`.
- Independent artifact verifier unavailable: Claude OAuth expired; no independent-model verification claim. Main-thread readability inspection extracted effect filtering/miss chance, kept chance numbers data-driven, and introduced no suppressions or dependencies.

## Proof boundary

The real-data Bevy/Lightyear [harness](../crates/server/src/spell_cast/test_harness.rs) authenticates HostClient loopback casts through known-spell validation, costs, resolution, AuraPlugin and CombatPlugin. Spells are granted as fixture known spells; normal progression is not proven. No DB/catalog rows or defensive aura state are injected by tests. Brew customization happens in the production spell script.

Incoming fire uses Fireball 133 metadata and deterministic pre-mitigation damage through production `deal_damage`; it does not prove incoming spell hit/deflect/reflect selection. Melee uses actual scheduled NPC AutoAttack, a fixed 100-point weapon, zero AP and +100 hit to remove random misses. NPCs retain their intrinsic 5% crit chance: assertions use the delivered crit flag to validate normal and doubled critical damage, shield consumption and overflow without changing NPC gameplay. The original audit incorrectly claimed zero crit. Victims have zero avoidance except computed Evasion. An unprotected control takes 100 fire and 100/200 melee. Shield capacity is fractional (4.638 × 100 healing power), while incoming spell damage rounds; partial-absorb assertions use measured remaining capacity and the existing integer log contract. Brew has separate wounded-health and StatsPlugin apply/remove proofs; explicit Expired removal exercises cleanup, not timer expiry.

Client presentation, live realm networking, all schools, dispels, secondary mechanics and talent/spec variants remain outside this proof. Follow-up tests add natural timer-expiry proof for Blur, Turtle, Deterrence and Die by the Sword. No other worktree or runtime was changed.

## Verification ledger

All Cargo commands use `/home/osso/.worktrees/build-lock.sh` and the project's `agent-run defensives` wrapper. Logs live in `target/`; none are committed.

- Original reproduction: 17 focused tests, 12 passed / 5 failed, subsequently ignored at `41fe1be7c`. Previous complete crate: 1557 passed / 0 failed / 65 ignored (60 pre-existing plus five gaps).
- RED, each individual enabled gap test: Paladin, Mage and Shield each observed 100 instead of 0; Evasion delta 0 instead of 200; Brew 100 instead of 80. Logs: `defensives-{paladin,mage,absorb,evasion,brew}-red.log`.
- Immunity GREEN at `ab9619b`: 2 passed / 0 failed / 0 ignored. `defensives-immunity-green.log`.
- Initial Evasion GREEN at `3d8f555`: 1 passed / 0 failed / 0 ignored. `defensives-evasion-green.log`.
- Expanded event RED: 15 passed / 3 failed, exposing legacy `MeleeDamage` events for immunity/full absorption; fixed by shared outcome classification. `defensives-event-red.log`. Subsequent 17/1 focused failures exposed test assumptions about fractional absorb capacity and NPC crits; corrected from catalog and production hit-table evidence, without changing NPC gameplay. Logs: `defensives-partial-log-red.log`, `defensives-npc-crit-red.log`. Intermediate missing-field/API compile errors were corrected; none are passing proof.
- Final `cargo test -p server --bin game-server defensives_` at **`e86c0385d4112ed720ddf3cef33478454aa241c3`**: **18 passed / 0 failed / 0 ignored / 1580 filtered**, exit 0 (3.26 s). `defensives-fixed-focused.log`.
- Final `cargo test -p server -- --test-threads=4` at the same revision: **1563 passed / 0 failed / 60 ignored**, exit 0. Main: 1538 passed / 60 ignored (144.14 s); remaining binaries: 18, 4 and 3 passed. All ignores predate these fixes. `defensives-fixed-crate.log`. Full suite ran once after the focused gate passed.
- `cargo check -p server`: exit 0, no warnings. `defensives-fixed-check.log`. Changed-file `rustfmt --edition 2024 --config skip_children=true --check`: exit 0. `defensives-fixed-changed-fmt.log`.
- Whole-workspace `cargo fmt --all -- --check`: exit 1 only for pre-existing ghoststate module ordering in `crates/server/src/networking_tests.rs`; that file is unchanged from `41fe1be7c`. `defensives-fixed-fmt.log`. Unrelated formatting was not changed.
- Exact argv, revision and exit codes: `target/defensives-final-ledger.json`; persistent one-shot runner used the same build lock and agent wrapper after interrupted tool calls left earlier waiting processes orphaned. No tests from those orphaned gate attempts had started. Later documentation-only commits do not invalidate this source proof.
- Independent Claude verification unavailable: authentication expired; no independent-model verification claim. Readability inspection retained the existing absorb iteration while separating world-side cleanup; no suppressions or new dependency added.
