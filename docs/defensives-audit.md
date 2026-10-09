# Class defensives audit

Verified: 2026-10-09. Base: `5b04aa931720d441aaea1f99d3c11dc2a69bba40` (fetched `origin/master`). Worktree: `/home/osso/.worktrees/game-server-rest`, branch `defensives`. No gameplay fixes or runtime/deployment changes.

## Results

All 13 classes in `crates/server/src/race_class.rs:13` are supported. Every candidate below exists in the local `data/world.db` (`spell_name` and DifficultyID-0 `spell_effect`) and loads through `SpellStore`. **Absent candidates: none.** This is catalog presence, not proof of learning, ownership, or every mechanic working.

Evidence tests are in [auras/defensives_tests.rs](../crates/server/src/auras/defensives_tests.rs). Names below omit the common `auras::defensives_tests::defensives_` prefix. Reduction tests assert **both** incoming fire and an actual melee swing, despite their `_fire_` names.

| Class | Spell ID / name | Catalog present? | Server effect proven / gap | Evidence test suffix |
|---|---|---|---|---|
| Warrior | 871 Shield Wall | Yes | 100 incoming → 60 fire and melee damage | `warrior_shield_wall_reduces_fire_40_percent` |
| Paladin | 642 Divine Shield | Yes | Fire immunity proven (0 damage + Immune log); **melee gap: takes 100** | `paladin_divine_shield_blocks_fire`; ignored `paladin_divine_shield_blocks_melee` |
| Hunter | 186265 Aspect of the Turtle | Yes | 100 → 70 fire and melee; **partial**, deflection not proven | `hunter_turtle_reduces_fire_30_percent` |
| Rogue | 5277 Evasion | Yes | **Gap:** computed dodge delta 0, expected +200 catalog points; fire still deals 100 | ignored `rogue_evasion_increases_dodge` |
| Priest | 17 Power Word: Shield | Yes | Fire absorbed, capacity decremented, depleted aura removed, overflow damages health; **melee gap: takes 100** | `priest_power_word_shield_absorbs_and_is_consumed`; ignored `priest_power_word_shield_absorbs_melee` |
| Death Knight | 48792 Icebound Fortitude | Yes | 100 → 70 fire and melee | `death_knight_icebound_fortitude_reduces_fire_30_percent` |
| Shaman | 108271 Astral Shift | Yes | 100 → 60 fire and melee | `shaman_astral_shift_reduces_fire_40_percent` |
| Mage | 45438 Ice Block | Yes | Fire immunity proven (0 damage + Immune log); **melee gap: takes 100** | `mage_ice_block_blocks_fire`; ignored `mage_ice_block_blocks_melee` |
| Warlock | 104773 Unending Resolve | Yes | 100 → 75 fire and melee; interrupt/silence protection not proven | `warlock_unending_resolve_reduces_fire_25_percent` |
| Monk | 115203 Fortifying Brew | Yes | **Gap:** successful cast, 100 fire damage instead of 80; health bonus not proven | ignored `monk_fortifying_brew_reduces_fire_20_percent` |
| Druid | 22812 Barkskin | Yes | 100 → 80 fire and melee | `druid_barkskin_reduces_fire_20_percent` |
| Demon Hunter | 198589 Blur → 212800 | Yes, both | Triggered aura reduces 100 → 75 fire and melee; dodge not proven | `demon_hunter_blur_trigger_reduces_fire_25_percent` |
| Evoker | 363916 Obsidian Scales | Yes | 100 → 70 fire and melee; interrupt/silence protection not proven | `evoker_obsidian_scales_reduces_fire_30_percent` |

**Summary:** 11/13 representative spells demonstrate at least one defensive effect. This does **not** mean 11 complete abilities. Five executable expectations reproduce failures and remain ignored with `defensive gap:` reasons; Rogue and Monk have no defensive effect proven here.

## Reproduced causes

- Divine Shield / Ice Block melee immunity and Power Word: Shield melee absorption: [combat.rs:844–853](../crates/server/src/combat.rs#L844) converts swing damage directly into health loss without `is_immune` or `absorb_damage`. Spell damage invokes both at [spell_cast/effects.rs:557–574](../crates/server/src/spell_cast/effects.rs#L557). All three failing melee tests observed 100 health loss, versus expected 0.
- Evasion: catalog APPLY_AURA 49 (+200); [auras/handlers.rs:197](../crates/server/src/auras/handlers.rs#L197) falls through to unhandled. [stats.rs:430](../crates/server/src/stats.rs#L430) computes dodge from ratings, with no aura flat dodge term. The test runs `StatsPlugin` and checks the before/after computed stat, not merely aura presence.
- Fortifying Brew: catalog effects are DUMMY 3 with +20 and -20 points, not a generic damage-reduction aura. [spell_cast/scripts.rs:36–45](../crates/server/src/spell_cast/scripts.rs#L36) has no hook for 115203; [spell_cast/effects.rs:113](../crates/server/src/spell_cast/effects.rs#L113) has no DUMMY handler. Successful cast deals no defensive effect: observed 100 incoming fire loss versus expected 80.

Ancillary limits from source inspection: Turtle's deflection/direction/attack-suppression aura IDs 184/185/186/288/28, Evasion/Blur dodge 49, and mechanism-immunity 77 do not have registry handlers (`auras/handlers.rs:125–197`). These are **code-inspection gaps, not separate end-to-end failures proven by this audit**. Blur's loaded 212800 dodge base points are 0, so a full dodge claim also requires spell-specific behavior not demonstrated here.

## Proof boundary

Uses the existing real-data Bevy/Lightyear harness (`spell_cast/test_harness.rs`): authenticated HostClient loopback `SpellCastIntent`, known-spell validation, costs, cast/effect resolution, AuraPlugin and CombatPlugin. Candidates are granted as fixture known spells; this does not prove normal class progression grants them. Catalog rows/effects are not altered and defensive auras/modifiers are never injected.

Incoming fire uses real Fireball 133 metadata with a deterministic pre-mitigation amount through production `spell_cast::effects::deal_damage`, the SCHOOL_DAMAGE/periodic damage handler. It deliberately does **not** cast the incoming Fireball or prove its hit/deflect/reflect selection phase. Melee is an actual NPC `AutoAttack` processed on the app schedule: fixed 100-point weapon, zero AP/crit, +100 hit chance to remove random misses, zero victim armor/avoidance. Unprotected control takes 100 on both paths. Absorb assertions read remaining capacity and verify subsequent overflow/depletion behavior.

Client buff icons/combat text, live networking/realm state, timers/expiry, all schools, dispels, cooldown balance, secondary mechanics and talent/spec variants are **not proven**. No osso-5000, other worktree, or production runtime was changed.

## Verification ledger

- Initial focused test development: 14 tests, 12 passed / 2 failed / 0 ignored. Log: `target/defensives-initial.log`.
- Expanded focused test development: 17 tests, 12 passed / 5 failed / 0 ignored. Log: `target/defensives-expanded.log`. Failures above were then annotated as ignored; assertions retained.
- Final focused suite and complete server-crate `cargo test`: pending final verification; exact results will be recorded before delivery.

Commands use the project `agent-run defensives` wrapper in this worktree. Ignored gap repro command: `cargo test -p server --bin game-server defensives_ -- --ignored --nocapture` (intentionally fails).
