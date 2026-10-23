# Demonology DPS parity

Lantha Shadeborne (follower230861, specialization266) qualification against public SimC76f8118. Reference reproduction and source census are established; native parity is **not certified**.

## Reference reproduction — 2026-10-23

Exact1000-iteration commands reproduced196789.57490042652 DPS for300s single-target and420736.290114185 DPS for the300s two-add/30s-wave fixture. Both use level90 MID2 Demonology Diabolist BiS, seed70448, target_level93, fixed_time1, vary_combat_length0, optimal_raid0, item_db_source=local. Three-target command intentionally omits Patchwerk: the preset clears explicit raid events.

Acceptance bands: ST177110.61741038388–216468.5323904692;3T378662.6611027665–462809.9191256035 DPS. Compare native31146 encounters with identical equipment and effective talent ranks; dungeons retain their native kit.

Reference `stats_pets` is a map, not a `pets` list. Recursively include each stat's `children` and divide raw damage sums by the player's DPS sample count999, not a conditional guardian's sample count. This reconciles196789.57490042632 DPS without parent compound double-counting:

| Source | DPS | Share |
|---|---:|---:|
| Owner |59016.525249368|29.98966%|
| Felguard |7940.939825147|4.03524%|
| Temporary summons |129832.10982591129|65.97509%|

Largest channels include Wild Imp10431822.67446%, Tyrant12640939.07443%, Demonbolt2641785.48728%, isolated Implosion13060775.47622%, Imp Lord12771164.45704%. These are full-reference shares, not oracle probe shares.

## Native data boundary

Private SQLite backup of canonical engine `data/server-world/world.db` has SHA256 `51455391e7d4962589cc66d239bf77b2dbf3f4372ec8daee80c092f1e7989d9b`. It lacks published `content_creature_template` rows for Wild Imp55659 and Dreadstalker98035; corresponding TDB templates exist. Queried `guardian_spells` contains no rows for55659/17252/98035/135002/250289/258584. Missing published templates/bindings are state/input gaps, not evidence of a coefficient bug.

Native guardian dispatch explicitly binds Wild Imp55659→104318 and base Tyrant135002→270481. Antoran Tyrant250289 instead reads `GuardianSpells`; source absence must be resolved before diagnosing its per-hit damage. Public SimC `sc_warlock_pets.cpp1510–1548` selects250289 via native265187effect3 and binds1264093; its two-second repeat cadence is explicit SimC policy, not yet claimed as DB2/TC-authored behavior.

The damage-channel census records63 spell IDs/101 native DB2 effect rows. Including75 active selected talent entries expands it to131 IDs/227 effects, with aura options, PPM rows, powers, cooldowns, class masks and labels. Raw TDB1210 `spell_script_names` matches686→`spell_warl_shadow_bolt`,264178→`spell_warl_demonbolt` and three utility spells710/108416/111400 across that expanded census. No matching signed `spell_proc` or PPM rows were observed. TC a352b1fa `spell_warlock.cpp691–710` implements Demonbolt's after-cast energize; absent TC scripts do not authorize invented summon timers or multipliers.

## Explicit summon-root publication

`add_guardian_templates.py --spell104317 --spell193331 --spell193332` (each option and number separated by a space) selects native difficulty0 Effect28/56 creature IDs, deduplicates them and copies exact TDB templates/models through the existing retail-content importer. These roots resolve to55659 and98035. Explicit roots replace the default Darkglare/Wild Imp selection; they do not add fabricated spell lists, alter world spawns or broaden the default Wild Imp publication. Missing summon metadata fails before publication; reruns preserve the same model rows.

## Neutral-target minion ownership boundary

Exact master9d13 native ST produced0 DPS and3T11517.5333 DPS on the original private DB. Fire63 ST control produced24378.3867 DPS. The initial fixture pulls after200ms while native30146 takes6000ms; the dummy's five-second inactivity exit can remove the owner's attack before pet preparation finishes. Native follower code already prepares out of combat and skips preparation with an existing living pet; this is not evidence of a dungeon resummon loop.

After the template-only delta,3T12715.8667 DPS still has no104318 casts. Live GDB on the exact9d13 binary, a detached source checkout over its baked path and the read-only prepared DB observed three actual104317 Imp actors alive and trying104318 on the correct31146. Each `targets::relation_matches` returned false; rejection occurred in `validate_target`, before power checks. A bounded native assertion exits1 (`imp-neutral-red.log`). Direct level32 Imp control succeeds because its target is hostile; the distinguishing condition is **neutral training dummy versus hostile harness target**, not demonstrated level scaling.

`FOLLOWER_CLASS_ACTOR_PLAYER_EQUIVALENT_MINION_FLAGS` is an explicitly approved project policy extending the class-follower spell-owner bridge: controlled minions/guardians inherit PLAYER_CONTROLLED8 from a player-affected owner through existing `faction::player_affected`. TC a352b1fa `Unit.cpp6286–6289` sets this flag only for actual Player owners; applying it to a class-stat follower is a project extension, not a TC claim. Ordinary NPC owners without8 remain unchanged; wild/non-minion summons retain their existing lifecycle, and companion immunities still OR with ownership. All owned guardian paths share `spawn_summon`; controlled Hunter pets/Felguard already unconditionally carry8 through `hunter_pet::spawn`, so that separate path needs no change.

## Evidence

Artifacts under engine `data/diagnostics/demo-parity-2026-10-23/`: `266.reference.{single,three}.{json,txt,proof.json}`, `reference-channels.json`, `source-census.json`, private `world.db` and `world.sha256`. Prior source references are `dps-parity-2026-10-21/266.reference.single.*` and `266.reference-fixed.three.*`, not `all-spec-expanded-references/266.*`.

## Sources

- [Oracle contract](../reference/simc-spell-oracle.md) — match actors/formula IDs and compare first differing factor.
- [Parity contract](../../specs/dps-parity.md) — matched inputs and native-boundary requirements.
- [Guardian dispatch](../../../crates/server/src/spell_cast/guardian.rs) — actual spell bindings and native admission.
- [Wild Imp source importer](../../../scripts/add_guardian_templates.py) — publishes qualified55659 only, not unqualified98035.

## See Also

- [[dungeon-simulator]] — fixture measurement.
- [[simc-apl]] — literal APL execution and native facts.
