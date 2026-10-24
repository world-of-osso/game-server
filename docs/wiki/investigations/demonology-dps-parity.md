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

A borrowed binary initially labelled9d13 was subsequently identified by the lead as an older petgate build, **unknown pre-9d13 revision**. Its historical ST0/3T11517.5333 DPS and Fire63 ST24378.3867 DPS are not exact-master baselines. Own compiled b5b118d86 separately reproduces ST0 and proves native out-of-combat Felguard preparation/no resummon/first-pull damage. The initial fixture pulls after200ms;30146 has native base6000ms cast time (actual duration depends on launch-time modifiers/haste), and dummy inactivity can expire engagement. No exact first-pull timing cause is certified from the borrowed binary.

The borrowed unknown-pre9d13 binary after the template-only delta produced3T12715.8667 DPS with no104318 casts. Its live GDB trace observed three actual104317 Imp actors alive and trying104318 on correct31146; `targets::relation_matches` returned false before power checks. This remains mechanism evidence, not exact-version proof; a9d13 source overlay cannot establish compiled-code revision. A bounded assertion exits1 (`imp-neutral-red.log`). Own47d499c88/b5b118d86 compiled proofs establish neutral-target admission and positive104318damage after the flags change, with an unchanged ordinary-NPC hostile-cast control. Distinguishing condition is **neutral training dummy versus hostile target**, not demonstrated level scaling.

`FOLLOWER_CLASS_ACTOR_PLAYER_EQUIVALENT_MINION_FLAGS` is an explicitly approved project policy extending the class-follower spell-owner bridge: controlled minions/guardians inherit PLAYER_CONTROLLED8 from a player-affected owner through existing `faction::player_affected`. TC a352b1fa `Unit.cpp6286–6289` sets this flag only for actual Player owners; applying it to a class-stat follower is a project extension, not a TC claim. Ordinary NPC owners without8 remain unchanged; wild/non-minion summons retain their existing lifecycle, and companion immunities still OR with ownership. All owned guardian paths share `spawn_summon`; controlled Hunter pets/Felguard already unconditionally carry8 through `hunter_pet::spawn`, so that separate path needs no change.

## Power-regeneration interaction (native boundary qualified)

Own b5b118d86 reports3T36121.01 DPS with7170647 Firebolt damage, but this output is **unqualified**: the existing native Imp energy-lifecycle test fails12casts versus100/20=5. `powers::read_creature_power_regeneration` treated every NPC with ownership flag8 as a player-resource subject, bypassing authored `UNIT_FLAG2_REGENERATE_POWER`. Wild Imp/Dreadstalker/Tyrant templates have unit_flags2=0; Felguard has2048. Ownership is not a regeneration capability. Player-resource classification now uses actual `Player`, `Follower` and `HunterPet` components, not ownership flags. Other NPCs, including temporary guardians, honor the template flag (TC Creature.cpp954–955; native field is unit_flags2 bit0x800). Felguard's authored flag remains2048, and its controlled-pet role stays explicit. Production correction088f90e32 changes only role classification. Own088 cohort has5PASS/1fixtureFAIL49.59s: Imp exactly five casts/exhaustion/despawn, real90 neutral-dummy Firebolt damage, hostile native binding, ordinary-NPC control and actual-player10Energy/s pass. The new role control reached native Legion Strike OutOfRange after its follower Mana assertions passed;81034539f fixes only fixture pet placement. Own810 follow-up2PASS24.96s qualifies real follower Mana, controlled Felguard Energy and controlled Wolf Focus regeneration. Seven distinct cases qualify current production; no compiler messages, private SHA22a233f978cc50921a4e1fa05dfc59630ed151a2d2c2d9d7af9e10843b66fb81 unchanged. Receipts `minion-regen-green.*` preserve the initial fixture failure; `minion-regen-controls-green.*` contain follow-up success and own810 executable SHA. Lead pushed810 as mf1052; DPS parity remains open.

## First-pull transition (confirmed; correction owned by shared fixture work)

Fresh canonical read-only backup SHA dd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae passes quick_check. All runtime loaders use its private worktree symlink, not a mixed older DB. Own810 native300s receipts are ST0 (damage assertionFAIL22.54s) and3T18514.6533 DPS (internal assertionsPASS15.14s), not parity qualification;3T is95.5995% below420736.2901. Actual Firebolt damage1914765 and Felguard209770 are lifecycle-correct, unlike b5's inflated measurement.

Own c8b2ccf32 adds only a transition proof over unchanged810 production. Both ST and3T deal0 in the first10s: owner-to-dummy reference exists at+1–4s, is gone at+5s while30146 still casts; Summon Felguard succeeds at+5.85s, then+6–10s has a real pet but no actor target/reference or further casts. The initial four-frame warmup starts the native six-second preparation but engages before it finishes. Three-target measurement later re-engages at its30s add-wave boundary; ST has no such second pull. This establishes a fixture ordering/engagement loss, not a dungeon pet-resummon loop. Native cast time is observed here; do not generalize it across haste/modifier inputs.

Trace `native-lifecycle-810/266.first-engagement-10s.json`, exact command/env/log `first-engagement-red.*`: genuine assertionRED15.10s, compile16m59s, no warnings. Shared precombat owner14766 and dummy per-attacker timer owner14116 own corrections; no duplicate local gameplay fix. Full193331/193332 casts additionally require target43/42 LOS and ground services absent from this map34 fixture. Guardian melee tests explicitly supply the fixture destination to the real native Effect28 dispatcher; that qualifies spawn→AI→combat, not destination geometry or full child-cast dispatch.

## Guardian melee (native hit enabled; frozen movement blocks clock qualification)

Native source:193331/193332 Effect28→98035, SummonProperties5029/3701 both CONTROL_ALLY1/TITLE_GUARDIAN2; template is demon type3, raw BaseAttackTime0 and speed_run1.14286. TC a352b1fa Pet.cpp847/882–884 sets Guardian base2000ms regardless of template zero; CreatureAI.cpp131–138 establishes owner-target attack pickup. SimC76f8118 pets1197–1202 qualifies Dreadstalker as a melee actor. Own9a effect-boundary cohort1PASS/1genuine assertionRED32.98s: real Dreadstalker emits0melee attempts in5s, while caster Imp retains positive104318 and zero physical swings. The earlier37 Unsupported target43 setup failure is not melee RED.

Shared `GuardianMelee` role is independent of a bound spell list. It selects only these sourced native root/entry pairs, uses ordinary CombatTarget→AutoAttack start/retarget/stop, ordinary PetMotion chase/return with authored run speed, and the native swing/outcome/armor/aura pipeline. Demon identity now comes from creature type even when no spell binding exists. It does not enable melee for caster Imps or invent repeat Dreadbite casts. Own ab79aee5a cohort4PASS/1FAIL24.91s (20m10compile, no warnings) preserves caster exclusion, native Imp exhaustion, ordinary-NPC control and Spirit Wolf AP/clock. Real Dreadstalker now lands1208 at10.25s, but only one of three expected attempts occurs; retarget/stop assertions remain unexecuted. GREEN stays open; root104316 dispatch,271971 binding and earned Carnivorous procs remain separate open producers.

Qualified melee damage reuses `effects::power_stats`, retaining the lead-approved existing57% owner-SP→AP policy; no coefficient/helper fork. TC Guardian::UpdateDamagePhysical1423 supplies its AP/3.5×base-time contribution. `GUARDIAN_INTRINSIC_MELEE_INPUTS_ABSENT`: private snapshot has no pet-levelstats table, and SUMMON_PET/HUNTER_PET level±level/4 weapon formulas do not apply to a non-pet temporary guardian. Therefore no intrinsic/base term, fake Strength22 or ordinary-creature flat10 is substituted. This is owner-contribution coverage, not complete numerical guardian damage. TC1358's IsPet applicability concern and SimC Dread1.06769 divergence remain ledgered for the pet-power owner; neither is silently retuned here. Antoran Burning Cleave1264093 replaces a melee swing on its sourced2s cadence, not additional physical damage; that mode is not implemented by the Dread role.

### Frozen-movement prerequisite

Own ab79 GDB shows the Dread AA/power path active all100ticks without AttackStop; authored193331/193332 duration is12000ms, excluding expiry. `guardian-melee-movement.log` records simulator `movement::move_one` moving it away from dummy x1.5: (2.16,-.23,-.008), (8.43,-2.44,-.087), then (28.24,-9.41,-.336). Thus later swings remain out of range, not stopped by the new clock. This is the frozen-edge simulator mover, not native PetMotion. `walk_route::read` projects off-graph endpoints onto native dungeon anchors and appends both endpoint offset legs; even identical off-graph start/end can produce an out-and-back route. `move_one` requests that route without a chase-reach check. Shared movement ownership was reported to the lead before changing it; no fake speed, collision data, timer or stationary-actor workaround was applied. First GDB script targeted inline-only helpers and yielded no evidence; v2 actual power/stop boundaries and movement probe supply the runtime proof.

## Shared attack-binding producer (implemented; targeted acceptance pending)

`import-guardian-spells.py` first imports authentic archive rows, then fills only wholly absent creature lists with approved `SIMC_DREADSTALKER_ATTACK_BINDING`98035→271971 and `SIMC_ANTORAN_TYRANT_BURNING_CLEAVE_BINDING`250289→1264093. Policy rows have SpellIndex0/VerifiedBuild0, not a fabricated verified build. Any native list for that creature wins wholesale; reimport removes stale policy rows before reading new archive data. The transaction rolls back native and policy changes together on invalid duplicate rows. CLI distinguishes authentic row count from applied policy names; these bindings do not supply summon dispatch or cadence.

Own portable30fbec3ff RED is2PASS/2FAIL: original native/rollback and no-archive controls pass; both approved missing policies are absent, including the Antoran fallback in the future-native Dread-list fixture. That fixture uses qualified IDs but is explicitly hypothetical, not a claim about current TDB1210. No measurement/canonical DB was changed. Full104316 dispatch, target42/43 geometry and earned Dreadbite/Antoran repetition remain open regardless of producer acceptance.

## Conditional Firebolt targeting (open)

`FEL_FIREBOLT_CONDITIONAL_TARGET_POLICY_ABSENT`: local DB2 and pinned SimC spell data both give104318 ChainTargets5, but SimC76f8118 `sc_warlock_pets.cpp754–765` keeps ordinary Firebolt single-target and enables the extra four recipients only with Unstable Soul1281512. Unconditional native ChainTargets is not qualified against that conditional behavior. Do not tune the stored target count to compensate; native conditional policy remains missing.

## Evidence

Artifacts under engine `data/diagnostics/demo-parity-2026-10-23/`: `266.reference.{single,three}.{json,txt,proof.json}`, `reference-channels.json`, `source-census.json`, private `world.db` and `world.sha256`. Prior source references are `dps-parity-2026-10-21/266.reference.single.*` and `266.reference-fixed.three.*`, not `all-spec-expanded-references/266.*`.

## Sources

- [Oracle contract](../reference/simc-spell-oracle.md) — match actors/formula IDs and compare first differing factor.
- [Parity contract](../../specs/dps-parity.md) — matched inputs and native-boundary requirements.
- [Guardian dispatch](../../../crates/server/src/spell_cast/guardian.rs) — actual spell bindings and native admission.
- [Wild Imp source importer](../../../scripts/add_guardian_templates.py) — publishes requested native Effect28/56 template roots and their authentic models.

## See Also

- [[dungeon-simulator]] — fixture measurement.
- [[simc-apl]] — literal APL execution and native facts.
