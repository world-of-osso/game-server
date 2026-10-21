## 2026-10-20 — Earthen Rage authored variance oracle

[Native bounds](systems/auras.md#earthen-rage-baseline--2026-10-19): gate-e7824 observed43/44/43;100SP×0.42471 with±2.5% native variance rounds41–44, not41–43. Only the oracle derives correct rounded endpoints; production/RNG/timers unchanged. Gate failure is RED evidence, targeted/repeated GREEN pending.

## 2026-10-20 — Encounter fixture determinism

[Root cause](systems/boss-scripts.md#encounter-fixture-determinism--2026-10-20): clocks already manual50ms; Maker fixture auto-engages before its late per-test seed. Two-app schedule regression RED precedes shared BossAi-reset fixture seeding. Source5–15s describes cast requests, not500ms-later impacts; Gargolmar aura presence is now checked on actual application instead of arbitrary70s. Under-load repetitions and scripted_ai integration pending.

## 2026-10-20 — Census resource/state/duration/sanctuary

[Source matrix](systems/auras.md#census-generic-resource-state-duration-and-sanctuary--2026-10-20): generic245/262/396/176, no new scripts/protocol. Ignored REDs pushed before consumers;262 corrected from uncomputed health state to actual Swiftmend15. TC NULL220/443 and absent phasing167/viewpoint72 remain explicit gaps. Native12GREEN31.37s before enabling; merge3e1add814→8922af04b retains137/307;67targetedPASS35.88s/check/fmt with zero warnings. Native670 unique receipts preserve source passives and91 selected affected memberships:34 owned family warnings clear across18 specs (262×11/396×6/245×4/176×13),57 named-gap memberships retain warnings. Interrupted96 complete receipts retained plus574 remaining PASS366.39s; exact manifest coverage verified, no whole-spec parity claim.
## 2026-10-20 — Generic census healing, label mobility and energize

[Generic next](systems/auras.md#census-generic-next--2026-10-20): initial RED498eeb521 was invalid (limited fixture store omitted native applied/triggered IDs). Corrected ignored68f31ad3d pushed with consumers reverted gives6RED and253 baseline probe receipts; restored6c76aead4 gives unchanged6GREEN plus253-receipt probe PASS178.44s before enabling. Earlier14-root warning delta includes354eight now inherited from merged master27743876c; duplicate354 consumer removed, existing master wildcard behavior retained. Merged bde34597a15PASS215.50s includes six cases, retained eight330 controls and253-receipt probe; check/fmtPASS, no warnings. Branch-owned delta is307four and137two across six specs; this is not full ability/spec parity.470 explicitly TC NULL; no fabricated timer scaling or per-spell scripts.

## 2026-10-20 — Generic combat/stat aura consumers

[Per-type source matrix and ledger](systems/auras.md#generic-combatstat-aura-census-slice--2026-10-20):12 generic consumers,4 explicit TC NYI gaps. Pushed ignored REDs first; corrected retired weapon ID, saturated armor and pre-owned expertise fixtures without changing native spell data.419 current mana clamps instead of scaling.316 accounts absorption but does not invent a script rescue. Final861d7edd5 targeted15 enabled PASS26.31s/0warnings, protocol600b755; no full suite or independent parent gate. Receipt-backed pinned sources supersede shifted citations from the broader TC tree; compared handler bodies are identical.
## 2026-10-20 — Guardian aura split and Death Grip

[Guardian damage](systems/auras.md#guardian-and-controlled-pet-damage--2026-10-20) separates531 guardians from429 controlled pets and matches victim380 to the guardian's owner; [Death Grip](systems/spell-casting.md#death-grip-and-destination-jumps--2026-10-20) binds native children and42/213 jump clocks/no-facing flag. Twelve ignored native tests first RED4PASS/8FAIL; owner-selection correction replaces invalid melee no-receipt with genuine10vs20 at `eee8013db`, same4PASS/8FAIL23.02s. Original380 Wound spell was an invalid IgnoreDamageTakenModifiers oracle; native208227 with real missile impact gives isolated1PASS/2FAIL28.71s before restoring the target consumer. Native49575 target64 radius15 lands3yd forward, not exact origin; corrected Death Grip5/5 already pass. Implemented/enabled at2304683e1: corrected13/13GREEN22.80s; six-name retained filters318PASS/1 missing-ground-envFAIL/2existing ignored226.67s, explicit-ground retry1PASS10.95s. Census/families preceded external shared sibling advancement; remaining integration pins private detached600b755. Merged28094f04e exposed RequeueSkirmish absent at600b755; parent explicitly superseded pin to private03cebdb. Final combined filters `pet guardian summon jump warlock hunter native_census_`:321PASS/0FAIL/2existing ignored171.38s, all13 census cases and native Command/Restoration included. Explicit ground env, three private path overrides; cargo fmt checkPASS/no warnings. Gate log `/home/osso/.worktrees/logs/census-grip-merged-03cebdb-gate.log`. Aura320 remains named **AURA_320_LABEL_SELECTOR_SOURCE_CONFLICT_70448**, not implemented.

## 2026-10-19 — Private native-proof purchase hold harness

[Hook and proof status](investigations/server-tick-lag.md#native-proof-purchase-hold-test-only-harness): explicitly approved non-default debug Cargo feature holds only the existing FIFO purchase/checkout work behind private files, keeping UDP live and the10s timeout unchanged. Pushed-first process cases reached two genuine missing-hold assertions; fixed30s watchdog refuses without charge and release builds are rejected. Feature7/default2 GREEN; parent persistence/check/fmt gate pending. Release guard source-inspected, not a release-hook build; no tracing or native runtime used.

## 2026-10-20 — Spell cooldown persistence

[Contract](../specs/spell-cooldown-persistence.md): player spell/category cooldowns persist as wallclock deadlines at logout and replay as `SpellCooldownUpdate` at world entry (TC SpellHistory SaveToDB/LoadFromDB/SendSpellHistory). RED e9b2860dc/cca7855fc failed at relog replay for Prismatic Sphere28027 and Berserker Rage18499 (in-session rejection already held); GREEN passes in session, after relog and after restart, plus load-time pruning.
## 2026-10-20 — Armor specialization equipped armor

[Item requirements](systems/auras.md#armor-specialization-equipment--2026-10-20): ignored RED pushed first (9b073510a, rebased ae2683f3b); three genuine failures at the aura-absent assertion, full-plate control passes. Fix b1a250fb2: player Attr8 armor passives need all eight armor slots to fit (Paladin loadout164 plate, Druid leather chest 72689 negative). GREEN 826/0, 4 pre-existing ignores, 252.73s: auras::, stats, followers::passive_stats, class_progression, inventory, combat::tests, class_completeness. No existing test changed. Remaining item-requirement gaps in the [spec](../../specs/player-damage.md#known-gaps-current-cycle).

## 2026-10-19 — Omen and one-use Clearcasting

[Native inputs and proof boundary](investigations/follower-data.md#omen-of-clarity-and-one-use-clearcasting--2026-10-19): actual learned15/native4%113043→16870, zero native charges with existing free-Regrowth modifier. Pushed three ignored cases0544ff450 give1PASS/2FAIL15.29s: buff never consumed, unrelated Rejuvenation refreshes it;15/15 tick/deadline changes plus generic DB100 reader expose missing native effect-chance check. Only113043 own-periodic33763/native4% check and composed next-Regrowth consumption added54d83ff56; unchanged3GREEN19.98s/zero warnings before enabling976f. Real acquisition48 eligible ticks each under derived339 confidence bound; no RNG distribution certification or fake inputs. Latest unrelated master merge2a7573d02; final independent gate pending.

## 2026-10-19 — Tranquil Mind two-stack Clearcasting

[Native source and proof](investigations/follower-data.md#tranquil-mind-two-stack-clearcasting--2026-10-19): actual25 native403521s3 chance5%/s1 op37+1 cap; existing generic cap already passes actual24 repeated procs/25 third proc. Pushedc4aa native3 gives2PASS/1FAIL28.55s: two earned stacks→first free Regrowth wrongly removes both. Minimal2b8f chooses native5% and existing spend_stack; unchanged3GREEN17.68s/zero warnings proves2→1→0 then actual paid Regrowth before enablingf43b. Binomial bounds cover multiple late-enough natural procs per finite native lifetime, not a frequency certificate/gameplay timer. Clearcasting index/investigation merge conflict resolved retaining both sides; requested merged Clearcasting3PASS16.04s atcd1bbe, then Tranquil rebased onto that tip with four patch-equivalent commits. Independent final gate pending.

## 2026-10-20 — Shattered Halls O'mrogg

[Warbringer O'mrogg16809 and heads19523/19524](systems/boss-scripts.md#shattered-halls-omrogg--2026-10-20) ported on BossAi(2) with a Burning Maul AuraScript and a Beatdown AfterCast; new `creature_aggro::reset_all_threat` (TC ResetAllThreat, references kept). Prepared fixture WorldData now carries canonical equipment. Qualification, pushed ignored RED, GREEN8/8 before enabling. `OMROGG_BURNING_MAUL_30620_ABSENT_70448`.

## 2026-10-20 — Blood Furnace Keli'dan

[Keli'dan17377 and Shadowmoon Channelers17653](systems/boss-scripts.md#blood-furnace-kelidan--2026-10-20) ported on BossAi(1) with a Burning Nova AuraScript; shared `Hook::JustReachedHome` at home arrival and non-forced `SpawnGroupSpawn` (keep living, replace dead). Qualification, pushed ignored RED, GREEN11/11 before enabling. `KELIDAN_HEROIC_VOLLEY_40070_ABSENT_70448`.

## 2026-10-20 — Blood Furnace The Maker

[The Maker17381](systems/boss-scripts.md#blood-furnace-the-maker--2026-10-20) ported on BossAi(0) with `Ctx::dungeon_mode` consolidation; qualification, pushed ignored RED, GREEN5/5 before enabling. Read-only trace found no canonical542/543 creature bindings (`RAMPARTS_BLOOD_FURNACE_BOSS_BINDINGS_UNPUBLISHED`).
## 2026-10-20 — db2_casc scalar-first pallet order

[Decoder notes](../db2-casc-sparse.md#fixed-record-decoding):
- `db2_casc` fixed decoding now goes through one `read_fixed` parser, which the engine readers are moving onto.
- Fixed latent bug: pallets were read in field order. Real PlayerCondition 1045411 declares an array pallet before scalar pallets, so all 48,491 rows read WeaponSubclassMask 1048866 / ContentTuningID 958 instead of 0, and the ContentTuningID pallet repeated entries.
- No import used that path; `world.db` PlayerCondition comes from the scalar-first engine CSVs.
- Regression: `test_db2_casc_player_condition`, RED on the old decoder, GREEN now.

## 2026-10-19 — Ordinary quest max-level bonus

[Authored bonus](systems/quests.md#authored-max-level-money--2026-10-19): pushed ignored65805b8eb ordinary62 cap RED105000≠105120 and missing-catalog startup RED; player89 control passes above quest tuning ceiling30. Selected-TC nullable metadata exports tracked catalog, required sibling loader and deploy shipment; ordinary settlement leaves base previews/WQ/captured/legacy paths unchanged. Fresh ignored607a8da1a GREEN4/0/33.87s/no warnings, then enabled; merged566 family89/0 plus disjoint rules/data/view19/0 (one existing private-population ignore), native producer/deploy3/0, helper check/fmt0. No deployment; independent lead landing separate.
## 2026-10-19 — Lifebloom near-expiration refresh bloom

[Source and proof boundary](investigations/follower-data.md#lifebloom-near-expiration-refresh-bloom--2026-10-19): native33763 tooltip names near-expiry refresh33778; explicit `LIFEBLOOM_REFRESH_BLOOM_THRESHOLD_SIMC` sources inclusive30% from SimC76f8118. Initial protocol mismatch is compile-only, then actual native7e5091e7e gives1PASS/1FAIL23.15s: late ordinary refresh retains HoT/extends deadline but missing33778. Minimal pre-aura-application old-own-deadline hook plus visibility correction4c3723efd gives unchanged two+retained lifecycle/swap/tank6GREEN16.25s/zero warnings, then enabledf952. Native costs/periods/global refresh unchanged; latest master/Wild Growth mergea51c preserves both specs, independent final gate pending.

## 2026-10-19 — Wild Growth native-coefficient falloff policy

[Source and proof boundary](investigations/follower-data.md#wild-growth-native-coefficient-falloff-policy--2026-10-19): explicit `WILD_GROWTH_FALLOFF_SIMC_SHAPE_ON_NATIVE_COEFFICIENT`, never legacy0.175/TC curve; absolute Retail shape uncertified. Pushed3ignored1fb0e4983 gives1PASS/2FAIL23.67s: native42 constant62×7 genuine decline RED,43 relative check blocked by same baseline prerequisite, natural expiry PASS. Minimal48438-only scheduled-deadline multipliere930 plus landed master merge43e0 gives unchanged3GREEN20.54s/no warnings:42 actual65→57,43 normalized native40%-less and unchanged seven-tick expiry. Enabled only afterward3327; final independent lead gate pending.

## 2026-10-19 — Vazruden actual slay speech

[Bounded slay](systems/boss-scripts.md#vazruden-slay-speech--2026-10-19): unrealistic1M buffer did not qualify lethal180s; approved native level10/class1 stat producer1110HP gives ordinary17537 melee death/corpse, qualification1PASS42.42s. Pushed ignoredfdeea2RED/1control55.54s plus real ownedHound reward-owner baseline1PASS59.78s; sole unfiltered killed_unit→Talk2 hook4d2f gives fourGREEN79.39s before enabling. Five-case composite with retained qualification, independent gate pending. Controls use approved production Unit::Kill HARNESS SETUP, no HP shortcuts/fabricated hooks or owners; no source timer/ability/native-audio/loot/persistence change.
## 2026-10-19 — Fel Iron chest flag preflight blocked

[GO_NOT_SELECTABLE_FLAG_UNMODELLED](investigations/go-not-selectable-flag.md) records the read-only placement/interaction census and existing Isle keep-banner unlock hazard. Fel Iron MakeActive stopped before implementation; raw census attached from the separate engine data evidence tree. No server/protocol/client flag semantics or chest behavior changed.

## 2026-10-19 — Herald flying Liquid Fire

[Flying actual-impact boundary](systems/boss-scripts.md#herald-flying-liquid-fire--2026-10-19): qualification73bc2PASS64.88s establishes natural victim/random admission and positive impacts in Normal/Heroic FIRST. Pushed ignored4b11→four genuineRED200.85s; production adds only four IDs to existing Fireball hit hookd75c→four full nativeGREEN287.96s before enabling. Actual frame-local child/freshGO prevents stale victim-field acceptance; source GO lifecycle/damage/copy oracle reused unchanged. Retained natural qualification2PASS plus extracted ground/source/refusal6PASS183.95s ate0e53 completes a twelve-case composite; independent gate pending. No broader flight/native/balance/loot/persistence claim.

## 2026-10-19 — Server-recorded crafting provenance, targeted GREEN

[Storage and proof boundary](systems/inventory.md#server-recorded-crafting-origin): pushed-first actual2153 craft/fresh-DB owner-snapshot RED; additive GUID metadata preserves ability1355 joinedspell2153 without changing legacy instance encoding. Actual2963×2 exposed unknown-stack merging; origin-compatible merge/capacity and split-copy controls pass, class item-creation remains unknown. Server2 targeted cases GREEN at5ea22bf55; protocol/engine proofs are separately pinned. Atomic lead gate and any new native session remain pending; recraft/evaluator/slots/watermark stay unavailable.
## 2026-10-19 — Ordinary NPC authored cadence

[Source timing](systems/player-damage.md#ordinary-npc-authored-cadence--2026-10-19): pushed ignored674bb88eb before two genuine ordinary-source-spawn REDs (Prowler1181500ms/Murloc1713000ms both observed2.050s). Ignored05df30702 adds real2825/+30,246/−35 and mid-swing progress controls: all five genuine RED, helperexit101/23.99s;246 measurement bounded before authored6s expiry. Loader/base+difficulty template cache now supplies the ordinary white clock with TC zero normalization and signed/progress factors. Player/controlled-pet/SpiritWolf weapons and per-hit damage stay unchanged. Fresh ignored2386d767dGREEN5/0/15.88s/no warnings, then enabled five plus source-zero and combined252/319 controls. Requested family proof: combat228/0 (all seven controls), creature/zones/world-data/startup242/0, disjoint scripted/encounter/SpiritWolf/pet-control380 passes plus private Azure2/0 at protocol50517c9. Fixture-only ac3db20a9 projects actual Herald/Nazan2000ms and explicit synthetic TC2000ms; no swing-count assertions changed. Missing-column/private-environment attempts retained separately; no full-suite or deployment claim.
## 2026-10-19 — Native forced-miss readiness oracle RED

[Follow-up](systems/auras.md#attack-readiness-oracle-follow-up--2026-10-19):7ff4c9857/cc83515b6 fixed recovery, not initial HP⇒attack helpers. Pushed ignoredea66eb290 native50240 counterexamples give2RED21.29s/three actual Miss attempts each; minimal TEST-only event/Pursuing/currentvictim correction preserves100/120-frame windows and every wound/death/evade/regeneration assertion. No production/RNG/input changes. CurrentVictim namespace compile correction1aea734bf (not behavioral proof); fresh two-module31GREEN94.76s/0warnings including new2ignored via --include-ignored, then both enabled. Independent lead gate pending.
## 2026-10-19 — Herald ground Liquid Fire

[Ground child/object boundary](systems/boss-scripts.md#herald-ground-liquid-fire--2026-10-19): source qualification1PASS27.70s; corrected actual-launch ignoredcb85 pushed on pre-runtime base, four genuineRED156.95s. Ground-only hit-unit self-children plus exact registered GO AI/native21252 ordinary pulses/damage/lifetimes pass four complete endpoints159.72s at7ea954c38; retained source/refusal controls make a six-case composite. Invalid initial network child receipt oracle excluded; no wire workaround. Tests enabled, independent gate pending; flying/native/balance/persistence/re-entry/loot excluded.

## 2026-10-19 — Auto Shot75 autorepeat

[AutoRepeat](systems/spell-casting.md#auto-shot-autorepeat--2026-10-19): ignored RED606c6ead2/2ded8d38e pushed first; four genuine failures (one shot, no repeat). Fix997d00e04 repeats on the ranged timer (rifle73000 delay3000/haste), no GCD; moving keeps it, out of range skips, StopSpellCast/first stun/fear/confuse cancel via `stop_all_casts`. Resume-window oracle correctedcbf8c8d9d (window included the next legitimate repeat). Stun test RED-proven by reverting only its call (exit101 at the cancel assertion). GREEN169/0,3 pre-existing LOS ignores,66.96s: auto_shot, Dark Luck75, defensives, autoattack range, attack speed, combat::tests, dummy, control stop, fear/confuse/stun. Gaps in [spec](../../specs/player-damage.md#known-gaps-current-cycle).
## 2026-10-19 — Herald fixture-only admitted Roar seed

[Ground diagnostic boundary](systems/boss-scripts.md#herald-ground-timing-diagnostic-boundary--2026-10-19): hc729's three legitimate category-cooldown refusals leave the1500ms probe without an admitted cast. Approved actor-local fixture seed0 at natural release gives authored initial Fireball3656ms/Cone7735ms, after Roar2000ms plus1500ms cast. EventMap clock/queue, source ranges/repeats/admission and production RNG remain unchanged; exact expiry oracle retained. Three independent native stability executions7eff0e0fa/protocolfc37 pass36.11s/37.75s/41.14s, each actual first Roar3.95 admitted/exact1450/1500ms AI expiry; initial compile-only visibility failure excluded. Independent changed-family gate/co-landing pending.

## 2026-10-19 — Herald native cast millisecond root fix

[Captured root cause](systems/boss-scripts.md#herald-ground-timing-diagnostic-boundary--2026-10-19): hr705 Cone38.65→53.70/Roar52.15+1500ms=53.65 isolates shared float progress1.4999996 after30×50ms. Pushed ignored actual native AI regressionbdb50ac74 fails39.30s; shared normal/channel/pushback3RED +nearest-ms control precede integer-ms runtimebb18da9. Shared53GREEN; strict native1450/1500ms endpoint50.60s plus six retained native ground/Omor pushback controls105.21s. Nearest-ms conversion/public f32 wire/pushback/source windows retained; no epsilon or broad rerun. Engine has no shared tick caller. Paired protocol/server independent gate/co-landing pending; original docs-only1a5 closure held.

## 2026-10-19 — Herald ground timing diagnostic capture

[Standing diagnostic boundary](systems/boss-scripts.md#herald-ground-timing-diagnostic-boundary--2026-10-19): original full-workspace Heroic36921 late repeat remains unreproduced after actual same-CPU load1PASS118.34s and small shared-process/six-thread7PASS49.68s. Approved permanent pre-assert spell/Roar dump and test-only opt-in actual AI/cast clocks added; single expanded module261PASS/2private-Azure-envFAIL/6ignored811.19s did not reproduce; subsequent hr705 isolated the shared cast-progress defect above. No assertion/source timer/RNG changes; standing diagnostic retained.

## 2026-10-19 — Herald real evade ingress RED

[Loaded-copy reset contract](systems/boss-scripts.md#herald-loaded-copy-evadereset-boundary--2026-10-19): ignored RED first; native group357/wipe HARNESS SETUP plus ordinary boss-damage death control2PASS/four genuine missingFAIL RED121.66s. Real source despawn/FAIL hooks ported before next reset-boundary proof. Ingress8985f8af3→four genuine group-restoration RED110.75s. Independent copyclock/ordinary group357 ignoreRespawn operation→four full endpoint GREEN67.43s atb549ffbaf after correcting fresh default-reaction oracle. Source exact deadline/alive identity/count/patrol/min1/fresh natural intro pass; tests enabled, independent gate pending; no canonical data/flag changes.

## 2026-10-19 — Nature's Guardian native RED and named binding

[Baseline](systems/auras.md#natures-guardian-baseline--2026-10-19): three-case qualification COMPOSITE FIRST (two retained PASS including actual player-cast preservation/native45s expiry/exact350340; corrected native2000ms wound observation single PASS17.53s). Pushed ignored823c414e4 genuine5RED/3controls21.36s; named30884 hooks use existing limited cooldown path then source-costed heal path. Strict threshold/no custom state/timer/API or HP/RNG/clock edits. First behavior7PASS/1idle-HP-oracleFAIL28.82s; diagnostic61805c213 proves seven natural20HP out-of-combat regeneration ticks/unchanged native heal count. Explicitly approved TEST-only continuous-combat correction retains initial26 hits/native45s and uses actual damage/heals for final HP, with no state/timer writes or runtime change. Corrected single case2c608090c1GREEN26.41s: actual expiry47.2/proc47.8/newCD92.8,46 ordinary hits/actual damage1260/healing800/finalHP540. Other7PASS retained; only then eight enabled. Eight behavior7+1 plus qualification3 is eleven-case COMPOSITE, not one batch; runtimebed0f8b2d unchanged/lead gate pending.248 remains DEFERRED.

## 2026-10-19 — Spirit Wolf native RED and lifecycle hooks

[Named baseline](systems/auras.md#spirit-wolf-baseline--2026-10-19): accepted active70448 receipt-matching inputs; qualification64cf5318e2PASS25.00s FIRST/exact1/2/3/4s cap4/movement1.2/real800damage, then pushed ignoredb8b6e3f036RED/2controls32.08s. Apply/remove hooks only on2645/260878 dispatch real260882 and clear260882/260881; no new state/timer/periodic/wire. Fresh ignored2d5ed1cfc8GREEN19.76s/0warnings, then eight enabled; qualification2PASS25.00s retained separately (ten-case composite). Independent lead gate pending;248 remains DEFERRED.
## 2026-10-19 — Bounded ordinary BG Preparation44521

[Aura lifecycle](../handoffs/pvp-bg-preparation-aura-proof-ledger.md): ignored36bfc2ea0 pushed before genuine6RED/7controls (13cases11.98s). Native authored44521 makes Fireball Mana20/Energy45 SinisterStrike cost/admission zero, then normal removal restores exact baselines. Consumerb0a123256 adds WaitingACK/start/BG-exit role boundaries with normal AuraAPI, no arena reset or new effects. GREEN13/0/0ignored12.32s at31dc2bb27 before enabling six regressions; separate retained76/0/0ignored18.89s at6d03ade8b after master merge (new13 skipped,89distinct notone89run). Lead integration pending. Generic interrupt semantics remain `AURA_INTERRUPT_LEAVE_ENTER_WORLD_SEMANTICS_UNQUALIFIED`; no zoning/logout/persistence purge.

## 2026-10-19 — Ordinary battleground preparation

[Preparation follow-up](../handoffs/pvp-bg-preparation-proof-ledger.md#bp732-integration-correction-and-test-driver-audit): bp732278PASS/3FAIL/1ignored, CHECK0/FMT0. Test-only natural wait for reconnect/flag captures, original delta preserved after native wait, fresh preparation per100 replacement matches. Grep14modules found two additional flag-channel tests; exact3+own4+flag2 GREEN9/0/0ignored53.80s on rebased2a6ab113e/protocol7f9bd7c. Production unchanged; Lead regate pending.

[Preparation](systems/battleground-preparation.md): source first-arrival120/60/30/0 clock, original roster minimum and sourced copied-map gates after native concrete/Random RED2. Actual10-map gate/loader2PASS; corrected pending-transfer/clock2PASS in partial retained114/25. Exact failed25→22PASS/3FAIL, then qualified ground root/restored original frame cadence→last3PASS53.66s atbc3850ae0. EDQUOT and two compile prerequisites remain separately recorded. [Proof ledger](../handoffs/pvp-bg-preparation-proof-ledger.md) qualifies revision scopes; no unified current-tip retained pass. Lead integration pending; Seething missing placements named; no UI/44521/resources.

## 2026-10-19 — Earthen Rage native RED and consumer

[Named baseline](systems/auras.md#earthen-rage-baseline--2026-10-19): qualification3PASS67.67s FIRST; ignored6ae2239e1 pushed before genuine5RED/3controls17.89s. Existing A4/A226 hooks plus one passive-caster target Entity dispatch native children; target removed with passive, no selection fallback/framework/wire. First consumer attempt6PASS/2FAIL20.25s exposes source-invalid two-child/no-in-flight-arrival oracles. Approved separate oracle correction cites SpellMisc144253 Attributes_5=8704/0x200 initial tick and child Speed90/LaunchDelay0.75; native0/3/6 dispatches and already-launched missile preserved,180-frame window unchanged. Corrected ignoredc27aad3788GREEN18.15s/0warnings, then eight enabled; qualification3PASS67.67s retained separately (eleven-case composite). er723 landed d53e8f2b8, gate/check/fmt0;248 remains DEFERRED.
[Named baseline](systems/auras.md#earthen-rage-baseline--2026-10-19): qualification3PASS67.67s FIRST; ignored6ae2239e1 pushed before genuine5RED/3controls17.89s. Existing A4/A226 hooks plus one passive-caster target Entity dispatch native children; target removed with passive, no selection fallback/framework/wire. First consumer attempt6PASS/2FAIL20.25s exposes source-invalid two-child/no-in-flight-arrival oracles. Approved separate oracle correction cites SpellMisc144253 Attributes_5=8704/0x200 initial tick and child Speed90/LaunchDelay0.75; native0/3/6 dispatches and already-launched missile preserved,180-frame window unchanged. Corrected ignoredc27aad3788GREEN18.15s/0warnings, then eight enabled; qualification3PASS67.67s retained separately (eleven-case composite). Independent lead integration pending;248 remains DEFERRED.
## 2026-10-19 — Aura343 white-swing vulnerability

[MOD_MELEE_DAMAGE_FROM_CASTER343](systems/auras.md#damage-and-healing) scales the attacker's own white swings; Colossus Smash208086 now applies effect1. RED aa26a134b genuine at the343 boundary (empty343 effects; swing35 outside44–46); fix bae6bf60c. GREEN167/0 142.95s,0 warnings across the new tests, Colossus T150, attack power, melee crit/creature-type/summon/armor/block, versatility, crit-damage and combat::tests.

## 2026-10-19 — Aura248 native-input gap

[Named gap](systems/player-damage.md#aura248-native-input-gap--2026-10-19): AURA_248_NO_NONZERO_NATIVE_INPUT_70448 DEFERRED after fresh all-difficulty canonical scan finds only261912effect4 amount0/Misc0 and no imported server-side248 rows. Synthetic-only implementation declined as speculative; contract/wiki record native-proof boundary, no production/test/data change or coverage promotion.

## 2026-10-19 — Voluntary leadership authority RED

[Source/policy/implementation](systems/guild-lifecycle.md#voluntary-leadership-transfer): permitted Retail UI/C API signature and full pinned TC self chain qualified before code; native Retail self behavior unproven, approved named Permissions refusal preserves state. Ignored P/E/S regressions pushed first; initial S unsupported-wire failures are not authority proof. Recognized-request baseline then reproduces four actual authority failures before member-rank mutation. Minimal rank0→target/current-lowest→former-master transform uses existing atomic persistence and recipient refresh. Six-rank/nonzero bank fixture, reopen, auth/foreign/self and real concurrent readers; focused GREEN/gate/native pending.
## 2026-10-18 — Shared interrupt provider staging

[Provider contract](investigations/follower-data.md#native-dispel-and-caster-control-conditions--2026-10-18): test-only API matches9468's ordered caster interface; production callers untouched. Native five readiness/level/order/no-pursuit cases PASS af379; charge actually moves to native source reach but its initial child CastSuccess oracle is unsupported. Ordinary106839 reference motion replaces that nonexistent wire expectation; native motion/legacy/pet17/pet16 four PASS d60 in62.79s, consolidated9enabled. Native parent/cooldown/source model reach retained, learned Axe Toss does not cancel owner Shadow Bolt. Runtime module activation +old-path removal reserved for atomic joint caller gate, not shipped.

## Source interrupt eligibility

[Live predicate](systems/spell-casting.md#live-source-interrupt-eligibility): full pinned TC OR/bypass ordering at selection and effect68; damage pushback unchanged. Current ce431b54c focused14/14 pass47.85s/0warnings, including native9, four real104773 protection negatives and unchanged Azure facing. Approved source-duration setup bound250ticks derives1.5s cast+6s stun+100 original50ms ticks; Fissure admitted147. Fissure source0 assertion corrected without altering geometry, native data, CC or timers. No live hazard timing or whole-branch GREEN claim.
## 2026-10-18 — Herald paired-death credit RED

[Paired death contract](systems/boss-scripts.md#herald-paired-death-credit-boundary--2026-10-18): native ordinary dead30693 qualification passes; six genuine missing1892/bit1 REDs108.35s plus four isolated reverse-order REDs83.24s. Source per-AI other-dead/action1 and last-death completion ported001d76905; fivePASS/sixFAIL identifies temporary17537's absent ordinary origin. Corrected actualNpc+same-copyTempSummon.summoner647169a24→all11/11GREEN187.96s,0ignored/no warnings; both orders/frame/copy/corpse/unrelated/deathspeech/deathCall pass. Enabled tests, independent retained gate pending. Chest/reset/Liquid Fire/native excluded.
## 2026-10-17 — Victim current-health crit183

[Inclusive current health](systems/player-damage.md#victim-current-health-crit183--2026-10-17): qualificatione5e06d364 FIRST2PASS30.37s0warnings/actual400ordinary10damageevents5000→1000thenbelow/noHPwrites, native18350/MiscB20/permanent800s; THEN ignored13c203c3cPUSHED. Genuine2RED/4PASS55.53s:above20wirezeroCrit20normal, exact1000purechance0vs50. BorrowexistingtargetHealth/sharedreader, inclusive truncated threshold before receivedclamp/334; forecastCURRENTsnapshotONLY/no future-crossing simulation. Numeric boundary on actualstate is not twenty independent wire rolls. Consumer49be5b891 fresh ignored6GREEN65.81s/0warnings +retained qualification2PASS30.37s (eight-case composite), only then six ignores removed; lead gate pending. No new query/state/wire/RNG/HP/clocks/Guardian changes,248 deferred.

## 2026-10-17 — Victim direct-summoner crit339

[Direct owner filter](systems/player-damage.md#victim-direct-summoner-crit339--2026-10-17): qualification FIRST retained8857544073PASS42.38s +focused817062fd2parent1PASS30.07s,0warnings; actual33950/6000ms,TENnaturalapplications×2ordinaryattempts,naturalexpiryBEFOREnext. THEN ignored79c63a6abPUSHED; genuine2RED/7PASS59.79s, matchingowner/parenteach0Crit20normal. ExistingTempSummon.summoner through tuple→Snapshot→Attacker/sharedforecast;339filtersdirectowner,306actualattacker retained, no new query/component/wire/RNG/HP/clocks/Guardian. Cancellation control waits ORIGINAL6sdeadline after real removal, not claimed natural removal; other windows natural. Consumerbd49764fe fresh durable `ignored-green2`9GREEN50.74s/0warnings plus retained qualification3+1 (thirteen-case composite); first foreground compile interruption is NOT behavior proof. Only then nine ignores removed; lead gate pending,actualTempSummonautoattack-only/248deferred.

## 2026-10-17 — Victim caster-specific crit306

[Actual caster filter](systems/player-damage.md#victim-caster-specific-crit306--2026-10-17): qualification73953d911 FIRST4PASS74.56s/native1259256 identity/+100/lifetime768s and source52/197/334/player0/NO_CRIT pairs; THEN ignored2fae8312c PUSHED. Genuine4RED/8PASS77.77s0warnings includes matchingNPC/player, source52 preclamp andNO_CRITsource+306; foreign/placement/cancel/197/334/summonowner controls pass. Pure existing victim-aura caster==actualattacker filter before received clamp, shared forecast/no new state/query/wire/RNG/HP/clocks/Guardian change. Consumerfcf19ae71 fresh ignored12GREEN87.79s/0warnings plus retained qualification4PASS74.56s (sixteen-case composite), only then twelve behavioral ignores removed; lead gate pending. No invented negative306/multicaster coexistence certificate,248 deferred.
## 2026-10-18 — Rust reader consolidation complete; named Python gap

[Coverage matrix and decision](systems/wow-formats.md#coverage-and-named-python-tooling-gap): `WOW_FORMATS_PYTHON_TOOL_READERS_NOT_CONSOLIDATED` records independent Python tools with no approved shared Rust call boundary. Keep ground_files.py unchanged; no one-use bridge or replacement. Rust runtime/geometry consolidation is complete.

## 2026-10-18 — Ground metadata tooling qualification

[Before-only boundary ledger](systems/ground-assets.md#before-only-metadata-tooling-qualification): frozen actual walk/CLI outputs, missing ordering and original failures; no deploy/extraction, parser replacement or bridge/API change. Keep permissive script policy separate from strict runtime projections until a raw-field boundary is approved.
## 2026-10-18 — BLP icon qualification, no migration

[Icon boundary and before-only ledger](systems/wow-formats.md#blp-icon-boundary): actual icon PNG/RGBA/error/panic receipts plus five source-selection edges; no production/pin/asset changes. Engine normalization and BC5 support are not server defaults. Fifth crate/icon migration remain unapproved.

## 2026-10-19 — Effect64 launch phase

The lead gate cs656 failed the Lantha Axe Toss test because [first-stun cast stop](systems/auras.md) ran before the hit-time effect64 child. [Effect64 launch phase](systems/spell-casting.md): RED cd7e783bd 2/2 genuine, fix ec953eeac. GREEN 44/0 in 118.99s, 0 warnings (control_cast_stop, pet_tests, Bo, queued_trigger families, warrior stuns, missiles).

## 2026-10-19 — First stun stops the current cast; Bo control oracle

[First-stun cast stop](systems/auras.md): native Paralysis115078/Leg Sweep119381 landed on a casting enemy without stopping it, making Bo's Leg Sweep oracle RNG-dependent ([follower data](investigations/follower-data.md)). RED e3f137f13 3/3 genuine (stun applied, CastState kept); fix3e0e8f09b. GREEN32/0 63.47s,0 warnings: three new/rewritten tests plus warrior stun, creature-trigger stun and follower CC controls. Lead family gate pending.
## 2026-10-19 — Online guild zones accepted

[Producer authority](systems/guild-lifecycle.md#online-member-zone): names load once with existing AreaTable; no GuildPlugin database open. Retained guild/bank/achievement writer/area/rest140/0/3ignored, coordinated gz689 passed/landed. Actual authenticated two-member both-skin zone/refresh/disconnect native proof accepted; engine wiki owns artifact/PNG provenance and limitations. No new sorting/timestamps/leadership behavior.

## 2026-10-18 — Herald DESCEND4 parent boundary RED

[Ground parent contract](systems/boss-scripts.md#herald-descend4-ground-parent-boundary--2026-10-18): ignored RED pushed before production; native qualification1PASS/four genuine failures86.63s at9691ff61b. Sourcefall+2000ms release and ordinary parent timers ported354302f74; five native casesGREEN137.57s. Enabled final43/43GREEN374.35s at1799e2743/master717ae67df,0ignored/no warnings; exact lethalbefore/after, source initial/repeats and actual parent damage/fear proven without overrides. Real shared-category OnCooldown preserved; first Roarfall+3950ms reflects source50ms combat advancement at release. LEAD gate/check/fmt pending. Liquid Fire effect77 children/pairedcredit/reset/native excluded.
## 2026-10-17 — Victim general crit197

[Received crit](systems/player-damage.md#victim-general-crit197--2026-10-17): qualificationc6da45e6d FIRST3PASS39.63s/native197+100/-101/raw-1/coexist768s plus187/52/player0/334/actualNO_CRIT joint inputs; THEN ignoredb48621910 PUSHED. Genuine6RED/6PASS76.80s0warnings includes positive/negative197, source52/187 preclamp, explicitplayer andNO_CRITsource+197. Minimal existingtarget_auras197+187 sum in sharedchance reader beforeclamp/334; no scalar/query/state/wire/RNG/HP/clocks/Guardian changes. Consumerd3a6b734e fresh ignored12GREEN70.07s/0warnings plus retained qualification3PASS39.63s (fifteen-case composite), only then twelve behavioral ignores removed; lead gate pending. Autoattack-only, not spell/ranged/generic197census/fullclass certification.
## 2026-10-19 — Bounded Telash live combat, no natural-field latency sample

[Live result](investigations/follower-data.md#bounded-telash-live-observation--2026-10-19): landed e27/sharedF9, published eyes and whole-party harness staging;360.47s/286samples, ten natural Frost Bomb attempts interrupted by genuine follower kicks, zero fields/trace. All three distributions n=0; reaction policy not exercised. Main thread near one-core saturation/retained1.4–14.9Hz, no stress/realtime claim.429-file SHA archive, owned runtime/window cleaned. No disabled kicks, fabricated fields or retry.

## 2026-10-19 — Azure live prerequisites and eye receipt

[Investigation](investigations/follower-data.md#azure-live-prerequisites--2026-10-19): native admission works; exact owned navigation disconnects entrance/Telash despite local coverage. Book/spell destination bindings remain explicitly unsourced; no blind rebake or gameplay traversal claim. Existing eye producer reloads134 displays/1340 poses, native1PASS; missing-row fragment awaits independent lead publication preserving existing rows/scales/mappings. Natural hazard latency n=0.
## 2026-10-18 — Herald point4 fall through DESCEND3

[Bounded fall](systems/boss-scripts.md#herald-point4-fall-through-descend3--2026-10-18): native qualification plus genuine Normal/Heroic missing50msfall and ordinary20538 threat1vs0 RED precede implementation3a96703c4. Targeted4/4GREEN70.06s; actual original-ray floor, TC analytic fall/integer-ms EFFECT completion, gravity enable and per-hit zero-but-retained threat. Source audit caught forced-floor endpoint: actual truncated-duration2RED55.69s→delete snap3d0fea7e5. Final enabled38/38GREEN314.51s at3d0fea7e5/master614c59b44,0ignored/no warnings; includes retained Gold/Nibbles/Gordoth/point controls and actual post-fall min1/no replay. LEAD gate pending; DESCEND4 and later lifecycle excluded.
## 2026-10-17 — NPC source NO_CRIT

[Source flag](systems/player-damage.md#npc-source-no_crit--2026-10-17): qualification04ef1a93f FIRST3PASS43.93s/native30838flags131072 level83→dummy80 HP3→3/3 after256ordinaryswings; positive52/290+334effect1 coexist768s. THEN ignored e9c68e588 PUSHED; genuine4RED/4PASS46.22s (flaggedbase/52/290/joint stillCrit, four controls pass). Minimal cached attacker metadata through existing tuple→snapshot→Attacker; source0 only, later187/334 retained, no query/state/wire/HP/RNG/clocks/Guardian changes. Consumerdf6575f78 fresh ignored8GREEN68.97s/0warnings plus retained qualification3PASS43.93s (eleven-case composite), only then all eight behavior ignores removed; independent lead gate pending. Foreign flagged victim/full tuned actor parity remain named proof boundaries.

## Source-qualified pacing fixture gap

[Bisect and native bounds](investigations/follower-data.md#pacing-fixture-qualification--2026-10-19): originalc759/P_a2 already fails3 affordability prerequisites; not a worker/threat regression. Current40d3/717 native100eligible endpoint footprints all≥2packs and unchanged23-pack entrance WaitReserve pass2/2 in68.09s/0warnings. Explicitly approved named ignore preserves exactly3test bodies/assertions; no stat/kit/budget/runtime tuning or claim ordinary first pulls are impossible. Final scoped26de10PASS/3namedignored47.31s/0warnings; check/fmt pass. Runtime pacing acceptance remains open.

## Worker-backed tank admission integration

[Route continuation and proof](investigations/follower-data.md#run-local-tank-observations--2026-10-18):57d6dd748/P_f9a merges worker master2e4dfb596; complete route publication gates movement, pending holds retain victims, native work keeps priority, and source/budget/field projection remains current. No synchronous runtime fallback or invented tolerance. Actual18planner/death +eight worker-request +nine hazard/reaction controls35GREEN245.77s; owned-eye Azure actual-kick/1041GREEN30.03s; check/fmt pass,0warnings. Initial merge compilation failure is not behavioral RED. Wide gate/provider remain open.

## Master-native forecast integration

[Forecast and targeted proof](investigations/follower-data.md#run-local-tank-observations--2026-10-18):8ed245c42 merges72ee66aa4/P_f9a9175 and repairs tb622 compile failures using actual source aura251/240 and production expertise divisor. Existing quantized native roll weights are available outside tests; no substitute roll or zeroed fields. Native12Stockade GREEN155.18s, owned-eye Azure actual-kick/104 GREEN34.67s, both0warnings; non-testcheck/fmt PASS. Lead gate and broader planner/provider remain open.

## Measured Overheat refusal origins

[Native origin/boundary proof](investigations/follower-data.md#run-local-tank-observations--2026-10-18):88b0309ac/P_a2b enumerates144authored origins, eight native-valid disposable kit/chase fixtures; four retain refusal/unengaged exposure/live12466 through ten50ms boundaries, zero movement violations. Native1PASS402.99s/0warnings, no input/prerequisite failures. Focused control uses measured20375694; positive20375738 remains existing defense. No runtime change/tuning; focused regression and stronger actual-kick frontal assertion pending.

## Actual destination-cone frontal RED and admitted point positioning

[Native proof/implementation](investigations/follower-data.md#run-local-tank-observations--2026-10-18): a3ea7ae49/P_a2b actual386660/104 pose1RED38.01s/0warnings exposes all4living other party members despite successful native70/205 intro/body/LOS/approach/cast and clear-pose controls. First104 goal now records before legitimate follower kicks, validates native recipient geometry/actual-body mutual LOS/ground, and projects full point-route exposure through unchanged admission. No eye defaults, stat/kit/budget changes or interrupt-provider activation. Native GREEN and broader frontals/stacking remain open.

## Azure frontal fixture and owned Leymor body input

[Source/proof/limits](investigations/follower-data.md#run-local-tank-observations--2026-10-18): native70/205 fixture reuses authenticated private2515 geometry read-only;512inputs/14outputs match. Startup relative-world setup failure is not frontal RED. Canonical eyes lack Leymor107127; existing algorithm4890aa236 produces10actual head-bone poses from model4279060/skin4279083, native1PASS0.06s/0warnings. Only owned fixture eye view changes, with algorithm/dependency/output hashes retained; no canonical writes/tile rebake or new eye defaults. Actual386660/104 pose RED still pending, native kicks enabled.

## Admitted caster pickup and native victim consistency GREEN

[Exact source/proof](investigations/follower-data.md#run-local-tank-observations--2026-10-18):6d0f1cf62/P_a2b native7PASS126.47s/0warnings: Overheat12466 pickup/ended-cast retirement, unchanged owner selection and matching actual tank victim, collector2 and pursuit safety/defense/injected4. Firstfb6 attempt preserved incompatible trash CombatTarget and correctly failed native chase; planner selection now matches its admitted caster instead of weakening that guard. Broader positioning/frontals and single-provider activation remain open.

## Native Overheat caster pickup RED and admitted chase

[Exact source/proof](investigations/follower-data.md#run-local-tank-observations--2026-10-18):6cc8a02c2/P_a2b actual12466/20375719 versus reachable20375738 gives genuine1RED55.53s/0warnings,14.209645yd unchanged. Complete native engagement/body/LOS/Rebuke controls pass; earlier exact floor-height assertion was a setup failure. One admitted caster entity goal now survives selected-trash kit pursuit through ordinary native chase; ended casts retire its chase. Targeted GREEN pending; no provider activation, budget or stat/kit changes.

## Tank baked-corner acquisition and corrected pursuit proof

[Exact proof/limits](investigations/follower-data.md#run-local-tank-observations--2026-10-18):36d7f43bc/P_a2b native5PASS160.19s/0warnings. All1353distinct interior native route points to20375689:35target-proximity refusals; remaining1235Reserve/83script refusals, no qualified corner. Native bodies/real LOS/stats and bounded other-source acquisition preserved. Corrected actual-tick pursuit4allPASS; Stockade policy3 geometry blocked, not behavioral RED. No budget/stat/kit tuning or full planner/gameplay acceptance.

## 2026-10-18 — Rewritten tank checkpoint and native facing integration

[Exact proof/limits](investigations/follower-data.md#run-local-tank-observations--2026-10-18):6c0aa→4df7 reset/masterac365 merge140879; pre-merge12PASS133.49s,99endpoint rows with only20375689 lower-bound allowance. Upstream melee facing requires shared chances-helper/forecast front-arc plumbing; numeric proof stays historical. Full authored-point routes20375689 only and current native/pacing verification pending; no policy/stat/kit changes.

## 2026-10-18 — Endpoint affordability proof prepared

[Proof and limits](investigations/follower-data.md#run-local-tank-observations--2026-10-18): lead approves origin-independent endpoint bounds only at measuredHPS0; explicit positive-HPS refusal, same native arithmetic and complete target table. Only passing endpoints require full authored-point routes. Inline pacing diagnostics removed; native execution/report and eligibility GREEN pending. No budget/stat/kit changes or global affordability claim.

## 2026-10-18 — Native trigger pull eligibility RED and repair

[Exact source/proof](investigations/follower-data.md#run-local-tank-observations--2026-10-18): real Stockade53488/20375809+20375810 candidate/proximity2RED44.05s. Route now uses native relation selection and cached template aggression, not faction hostility alone. Policy3 failed only positive-capacity prerequisites; no budget/stat/kit tuning or pacing acceptance. Eligibility GREEN and real-combat enumeration pending.

## 2026-10-18 — Pursuit proof corrected at new-pack boundary

[Native diagnostic and correction](investigations/follower-data.md#run-local-tank-observations--2026-10-18): c547 movement4 gives3PASS1FAIL;79aef first-drift route7 is wholly inside14 already-engaged sources, so freezing it would wrongly block defense. Lead-approved50f035 checks an actual unengaged-route tick and next fixed movement instead; corrected case and pacing3 remain unexecuted/capacity-blocked.

## 2026-10-18 — Production pursuit admission seam; owner-paced policy

[Exact boundary/proof](investigations/follower-data.md#run-local-tank-observations--2026-10-18): accepted admission10 remains aae/P029; production no-injected-position7a258 RED moves at tick2. Production route/source/budget refusal0e9 plus defense control and owner-paced idle/active policy fixtures are committed, targeted execution capacity-blocked. No goal-generation, positioning, interrupt activation or clear acceptance.

## 2026-10-18 — Run-local tank observations

[Native tank boundaries and proof](investigations/follower-data.md#run-local-tank-observations--2026-10-18): history, native forecast/lifetime, sourced healing capability, source/formation presence, full route exposure and admitted-point consumer. Production budget/goal generation and single interrupt-provider integration remain unfinished; no full-clear or gameplay acceptance.
## 2026-10-17 — NPC general crit290

[Combined raw source](systems/player-damage.md#npc-general-crit290--2026-10-17): qualificationa20fb904d FIRST2PASS48.51s, actual signed290/mask6 and full52+290 live768s; THEN ignored294a0606a PUSHED before genuine4RED/4PASS54.62s. Positive0Crit/5, negative/signed11Crit/256, joint52+290zeroCrit/256 prove missing NPC290. Reuse/rename existing accumulator to npc_crit_pct52+290, retain player StatModifiers fold/NPC-only consume and pre334clamp; no new scalar/query/snapshot/wire or RNG/clocks/HP/Guardian changes. Consumer794157734 fresh ignored8GREEN41.92s/0warnings plus retained qualification2PASS48.51s (ten-case composite), only then eight behavior ignores removed. Lead gate pending; player52/NO_CRIT/NPCspellcrit/mitigation excluded.

## 2026-10-18 — Herald min1 precommit source guard

[Min1 guard](systems/boss-scripts.md#herald-min1-precommit-guard--2026-10-18): real intro/direct/self/spell/native melee and ordinary real1s periodic probes reproduce six lethal commits; exactHP1/no death/credit six-case GREEN146.24s. Source state, unchanged Gold/Nibbles behavior, listener audience and opt-in ownerless event capture are explicit. Approved six synchronized copies/two native cadences prove same-actor repeated melee82.49s at2639aed02; enabled all16 Herald+four grounding+five Gold+two Nibbles27/27 PASS209.65s atcbb9ef32f after master69ed NPCblock51 merge; LEAD gate pending; release/arrival/fall/paired credit/reset/native excluded.
## Follower hazard trace implementation proof

[Opt-in trace](systems/navigation.md#follower-hazard-live-trace) joins native contact/reaction/hold/displacement/exit with current-ID worker publication and owned corridor adoption. Missing native events give a genuine RED; native trace/disabled-payload/reaction/FIFO/request14 controls pass, and three-distribution/policy/cancellation/cache/latest-ID analyzer7 controls pass. Owner client068066e6a/F9 native extension+CLI artifacts built without warnings. Bounded real Azure/Telash collection remains pending a granted server_perf/5761 window; no new runtime flags, gameplay changes or stress-capacity claim.
## 2026-10-17 — NPC weapon crit52 and shared clamp

[Signed52/clamp](systems/player-damage.md#npc-weapon-crit52-and-shared-clamp-ordering--2026-10-17): ignored00ef37b6d pushed first; native52/334 and separate player187/334 qualification2PASS56.14s survive768s. Fresh6 genuine RED/2PASS56.19s includes explicit player0-60187+50334 zeroCrit/256; source clamps before334. Existing raw AuraStatMods sum/NPC-only consumer plus shared clamp correctionbd3cf7db7 committed after RED; fresh ignored8/8GREEN41.04s/0warnings plus retained qualification2PASS56.14s (ten-case composite), only then all ten enabled. Independent lead gate pending. No RNG/clocks/HP/native-input changes; player52 masks/290/NO_CRIT/mitigation excluded.
## 2026-10-18 — Natural shared hearthstone charge coverage GREEN first execution

[Scoped final toy follow-up](../toy-effects14-coverage.md#natural-shared-hearthstone-charge-coverage-green-first-execution-2026-10-18): tt653211/0/check/fmt landeda989c0f3b; timer branch cleaned. Source2309 MaxCharges1/recovery1800000ms drives two-direction real learned281136/281615 tests: before30min refusal, ordinary recovery, full sibling10s cast/spend/original refusal. First execution7adfb7b03 GREEN4/0/78.45s(new2+retained2); existing machinery works, no behavioral RED/runtime repair. Normal50ms updates/observed completion bounds only; no runtime/data/clock/reset or native claim. Lead integration/landing pending.

## 2026-10-18 — Ordinary Loa timer coverage GREEN first execution

[Scoped follow-up](../toy-effects14-coverage.md#ordinary30-minutehour-coverage-follow-up-green-first-execution-2026-10-18): after docs landingec7df3660, two existing280820/280822 cases observe natural30min authoritative/replicated expiry and one-hour recovery/reuse through ordinary50ms updates. Independent cast-time bounds, no internal-deadline oracle or clock/cooldown mutation. First executione10b9562b GREEN4/0/79.19s: two new lifecycles and two retained cancellation cases. Existing timers work; no runtime defect/repair/fabricated RED. Lead integration/landing pending. Accepted sourced boundary/gaps and other native/full-duration scopes unchanged.

## 2026-10-18 — Accepted toy sourced boundary and three named gaps

[Current accepted matrix](../toy-effects14-coverage.md#accepted-sourced-boundary-full1166-plus-seven-2026-10-18), [contract](../specs/toys.md#accepted-sourced-boundary): Full1166+seven published extras complete to current sourced boundary; ty639209/0/check0/fmt0 landed69ed3c682. Read-only batch4 research found no complete action contract for Aqir275039, Libram284159 or Quill286643; all accepted named gaps remain outside Toy membership. Numeric143823 is not a Glyph reward binding, and Quill's real scenario92 criterion is not its full party teleport action. No code/data/import/extraction change; native/full-duration proof remains separate.

## 2026-10-18 — Batch2 landed; footstep70448 pair admission RED

[Current matrix](../toy-effects14-coverage.md#expanded70448-footstep-cosmetics-canonical-cpu-green-2026-10-18): explicit tb625 stationary landing641cb2460, ancestry verified/feature branch deleted. New Halazzi/Jan'alai real-learning RED0/2/54.36s; lead exact twoToy-row and40file/57path publication; canonical aura4 RED0/2/30.71s. Two strict named admissions and same-branch census triples retain Full1166+seven; canonical1e5e55ae0 pair/all13 census/previous-family/absent-visual GREEN22/0/76.90s; lead broad gate/source landing pending. Absolute canonical shared-protocol patch resolves stale native-root interface without protocol/cache mutation; no native footstep/particle claim.

## 2026-10-17 — NPC victim block51

[Block51](systems/player-damage.md#npc-victim-block-aura51--2026-10-17): ignoredcd95 pushed first;10021+62356effect0 sum-100live8s/49=0 and62627fold qualified1PASS26.18s. Live+100→0Block4Melee1Crit andsigned-100→1Block4Melee give genuine2RED/5PASS64.12s. ExistingNPCguard/target_auras consumer+max0 without240subtraction; masks/direction/playerStats/blockedamount253/Guardian/RNG/clocks unchanged. Consumer28638df38 fresh ignored7nativeGREEN40.77s0warnings+retainedqualification1PASS26.18s (8composite,notone8batch), onlythen alleight enabled/pushed; lead gate pending.

## 2026-10-18 — Follower hazard reaction project policy

[Policy and root cause](systems/navigation.md#follower-hazard-reaction-policy): master10PASS/0FAIL versus worker0PASS/10FAIL exposes field growth overtaking an exhausted corridor while the next batch is Pending. User accepts late-worker standing and defines50ms authored-radius prediction, immediate async request,250ms reaction and first-ready-tick movement. Three pushed behavioral REDs become GREEN; selected published candidate corridors eliminate a second worker roundtrip. Existing Azure oracle now observes predicted escape before actual aura cleanup. At6b374c251/protocolf9a91753, requested Azure4/hazard-approach7/reaction2/pet11/FIFO1/Gold8 passes33/0fail/0ignored in72.51s with no compiler warnings; lead merged-master wide/check/fmt/worker gate remains pending. This is project policy, not sourced Retail timing.

## 2026-10-18 — Controlled-pet navigation request ownership

[Route pruning](systems/navigation.md#navigation-worker-migration--2026-10-18): real controlled pets lack CreatureState, so filtering on that AI component cancelled their live routes. Pushed ignored real-spawn RED d0b176d52 fails at the live FixedFirst boundary; production pruning uses the requesting entity's Position for every owner, preserving optional InMap metadata and existing map/teleport checks. Live-route/despawn control and both unchanged Stockade finish controls pass3/0fail/0ignored; the new control is enabled afterward. Master b390e37b2/protocolaa6d925 passes these two existing routes; Bite focus76vs75 also fails master and is excluded from this branch. The three surviving scripted failures publish actual no-walkable-polygon errors at their offset fixture targets: the Gold-era global NavigationPlugin addition loaded real bakes into tests that had never opted in. Remove that global fixture addition, retain the existing opt-in test-only drain, and keep all assertions/positions unchanged. Final a1cae7031/protocolaa6d925 passes all15 requested controls (three scripted, three pet routes, eight Gold and actual FIFO) with0fail/0ignored in72.46s; broader lead gate remains pending.

## 2026-10-18 — Navigation worker all production callers converted

[Migration/proof](systems/navigation.md#navigation-worker-migration--2026-10-18): hazard approaches and eight-candidate escape batches, fear collision→main LOS→path, and ground C++ waypoint controls now use current-ID worker continuations. Removed production NavigationMap/mutex/synchronous APIs; test-only inspections explicitly drain the worker. Three raw-decision REDs became GREEN. Native growing aura395 exposed candidate-identity starvation; capture one batch through completion and revalidate against current fields, retaining the unchanged test and radius/policy/timer/RNG. Post-master-eedd9a887 server6ab0b0a3d/protocola2b07a6 passes17/0fail/3existingignored; scoped worker dispatch recheck6/0fail/0ignored. Lead owns check/fmt/broad gate. Sight/faction CPU and live performance acceptance remain separate.

## 2026-10-18 — Navigation worker and advance caller group

[Worker migration](systems/navigation.md#navigation-worker-migration--2026-10-18): actual held-tile FIFO RED atf52 stalls before=1/held=1, with release/path/cleanup assertions passing. Worker/types4f41 plus receiver-publication ordering6a1 pass6 native library controls. Navigation::advance now queues immediately, installs current-ID routes at FixedFirst, retains old movement while pending and refreshes request feet; map/teleport/despawn cancellation and no-path timer pause are in scope. Server c2bd6e38f merged/protocola2b07a6 selected. Route integration controls are newly implemented, not yet passing evidence; fear/hazard/C++ raw callers still synchronous. No push or universal ten-second fix claim.

## 2026-10-18 — Navigation worker design and sight CPU gap

[Design](design/navigation-worker.md) records approved pending movement/latest-request/fixed-order/error/test-drain behavior, exclusive map ownership, complete synchronous caller inventory, per-entity queue bound and explicit worker-death failure. Base83a8ce176/protocol029a927; no Rust implementation or RED yet. [SIGHT_FACTION_MAIN_THREAD_CPU](investigations/server-tick-lag.md#sight_faction_main_thread_cpu) records9of12 B partial stacks and sampled A/B CPU counts, with uncaptured maximum-gap stack and load/observer limitations. Navigation reads are a confirmed contributor, not sufficient evidence alone for historical ten-second stalls.
## 2026-10-18 — Scarab target32 first-destination oracle

[Scarab oracle](systems/toys.md#scarab-target32-grounded-oracle-2026-10-18): native92146 SUMMON28/32/noFORCE correctly grounds; pre-grounding oracle REDbe5cd97ae39.32s. Corrected identical pre-clampY ray/strict floor bits, retained count/scatter/follow/lifetime; strict oracle+retained controls17/17 GREEN144.23s at206b095ed, independent LEAD full-Toy gate pending. Production unchanged.
## 2026-10-17 — NPC victim parry47

[Parry47](systems/player-damage.md#npc-victim-parry-aura47--2026-10-17): ignored49f20 pushed first;23547/64651 signed0live8s andactualSunwellhit5/dodge-20 qualified1PASS41.59s. Front+100→5Damage/0Parry andnegative-100→1Parry/4Damage give genuine2RED/6PASS47.04s. Existingtarget_auras/NPCguard consumer before240/clamp; masks/direction/playerStats/block/Guardian/RNG/clocks unchanged; Consumeraa2ee70c1 fresh ignored8nativeGREEN34.40s0warnings+retainedqualification1PASS41.59s (9composite,notone9batch), onlythen allnine enabled/pushed; lead gate pending.
## 2026-10-18 — Stationary70448 cosmetics admission RED and scoped repair

[Source/RED/publication matrix](../toy-effects14-coverage.md#expanded70448-stationary-cosmetics-canonical-cpu-green-2026-10-18): rebase761→3ded411f0 onto rewrittenac36572bb with verified new ancestry. New-master learned-use RED0/3/78.01s; lead's exact threeToy INSERTs/34fresh-CASC files published; canonical aura4 admission RED0/3/59.41s. Minimal three-ID strict cosmetic admission plus same-branch published-census triples; canonical81c554bb1 GREEN20/0/151.90s includes all13census, prior3batch1 and absent-visual negative. Broad lead gate/source landing pending. Source30min/1h/cancel contract only; no genericdummy/native/full-duration/other-five claim.
## 2026-10-18 — Herald own nonlethal below30 ingress

[Own30](systems/boss-scripts.md#herald-own-nonlethal-below30-ingress--2026-10-18): TC217–220 actual35→29 after intro reuses the descent latch, no30693; natural Normal/Heroic/INTRO1 negative RED1PASS2FAIL→GREEN3/3 62.04s. Real source-speed first step and cast interruption; lethal guard/arrival/fall excluded. Enabled all10 Herald+four grounding14/14 PASS172.99s ataed52b5b6; LEAD gate/check/fmt pending.
## 2026-10-18 — Player-only native80 base-stat stage

[Native80](systems/auras.md#player-base-stat80-stage--2026-10-18): normal9417 known-passive +10Int producer proved, no fakeflatstat/gear. Ignored989f0a010 gives3controlsPASS/4genuineFAIL34.97s: nativeMage38vs41.8 and48vs51.8; actual controlled NPC bridge stays unchanged. Separatebase-percent field beforebonuses/137 plus actualPlayerguard; GREEN pending, no legal-kit/full13326/global80/NPC-kit certificate.

## 2026-10-18 — Source-qualified follower137 oracle

[Shared137 consequence](systems/auras.md#independent-primary-stat137-composition--2026-10-18): real native Garrick initialization/removal-reapply/evade3FAIL84.42s exposed old35%/10584 expectations. Authenticated independent Aegis30%/Plate5% require36.5%/1337.7stamina/10696HP; unbuffed5%-only8232 unchanged. Test/spec oracle correction only, no follower runtime/health/stat/budget override; targeted GREEN pending. Historical native10584 capture receipts remain unchanged as observations.

## 2026-10-18 — Independent native137 composition

[Source-gated137](systems/auras.md#independent-primary-stat137-composition--2026-10-18): completeTDB1210 groups/rules +all32pinnedworldSQLupdates establish independent51271/53365. Ignored8aef2fabb RED3FAIL/3controlPASS18.87s shows real HumanDK Strength33→44.55vs45.54 bothorders/removal. Minimal percentage-representation fold preserves flat29/182 addition/masks. At853447542 native6 +retained23 GREEN29/29,46.49s,0warnings/ignored; stats substring also covers item/frame families. Fullcargo fmt0, independent lead gate pending. No fake stats/source/clock edits or legal-kit/runeforge-proc/fullPillar/group-engine/PvP certificate.

## 2026-10-18 — Native continuous power prevention294

[Native masks](systems/auras.md#native-power-prevention294--2026-10-18):232865/262173 amount0/masks8/1; TC prevention before base/%/flat. Actual Elixir/WaterShield RED3FAIL/3controlPASS16.93s,0warnings at3041d69e3 pushed first. Minimal recipient-mask Mana/Energy consumer; Rune/discrete-pet/other/global294/legal-build/full-ability claims excluded. Combined46GREEN21.88s at7e541f03b; tests-only416735d34 explicitEnergy prediction0 assertion has fresh6GREEN19.39s,0warnings. Retained40 unchanged; fullcargo fmt0, independent lead gate pending.

## 2026-10-18 — Water Shield flat Mana

[Native79949](systems/auras.md#water-shield-flat-mana--2026-10-18): TC base-mana multipliers then85/5; actual8004/137039 spend and ordinaryShield RED125vs127 in two cases,3controlsPASS25.33s atf4c17e35b. Initial harness-only failure excluded. Minimal Mana consumer; sibling79508→79950 unchanged. Merged82f8476d4 native5 +Elixir5 +regen/resource30 GREEN40/40,26.47s,0warnings; fullcargo fmt0/cleanstatus, lead independent gate pending. No Retail52127/legal-build/full-proc claim.

## 2026-10-18 — Energizing Elixir flat Energy

[Native115288](systems/auras.md#energizing-elixir-flat-energy--2026-10-18): source75Energy/5s AFTER percentage modifiers, real Tiger Palm deficit and ordinary Elixir RED10vs25, native13750 RED17.5vs32.5; sibling2Chi preserved. Tests-only ignored3993b7fe8 pushed first;3controlPASS/2genuineFAIL26.42s. Minimal continuous non-mana85 aggregation/consumer atc62d61e9b: native5GREEN14.48s +retained regen/resource30GREEN14.31s,0warnings; independent lead gate pending, no legal-loadout or generic85 certification.
## 2026-10-17 — NPC victim dodge49

[NPC49](systems/player-damage.md#npc-victim-dodge-aura49--2026-10-17): ignoreddf529 pushed first; actual5277+62356effect1 sum0 across8s/block51=0, qualification1PASS33.62s. Nativefront/behind each5Damage despite live200, genuine2RED/4PASS35.27s. Existingtarget_auras consumer onlyNPC/allowedhit path before251/240/clamp; no playerdoublecount/otherdefense/Guardian/query/wire/RNG/clock change. Consumere6e99fb6e fresh nativebehavior6/6GREEN57.40s0warnings +retained qualification1PASS33.62s (7composite,notone7batch), onlythen allseven enabled/pushed; lead gate pending.

## 2026-10-18 — Hunter-pet Focus interval

[Focus timing](systems/combat-pet-lifecycle.md#hunter-focus-timing--2026-10-18): exact unchanged Bite RED76vs75 and native boundary RED76.25vs75; cost25 authenticated, pinnedTC24/4000ms spawn/full-pool phase consumer added only for Focus pets. Player/demon paths unchanged; merged3cc539edd exact originalBite/new interval/player5/s3GREEN62.27s + retained lifecycle/autocast/Imp4GREEN172.48s,0warnings; lead combined/check/format gate pending.

## 2026-10-17 — Character-owned recipe Favorites

[Professions](systems/professions.md#character-owned-recipe-favorites) adds known-spell-only preference requests and a character-keyed redb multimap without changing existing profession record encoding. Existing owner snapshots carry current favorites; request/refusal and reopen/isolation proof is pending targeted GREEN and the coordinated consumer gate.
## 2026-10-18 — Grounding-only target32 and natural INTRO2 GREEN

[Bounded contract and proof ledger](../specs/ramparts-herald.md): model-free Z-only reader preserves XY and missing-height casts; full reach-dependent port remains blocked by122612 metadata. TC709–748 source-height query is made once with pre-clamp Y; traced identical-input rays are bit-stable, while a post-clamp-Y ray is a different query. Approved exact source-ray oracle uses finalXZ/pre-clampY and strict bits, no tolerance or WMO-reader changes. Grounding plus full follower-pet and retained controls pass; Normal/Heroic natural first-West and actual facing bits pass after master6c8929589 merge. Future source engage events remain scheduled/unasserted; later lifecycle/native/XY/collision/hover-swim/backoff gaps explicit. LEAD independent gate pending.

## 2026-10-18 — INTRO2 rewritten integration / target32 grounding blocked

[Source transition](systems/boss-scripts.md#herald-intro2-first-west-boundary--2026-10-18): reset rewritten870659102, merge exact masterac36572bb including target32 revert; no port resurrection. Standalone copy-only prep and shared test visibility/difficulty API preserved without changing defaults. After excluding compiler/setup failure, ff1502383 gives4PASS/1genuineFAIL50.82s: exact actual-finalXYZ source floor89.05998 versus80.80767, no ULP tolerance. Normal initial transition/child and copy/duplicate/INTRO1 checks pass; Heroic/first-West assertions not reached. LANTHA_DISPLAY_122612_MODEL_INFO_ABSENT blocks returning target32 port, not an accepted runtime gap. No summon-position/admission workaround, no ready/full-encounter claim.
## 2026-10-17 — NPC autoattack expertise

[NPC240](systems/player-damage.md#npc-autoattack-expertise--2026-10-17): ignored2abb pushed first; native28305 singleapplication qualified14 across1536s,1PASS32.88s (post-command wrapper error, no invented exitcode). Genuine Rogue5DodgeRED plus cancelled/foreign2PASS; two owner-log fixture failures corrected to actual CombatEvent broadcasts without gameplay changes. Ignoredb1c2 native14/10000 give genuine2RED66.39s; minimal NPC-only240/4 dodge/parry consumer, block/Guardian/RNG/clocks unchanged. Consumer925224b2a fresh behavior5/5GREEN70.24s0warnings+retained lifetime1PASS32.88s (composite6,notone6batch), then allsix ignores removed; lead gate pending.
## 2026-10-18 — Frozen Ground fg602 landing and world-delta qualification

[Qualified proof](systems/spell-casting.md#ordinary-frozen-ground--2026-10-18): independent fg602359PASS/0FAIL/14namedignored1998.44s includes fresh3 Frozen Ground cases; check/fmt0. Lead channel671 confirms08e3→b24e is only3Toy INSERTs, so retained3PASS stands without replay; original guard failure preserved. After push-race reconciliation, landed361efa1d2 with post-merge check/fmt0. Deployment started per lead; no live-dungeon/canonical-geometry-publication claim from this slice. Historical2+1/observer and input-provenance boundaries remain recorded below.

## 2026-10-18 — Frozen Ground native2+1 and controls qualification

[Current matrix](systems/spell-casting.md#ordinary-frozen-ground--2026-10-18):13d7 Player periodic/slow/lifecycle+curve/30s2PASS;5730 genuine follower escape/next-membership cleanup1PASS118.14s, no source scheduling change. Retained Rune/Sprout/curve3PASS91.01s but08e3→b24e world guardfailed; source/protocol stable, no automatic rerun or clean-controls/final-gate claim. History rewritten b548→867c, then exactac365 merged; originalRED content proof preserved.

## 2026-10-18 — Ordinary Frozen Ground source field

[Finite387149/26505 binding](systems/spell-casting.md#ordinary-frozen-ground--2026-10-18): accepted real Frost-expiry aura/geometry RED;4LOS/4M2-floor/4nav private2515 bake with explicit mixed input provenance. Source action/curve/lifetime implementation committed before GREEN; downstream native follower escape/lifecycle proof pending, no arbitrary179/Mythic/VFX/full-Azure expansion.

## 2026-10-17 — Attacker251 / Guardian merge

[Attacker251](systems/player-damage.md#autoattack-attacker251--2026-10-17): rewrite maps old035ae5cfb to de08aab30, identical tree; original proof labels retained. Merge masterac36572bb preserves Guardian same-tick try_swing one-shot health commit and251 scalar/roll consumer; requested original9 plus Guardian/Ardent integration proof pending.

## 2026-10-17 — Autoattack attacker251

[Attacker251](systems/player-damage.md#autoattack-attacker251--2026-10-17): TC Unit.cpp2389/2775 signed caster modifier. Ordered ignored d5587d263 native2RED/4PASS87.19s (negative20Dodge; positive20Damage/0Dodge); minimal transient snapshot/roll consumer preserves gates/clocks/RNG. At1dbe19063 explicit ignored6/6GREEN110.87s,0warnings before enabling allsix. Master7a0f00fc0 integrated e4d9c1efd: ordinary affected cohort122PASS/3legacyLOSignored154.21s,0warnings, all6newnative enabled. Lead gate pending; expertise/248/Totem/flags not certified.
## 2026-10-18 — Published70448 extras census oracle correction

[Current regression/correction](../toy-effects14-coverage.md#tg578-census-regression-named-baseline-and-published-extras-2026-10-18): tg57812 and st57413 census failures come from one stale1166 assertion after twoToy-row publication. Shared oracle now retains Full1166 plus the two explicitly published item/name/spell triples; future batches append the list after publication. Exact748a5ad78 merge, no runtime/data change, batch2 RED isolated; requested13census+3batch1 targets pending.

## 2026-10-18 — MaxCount landed; expanded70448 hearthstone RED to GREEN

[Current landing/batch1 matrix](../toy-effects14-coverage.md#mc548c-maxcount-landed-expanded70448-hearthstone-batch-green-2026-10-18): explicit lead mc548c receipt lands MaxCount and retained three-item family proof; earlier pending labels are historical. Two newly scoped hearthstones have real source dependencies but no Toy membership: actual learned-use RED0/2/56.76s. Selected producer3808fe58f preserves unrelated rows, RED0/2→GREEN2/0; copied-world diff is twoToy INSERTs,16 local-CASC audio files authenticated. After lead's byte-identical diff/fresh-CASC publication, canonical GREEN3/0/71.90s covers both hearthstones and retained5efec9c6f case. Direct canonical row/audio hashes agree; broad tg/toy gate/source landing/native/full1166 still unclaimed.

## 2026-10-18 — Native melee facing and actual direction override

[Melee facing](systems/player-damage.md#melee-front-arc-and-direction-override--2026-10-18): ordered ignoredREDdeb2168f0 pushed first; actual31144/equal80/256behind9Parry+11Block and actual5277/200/10s/fivebehindDodge give genuine2RED31.66s, front+native97574/288 controls4PASS. Existing shared front-dot/rotation conversion now feeds melee: behind noParry/Block, player-only noDodge, NPCdodge retained. Source masks/CC/RNG/order/clocks unchanged; at64483bb3a native6/6PASS37.81s,0warnings in explicit ignored execution, then allsix ignores removed. Merged17f061dc5/master41e386c4d discovery/affectedcohort109PASS/2FAIL/3ignored78.29s, all6nativecasesPASS. Two old front-defense fixtures actually faced away; only their orientation preconditions are made explicit, retaining masks/CC/stats/RNG/counts. Corrected5b214cc4f control modules7/7PASS36.87s,0warnings; retain104 disjoint17f passes=111 scoped executable GREEN,3legacyLOSignored, no duplicate111 replay. Latermaster748a5ad78/5322 merge is disjoint PVP-port handling; final checks/lead gate pending. Prior NPC flags nf570 EXIT/check/fmt0 LANDEDcb55b98c6; no fullhit-table/UDP/class claim.

## 2026-10-18 — Target32 invalid-height repair / gate tg571

Gate tg571 exposed eight missing-height summon regressions. Merged serverf0f9b31d6/protocola2b07a6; real688 RED1fa8ed113 captures exact NotSupported. e674cf7c0 follows TC Object.cpp701–706/732–748/761–762 and FORCE2808–2829: no invented height and no missing-height cast failure. New688 absent/empty-ground control plus unchanged consumers/primitive batch22/23 GREEN168.86s. Remaining Lantha30146 is refused at completion before spawning: actual display=None/reach=None in incomplete generic-NPC fixture. Diagnostic removed; existing tests unchanged; gate remains blocked, not full GREEN. [Contract](../specs/caster-summon-destination.md).

## 2026-10-18 — Caster summon destination32

[Primitive](systems/spell-casting.md#caster-summon-destination32--2026-10-18): distinct source reach/front-left selector, represented dry-floor/flight adjustment and separate FORCE backoff. Genuine XY/below-floor/FORCE REDs precede a2373c0c5 targeted7/7 GREEN49.47s, no warnings; three primitive cases enabled afterward without behavior changes. Lead owns check/fmt gate. Exact geometry/hover/swim/height residuals and unchanged42/44/47/49/50/72/55 remain named; INTRO2 is separate.
## 2026-10-18 — Post-guild merge codec and MaxCount targets GREEN

[Exact current matrix](../toy-effects14-coverage.md#post-guild-merge-codec-and-bounded-maxcount-proof-green-2026-10-18): after explicit gr567 landing, protocol029a927/server41856b72b/enginef26cc691e merged; protocola2b07a6 wire2/0+combined-discriminants2/0, server33c9d2eb2 five+MaxCount17/0/47.77s, engine6c00abbf4 dispatch1/0. Old tags preserved/new ItemMaxCount21 and SetMemberRank13 bincode+JSON round-trip; both wiki-index sides retained. No source data/weights/oracle change or broad/landing/native/full1166 claim; lead mc548b rerun pending.

## 2026-10-18 — mc548b five-regression recovery targeted GREEN

[Current proof/limits](../toy-effects14-coverage.md#mc548b-five-regression-recovery-targeted-green-rerun-pending-2026-10-18), [roots/repairs](systems/inventory.md#mc548b-regression-recovery--targeted-only-2026-10-18): mc548b771/5/13ignored, exact requestedcfdf merge356d4493a/protocol999b52e reproduces0/5; f97c118e4 passes5/0/25.95s. One shared Inbox consumer repairs three item-use helper double-drains; production retains real metadata and makes empty grants/capacity checks no-ops. Original expected counts and real MaxCount/source/typed-error guards unchanged. The three-item2513 test individually passed REDmc548b; no aggregate-green inference. No own broad rerun or combined landing/native/full1166 claim; lead rerun pending.

## 2026-10-18 — Gate548 MaxCount repair targeted GREEN; combined gate pending

[Current Toy proof/limits](../toy-effects14-coverage.md#gate548-maxcount-defect-coordinated-repair-targeted-green-integration-pending-2026-10-18), [authoritative mechanism/source matrix](systems/inventory.md#ownership-maxcount--implementation-and-targeted-proof-2026-10-18): sourceMaxCount/shared stores/passive personal-bank count, quest direct-push migration and typed regular-cast owner error implemented. Targeted model5/0, quest3/0, Doomsday4/0, protocol1/0/client-dispatch1/0. Source Spell.cpp7648–7656 corrects regular second-use semantics; triggered-effect cap and genuine-full error controls pass separately. Three-item2513 batch retained/compiled but behavior-unexecuted. No data/weights or native presentation changes, no own broad rerun; lead combined three-repo gate/landing pending.

## 2026-10-18 — Three-item2513 cancellation/recovery batch pending

[Exact batch/limits](../toy-effects14-coverage.md#three-item-batch-saptor685-idol702-badge710-cancellationrecovery-pending-after548-2026-10-18):604c35311 batches Saptor685/Idol702/Badge710 learned-use/source-model/one-hour aura, real cancellation/restoration/no2513 bypass and ordinary600s Pango reuse. Original fixtures and weights unchanged. Format/manual only, uncompiled/unexecuted AFTER548; no Masquerade cancellation/hour-expiry/exact-wall/native/distribution or runtime/data claim, no own jobs.

## 2026-10-18 — Doomsday419 ordinary recovery/product cap pending

[Exact acceptance/limits](../toy-effects14-coverage.md#doomsday419-ordinary-recovery-and-unique-product-repeat-pending-after543-2026-10-18):cb46a33f9 extends learned202309/398572 with first persisted202310 held across ordinary300s recovery, early refusal and completed second root while preserving original GUID/one unit. TC `SpellEffects.cpp:1426–1440` stores only allowed quantity, no invented cast rejection. Original first-product/Offering tests unchanged. Format/manual only, uncompiled/unexecuted AFTER543; no runtime/data/clock/inventory mutation or product-use/native/bank/account-cap claim.

## 2026-10-18 — Gate543 GREEN/landed Masquerade697

[Exact proof/landing](../toy-effects14-coverage.md#gate543-green-and-landed-masquerade697-cast-boundary-2026-10-18): lead receipt/ancestry confirmsa8b96f8d9 on master3accdf1a1;543server318/0/1ignored635.32s/EXIT0/FMT0/CHECK0, exact697 learned pre/post-cast test OK. No exact timing/interruption/hour-expiry/native/all-five/full1166 certificate or duplicate jobs.
## 2026-10-18 — Source-cached NPC autoattack avoidance flags

[NPC flags](systems/player-damage.md#npc-avoidance-flags--2026-10-18): native Ice Tomb36980/80/256 real swings genuinecfb1638ac RED15Parry/7Block, unflagged31144 controlPASS; initial incomplete native class-level fixture is not feature RED. Copy flags at shared EntryProjection, read cached metadata in existing roll, remove only NO_PARRY0x4/NO_BLOCK0x10. Missing-cache NPCs remain attackable with prior avoidance and once/entity explicit error, not query exclusion. Atb3b6e49a5 native flagged/unflagged outcomes pass; actual spell-summoned wolf and Call Pet each genuinely lack the source cache, so private SummonTemplate and pet spawn metadata now carry it with no extra lookup. Initial absent ordinary Kobold placement is invalid setup, not feature RED; use the actual49869 placement/ground data. At357c32137 native flags/cache/producer plus player-defense/pet/Dummy12/AP/table controls57/57PASS79.65s,0ignored/0warnings; only common fixture repair attaches actual native Kobold6 flags while preserving caller WorldData. Current1be6c12b7 server-bin check37.50s/fmt2.38s pass warning-free; manual changed-Rust readability/diff whitespace pass, metrics unavailable. Master41856b72b/protocol029a927 integrated; independent lead gate pending. Once/entity logging and same-actor entry cache refresh retain code/wiring proof only; aura251/facing/IsTotem/other flags and nonattackable battle-pet companions deferred. No wire/RNG/clock/native-row edits.
## Telash AfterCast return/counters

[Native return](systems/spell-casting.md#telash-aftercast-return-and-counters): cc62/first source attempt retracted as invalid SpellGo/owner-only-log oracles; source2da reverteda510, history preserved. Forward453/protocola2 then corrected5f08 actual-motion/CastState RED1PASS/3genuineFAIL75.36s/compile5m12s; no-target still stops at action prerequisite. Exact source reapplied only after corrected RED ascae7def32: fresh4GREEN/0ignored260.65s, compile3m12s, stable453/a2/ce source guards; all3 endpoints/real714 resumption/native3-2 clocks/next center/no-target/no-victim pass. Lead accepts4 and owns retained natural/strip/Frost/check/fmt gate. Source Icy2000+3000ms, addon bars corroboration only; no synthetic arrival, gravity reset, HP/resource priming or full-Azure claim.

## Absolute Zero Rune strip

[388008 effect1](systems/spell-casting.md#absolute-zero-rune-strip): separate9efa1PASS(cancel)/1genuineFAIL(205 completed strip)56.08s/compile2m11s, qualified ordinary activation/Player entry/native8s cast/exact hits/real damage;23 downstream. Ordered388065 then390201 handler only, no synthetic unoccupied shield producer. Production2d2996e47 merged required418 as26c397763/protocol029: fresh native strip2+retained activation2 GREEN4/0ignored116.75s, compile3m14s, stable source/HEAD/protocol/worldce guards. Completed205+23, occupied-only Mythic consequence and cancellation pass. Independent lead gate owns check/fmt; AfterCast/return/counters excluded.

## Vault Rune phase activation

[Native390209](systems/spell-casting.md#vault-rune-phase-activation): separate test-onlycd90c9091, ordinary60-to100 Energy phase and exact source-selected Rune hit sets; native2 missing388065 RED77.10s/compile3m18s, pinned protocol4be/worldce. Handler only skips390238 and makes the Rune cast388065 with implicit self. Production305512cf7 freshly passes native2+retained natural3/Frost2:7PASS258.97s/compile1m57s; actual owner/unchanged inactive/copy controls. Scoped check22.51s/fmt4.31s pass; guards stable, independent lead gate pending. No strip/AfterCast/return/counter/ordinary-budget/full-Azure claim.

## 2026-10-18 — Direct member-rank assignment

[Authority](systems/guild-lifecycle.md#direct-member-rank-assignment): source TC1894–1930 direction/self and source UI hierarchy/index mapping qualified first. Actual typed-request RED2/2 refused legitimate GM/multirank and promote-only assignments; earlier snapshot import compilation failure unclaimed. One direct validated rank/member transaction, recipient refresh and no synthetic step events; existing explicit step history preserved. GREEN/transport/joint/native proof remains pending.
## Vault Rune scripted field implementation

[Native Rune](systems/spell-casting.md#vault-rune-scripted-field): separate native388072/FRIEND action from exact scripted callbacks; general unit membership, full-set publication, actions before Enter/Exit hooks, OnRemove before clearing. Field REDd471, entry REDf034 and action/missing-field REDce21 retained separately. Existing aura87 handles native-50/mask127; Native first4 atf9e7 PASS93.75s, test-onlyd208 other5 PASS201.03s, explicit9-case composite; affected395/Sprout9 PASS201.94s, scoped check2m38s/no warnings/fmt PASS at docs-only80ea, stable source/protocol/DB guards. Lead ru561 passed/landed2f1ab6c53. No activation/strip/return/visual or full-Azure claim for that field slice.
## 2026-10-18 — Short-window attempt oracles and native Northshire fixture

[Fixture boundary](systems/creature-aggro.md#aggro-on-sight): test-only329bd18a6 requires real attempts instead of landed melee damage in two pet cases and both Northshire directions; Meredy preserves authored133 CastSuccess rather than accepting an unrelated melee hit. Unmodified master03a4e6d14 reproduces missing ModelReaches/Idle-before-Pursuing; separate1a9c633d2 loads existing read-only native geometry, no constants/flags/clocks/RNG edits. Five affected/neutral controls pass21.24s; hostile Northshire is un-ignored only after this proof. Six longer samplers remain unchanged. After merging344413893, ordinary discovery runs the newly enabled hostile case1/1PASS20.91s; current server-bin check65.78s/fmt4.12s pass warning-free at5c2e9b4df. Pet control's18 cases separately pass66.94s; no full-suite or new random-feature RED claim. Independent lead gate pending.

## 2026-10-17 — Generic periodic deadline origin/ordinal

[Anchored deadlines](systems/auras.md#application-anchored-periodic-deadlines--2026-10-17): independently sourced native436117 A3/200ms/3sec/2damage over four real lifetimes gives genuine7cf3a9906 RED[14,14,14,14] versus[15,15,15,15],5.36s. Private application/area-origin plus next ordinal now derives deadlines in integer-millisecond product units before seconds conversion; repeated +=period/cache removed, existing refresh/ExtraInitial/area phases retained. No added epsilon/clock/data/count/protocol/callback change. Developmente5ed121f6 generic4-lifetime plus refresh/ExtraInitial/phase/area-pulse/rolling-budget controls pass8/8 in4.55s,0ignored/0warnings, native counts15each. Generic prefixd5403b2e9 is independently pushed before reapplying the named hook. Union blast1548PASS/9FAIL/2IGN1136.50s; all nine pass after landed follower enum/master and explicit prepared-world/ground env. Combined native/compatibility/generic19PASS60.03s includes original40sec repeat10+10; prior-failure/geometry/Singed13PASS41.98s. Plain master3accdf1a1 five requested env comparisonsPASS15.80s; later economy/toy-test/docs merge retains disjoint proofs. Server-bin check23.65s/fmt2.33s pass atc80db3cac without warnings; manual changed-Rust audit/diff whitespace pass, metrics unavailable. Independent lead gate pending; up-front Fel source conflict stays deferred. Initial signed/unsigned test compilation error is not featureRED; named ignore removed with implementation.
## 2026-10-18 — Herald Sentry INTRO1 implementation

[Bounded INTRO1](systems/boss-scripts.md#herald-sentry-intro1--2026-10-18): ordinary GUID/patrol and real lethal/copy controls reach genuine2RED21.34s at7844943bc after excluding missing-waypoint setupFAIL. Added transient copy counter, source1730700 once-only AI, stop original patrol and actual center point movement. Targeted GREEN pending; no INTRO2 stub, morph/summon/flight/phases/credit/reset/native claim.
## 2026-10-18 — Native attack restriction264

[Native264](systems/auras.md#native-attack-restriction264--2026-10-18): actual Turtle producer yields wrong Pacified priority; native Phalanx NPC with264/no25 still swings. Genuinebc5a0c05e RED2→312234334 consumer; native/retained22GREEN, fresh-master4457dcfe9 expanded24/24 GREEN7.06s/146566ms. Eight new cases exercise real attack predicates, Cheetah/Fireball, midcast gain, natural expiry, full-trigger damage and NPC swing removal; real263 family controls retained. No row/clock/RNG/budget/producer/wire changes; no264 positive offensive-family input qualified or whole-ability claim. All-target check107.438s/0warnings and fmt/manual Rust readability pass.
## 2026-10-18 — Anvil2483 deferred with Tea Party2064 OverrideIndex1 gap

[Single gap](systems/pet-battle-engine.md#anvil-drop--tea-party--deferred-overrideindex1-2026-10-18) `PET_BATTLE_OVERRIDE_INDEX_1_SEMANTICS_UNPROVEN`: Anvil4816/4817 and Tea Party3729 retain nonzero property24 OverrideIndex; no guessed hit/critical/variance reuse. Actual core5 RED365a1a3d7, owned slot correction1a68de2e6 then genuine owned RED; local unshipped registry-only experiment still fails parameter preflight. Remove experiment from published history/production diff; retain six ignored REDs and coverage1885/1040. No handler/parameter/math/clock/native claim; sourced behavior packet pending.

## 2026-10-18 — Poison1954 native framing blocked

[Bounded evidence](systems/pet-battle-engine.md#poison-protocol--native-framing-blocked-2026-10-18): rewritten4cbc/fresh55e/currenta2b,10 canonical asset hashes/strict6mesh/ExtractedOnlytripwire0. Single Modern engage succeeds but posed-AABB x=-254 fails Anomalus framing; visible body seems inside, actual drawn clipping versus conservative bound remains unknown. PNG explicitly unaccepted/no1954 cast/noForever/no workaround. Physical Forfeit/Confirm/Close/PIDsabsent/sliceinactive/5961unbound/windowcleared/all4artifact hashes retained. Named `PET_BATTLE_ANOMALUS_NATIVE_FRAMING`; actual vertices and sourced placement/scale must be measured before any fix. CPU1885/1040 retained; Blood engage observation unresolved.

## 2026-10-18 — Poison Protocol1954 CPU

[Poison Protocol](systems/pet-battle-engine.md#poison-protocol--cpu-2026-10-18): only registry+bounded54 per-victim current-action positive direct damage qualification; named source-row project policy, existing379 math/clock unchanged. Actual core7+owned1 unsupported RED and favorable-after-miss RED→c7abb9d35 core8+owned1 GREEN/no fixture correction. Nine active pairs/misses/block/zero/dead/faint/prior-periodic exclusion/swap/refresh/expiry; legal Anomalus2842/4 slot3/power50 direct26+periodic11, Rabbit147→110→99. Retained lib209/main48/shareda2b clean, all ten typed70448 tables, census1885/2829=66.63%/1040graphs (+6/+1 only1954), fmt/manual readability. Lead final gate/native pending; historical70391/1051/1762 preserved, Retail timing/rounding/VFX unproven. Prior Blood engage failure unresolved; recurrence requires investigation.

## 2026-10-18 — Mega Discharge2365 CPU-only

[Mega Discharge](systems/pet-battle-engine.md#mega-discharge--cpu-only-2026-10-18): ONLYregistry+50casterguard/existingmath-clock; actual471ebcore5+NPC1unsupportedRED→273382core5GREEN/fixture-onlyd91equippedMetalFist384correction/NPCGREEN1. Nineactive/180primary90back/onlycasterself542/misses+allmiss/deadreserves/terminalpolicy. `PET_BATTLE_DURATION_ONE_SAME_ROUND_POLICY`: duration1 counts application round, queuedafterBite180 versusbefore120/next120;Retailtimingunproven. NPC2814ONLYguidNoneenemy1/NEVERjournal/source491HP-10power-7speed;fasterlegalplayerMetalFist30 then45/22/22→player114/137/137/NPC461. Boss1runtime0/unnamed1832363/compositionmissing/NPCserviceabsent namedgaps,CPU-only/no native/privatehook/fullNPCparity. Integratedf84lib201/main47/check25.36s/no warnings/fmt/shareda2b07a6clean;fresh10typed70448tables/census1879/2829=66.42%/1039graphs,+4/+1ONLY2365/historical70391+1051/1762retained. NoRetailtiming/rounding/VFXclaim.

## 2026-10-18 — Whirling Gears2351 CPU-only

[Whirling Gears](systems/pet-battle-engine.md#whirling-gears--cpu-only-2026-10-18): ONLYregistry/reuse24enemyrecipients+139lowestABSbench/ties/locks. Actual41293core6+nonownedNPC1unsupportedRED→2733core6GREEN/8228fixture-onlySquirrel159vsRabbit147correction/NPCGREEN1;9active/72×3/minABSnotpercent/posthitmissselection/allmissindependentswap/dead-noeligible/ties/locks/terminalpolicy. NPCMath2803ONLYguidNoneenemy1/NEVERjournal/source156HP-7power-10speed→16×3/player143-84-104→active1/NPC156/CD5;controlledcompositionnotauthored. Elite1/runtime0/unnamed183=2235/missingcomposition/NPC_PET_BATTLE_ENCOUNTER_SERVICE_ABSENT namedgaps;CPU-provenONLY/no native/privatehook/fullNPCparity. Integrated63f677lib196/main46/check1m16s/no warnings/fmt/shared029a927clean;fresh10typed70448tables/census1875/2829=66.28%/1038graphs,+4/+1ONLY2351/historical70391+1051/1762retained. NoRetailtiming/rounding/VFXclaim.

## 2026-10-18 — Blood Harvest2457 native proof

[Blood Harvest](systems/pet-battle-engine.md#blood-harvest--2026-10-18) afterbh563: legalHemospore4507/15physicalPass→slot2bothskins. Modern904→861(Scratch43),96/Rabbit51,+43cappedheal−Scratch30→874;Forever874→846(Scratch28),103/Rabbit44,+58cappedheal−Scratch26→878;two0benchhealseach. Four individually inspected1920×1080 `blood-harvest-{modern,forever}-{before,applied}.png` AgentShared2026-10-18/petbattle-effects;twoframedmodels/ExtractedOnlytripwire0/geometry96-105onlyHPfills/timedfloats. Lead8productfiles+fragment/all9canonicalhashlinks/strict4meshPASS. Coverage report owns85bserver+seeder/3214base/shared4be versusapprovedee951compiledshared11a/onlythreatmathdiff+27UI-model/8networkunrelateddiffs/preposthashes. PhysicalForfeitConfirmCloseboth/PIDsabsent/sliceinactive/5961unbound/windowcleared. FirstModernengagefailureUNEXPLAINED/notfixed:16:49:41.479–16:51:42.936ZnoHUD/models;retry16:52:16.171–16:52:21.776Zworked;privateobservationledger preservesinvocationtimes/logexcerpt/requestIDs. Recurrenceinlaternative→nameddefect/investigate. Nativebench/miss/deadpairCPU-only;source-rowinference/RetailtimingroundingVFXunproved.

## 2026-10-18 — Blood Harvest2457

[Blood Harvest](systems/pet-battle-engine.md#blood-harvest--2026-10-18): ONLYregistry+paired24enemy/32ally recipients/scopedactualdamagecontext; paired-ally mapping explicitly source-row inference, not Retail capture. Actual7c15core6+owned1unsupportedRED→80bpartialGREEN/deadally0→60, separate2a2caster-modifierRED→0f243livingrecipient/originalcasterheal_between GREEN7+1;±10%criticalfixture121correction. Nineactive/asymmetric4-20-60/caps/eachmiss+allmiss/no-stale777/deadreserves/casterdone-recipienttaken/criticaltype121/finalfaintheal1. LegalHemospore4507/15slot2power185→102/Rabbit45/caller100→202/benches200-300unchanged. Integrateda364lib190/main45/check1m34s/no warnings/fmt/shared4be0b6dclean;fresh10typed70448tables/census1871/2829=66.14%/1037graphs,+6/+1ONLY2457/historical70391+1051/1762retained. Nativefollowslanding;source-rowinference/Retailtiming/rounding/VFXremainproofboundaries.

## 2026-10-18 — Call of Winter1013 native proof

[Call of Winter](systems/pet-battle-engine.md#call-of-winter--2026-10-18) after557: legalButtercup3255/20physicalslot3bothskins. Modern91critical/Rabbit56/Scratch29/caller1107→1078;Forever68/Rabbit79/Scratch26/retained1078→1052;Blizzard8both. Four individually inspected1920×1080 `call-of-winter-{modern,forever}-{before,applied}.png` originals in AgentShared2026-10-18/petbattle-effects;twoframedmodels/ExtractedOnlytripwire0/geometry94/110onlyHPfills/timedfloats. Existingcanonical8assetshashlinks/strict1meshPASS,nopublication. Coverage report owns exact514afbserver+seeder/a8base/approvedee951engine/shared11a/18unrelatedUI-model+4networkdiffs/preposthashes. BothphysicalForfeitConfirmClose;ownedPIDsabsent/sliceinactive/5961unbound/windowclearedimmediately. Nativeprechilled/backline/recast/misses/expiryCPU-only;Retailrounding/timing/VFXunproved.

## 2026-10-18 — Call of Winter1013

[Call of Winter](systems/pet-battle-engine.md#call-of-winter--2026-10-18): ONLYregistry+24/29pairedceil2recipient/ownChilled52; existingconditionalmath/Blizzard205unchanged. Actualc6f9core6+Buttercup1unsupportedRED→ae7production/165fixture-onlyChilleddelta+Elementalweakcorrections/merged2be80GREEN6+1. Nineactivepairs/chillednone-each-all/48base+12bonus/misses+allmissweather/deadreserves/priorweather60+15/replacementexpiry/finalfaintnamedpolicy; legal3255/20slot3power221→64/Rabbit83/alliesunchanged/CD3. Integratedlib183/main44/check2m16s/no warnings/fmt/shared11aclean;fresh10typed70448tables/census1865/2829=65.92%/1036graphs,+7/+1ONLY1013/historical70391+1051/1762retained. Nativefollowslanding;noRetailtiming/rounding/VFXclaim.

## 2026-10-18 — Sunshell2410

[Sunshell](systems/pet-battle-engine.md#sunshell--2026-10-18): ONLY2410registry/reuse404+403; b273core5+nonownedNPCRED→194804production/9130GREEN5+1, nineactivepairs/30×3/misses+allmissweather/deadreserves/all6maximumHPwithoutcurrentHPincrease/globalhealing25/replacement/expiry. Crystalsnap3068ONLYguidNoneenemyteam1,neverplayerjournal;controlledlevel1/Common/breed3/source174HP/12power/7speed→12×3/max261/current174/CD3. MissingNPCcomposition andsourceBoss1/runtimeBoss0 retained as namedgaps;initialfixtureBossruntime1 overclaimcorrectedbeforeGREEN. Integrated2203lib177/main43/check18.27s/no warnings/fmt/shared11aclean/ten-table70448census1858/2829=65.68%/1035graphs,ONLY+4/+1;historical70391/1051/1762retained. LEADacceptsCPUenemyproof2410;native `NPC_PET_BATTLE_ENCOUNTER_SERVICE_ABSENT` (no liveNPCtrainer/teamcompositionservice),NOprivatehook/nevermake3068wildorowned. Asset-only13canonicalfileshashlinks;no nativeNPC/fullboss/Retailtiming/VFXclaim.

## 2026-10-18 — Shell Rush1989 native proof

[Shell Rush](systems/pet-battle-engine.md#shell-rush--2026-10-18) after539: legalGlimmershell2687/4 physicalslot3bothskins. Modern36/Rabbit111/highestbench300→275afterScratch25;Forever37/Rabbit110/retainedhighestbench275→250afterScratch25. Original344caster/other200benchunchanged;controlledcurrentHP not acquisition/history proof. Four individually inspected1920×1080 `shell-rush-{modern,forever}-{before,applied}.png` originals in AgentShared2026-10-18/petbattle-effects;twoframedmodels/ExtractedOnly/tripwire0,geometry91/96onlyHPfills. Existing15canonicalassets/links/strict2+1meshloadersPASS,nopublication. Coverage report owns exact0e658server+seeder/approvedee951engine/shared11a and pre/posthashes;not current Sunshell source. Physicalforfeitconfirm/Closeboth,ownedPIDsabsent/sliceinactive/5961unbound/windowcleared. Nativeenemyreserves/misses/ties/locks/finalfaintCPU-only;independentRetailtiming/rounding/VFXunproved.

## 2026-10-18 — Shell Rush1989

[Shell Rush](systems/pet-battle-engine.md#shell-rush--2026-10-18): only1989registry+145CASTERrecipientguard, reusingforce_swap/validate_swap/highestabsoluteHPbench/casterexclusion/ties/locks. Actual33d8da793core6+owned2687/4unsupportedRED→ad081production/merged636GREEN; nine active pairs/72×3/ownteamnotenemy/absolute-not-percent/HPstats/CDownership/eachmiss+allmiss/dead/no bench/tie endpoints/locks/lastenemyfaintSwapbeforeFinished. Tests explicitly cite70448row3606/145Accuracy100only independentof3603–3605; finishingauthoredactionbeforeWinner is namedexistingpolicy,notDB2-provenRetailterminaltiming. Legal Lustrous Glimmershell2687/4slot3power44/max344 deals38/Rabbit109 and chooses300-HPbench ratherthan344-HPcaster,ownHP344/200/300unchanged. Integrated650f92049lib172/main42PASS/shared11acleanbeforeafter; fresh ten-table-reconciled70448census1854/2829=65.54%/1034graphs (+4/+1/no removals); packagecheck27.74s/no warnings/fmtPASS/manualchanged-lineRustreadability no new violations(metrics unavailable). Historical70391/1051/1762/allotherrejections retained; post650onlyRaysnative-docs+thisdocs/census,no duplicatedbroadproof. Native-after-landing pending; oldfc1serverlacks1989, noRetailrounding/timing/VFXcertification.

## 2026-10-18 — Threat current-reference selection

[Victim selection](systems/creature-aggro.md#victim-update): separate server-owned current reference/deadline, strict thresholds and lower-ranked melee search. Source-clock RED3/protocol RED2/lifecycle RED4 precede implementation. Requested broad server549PASS/5FAIL/40ignored2111.99s retained; two fixture/timing repairs and three missing-env Trading Post cases rerun only in affected scopes, producing555 distinct passing server cases (composite, not fresh555) plus shared38/check0/no warnings/bothfmts0. No production changes after broad proof. Feero shared fixture loads native faction/model authority; Provoke retains all quantities/identities while observing source1000ms reselection. Complete pinned taunt call chain reviewed by lead: no forced current-ref refresh on the already-engaged/live-rival path; `TAUNT_IMMEDIACY_RETAIL_UNVERIFIED` remains user-awareness state, not a product change. Source/heads stable; canonical2c→380 drift reconciled as12follower_spec_kit updates only. Independent paired landing pending; no Azure-specific workaround, wire or data change.
## 2026-10-18 — Rays of Light2378 native proof

[Rays of Light](systems/pet-battle-engine.md#rays-of-light--2026-10-18) after531: legalRiverpaw4450/15physicalPass→slot2bothskins. Modern929→902(Scratch27),110damage/Rabbit37,+27cappedheal−Scratch30→899;Foreverretained899→870(Scratch29),112/Rabbit35,+53heal−Scratch27→896. Fullbenchheals0; injuredbench/miss/finalfaintcontrolsCPU-only. Four individually inspected1920×1080 `rays-of-light-{modern,forever}-{before,applied}.png` originals in AgentShared2026-10-18/petbattle-effects;all4phases2framedmodels/ExtractedOnly/tripwire0,geometry95/104onlyHPfills/timedfloats. LocalCASC6/6→lead8productfiles+fragment;9canonicalhashes/links/actual1meshstrictloaderPASS. Freshfc1server/seeder fromlanded506/shared11acleanbeforeafter;leadexplicitlyaccepts frozenengineee951 after exact003e35233 protocol/petbattleUI unchanged versus17unrelatedUI-model changes+networkWorldServerInforeceive. Fullpatches/actualhashes retained;workingShell650notruntimecohort. All3hashesunchangedthroughShellCPUandnative. Bothphysicalforfeitconfirm/Close;knownPIDsabsent/sliceinactive/5961unbound/windowcleared. Coverage report owns exactreceipts/limits;functional/visualonly,noindependentRetailrounding/timing/VFXcertification. Leadacceptednative;closingdocs-onlyhead doesnotchangeproductionorcensus.
## 2026-10-18 — Masquerade697 authored cast timing acceptance pending

[Exact acceptance/limits](../toy-effects14-coverage.md#masquerade697-authored-cast-prepost-completion-acceptance-pending-after541-2026-10-18):35168319d extends existing276207/1298957 learned600ms cast with no early appearance/SpellGo/category recovery, then source145134/misc267626,8/one-hour aura/2513 recovery and Idol refusal. Old fixtures unchanged; format/manual only, uncompiled/unexecuted AFTER541. No runtime/data/protocol/new CancelCast, exact-timing/native/hour-expiry/all-five certificate or own jobs.

## 2026-10-18 — Gate541 GREEN/landed Singed593 and Pango684

[Exact gate/landing proof](../toy-effects14-coverage.md#gate541-green-and-landed-singed593-repair-plus-pango684-lifecycle-2026-10-18): explicit lead receipt plus ancestry confirmsa6f8505f7 on master312f5aebd.541server317/0/1ignored469.98s/EXIT0/FMT0/CHECK0 with exact593/Pango tests OK.535's real runtime admission failure repaired by named Singed slot, not fixture change or blanketdummy. Native/full1166/all-five/hour-expiry/distribution limits remain; no duplicate jobs.

## 2026-10-18 — Singed593 repair/Pango684 owned CPU GREEN

[Exact proof/limits](../toy-effects14-coverage.md#singed593-repair-and-pango684-cpu-verification-green-successor-landing-pending-2026-10-18): real535RED→runtime5ea7efcaa at6b54512a0/canonical shared11a058cf, own focused5931/0/25.16s; toys197/0/337.82s includes593, PangoCancelAura/ordinary600sSaptorreuse and unchanged Citizens/generic dummy refusals. Workspace all-targets check1m24s/fmt pass. Only audited Singed slot admitted; sourceTC76/4836–5117, not blanketdummy. Initial old-interface build failures resolved by absolute current shared path without protocol/cache mutation. Lead successor/landing/native/full1166 proof remains pending.

## 2026-10-18 — Gate535 genuine RED; Singed repair pending

[Exact failure/source-grounded repair](../toy-effects14-coverage.md#gate535-genuine-red-narrow-singed-admission-repair-pending-verification-2026-10-18):535server315/1/1ignored477.06s/EXIT101/FMT0/CHECK0 rejects593child aura4 before cast; no landing/fixture correction. Authorized5ea7efcaa admits only1220867/slot0 under zero-payload visual-aura guard, preserving unrelated script refusal. TC `SpellAuraEffects.cpp:76,4836–5117` contains named dummy actions, not generic no-op authority. Merged landed baselined0e0d7fc3 for queued64 fidelity, Pango stacked; own focused compile/test running, no GREEN/native proof yet.

## 2026-10-18 — Pango684 cancellation and shared recovery pending

[Exact acceptance/limits](../toy-effects14-coverage.md#pango684-cancellation-and-ordinary-shared-recovery-pending-after535-2026-10-18): test/spec223d0916c extends existing Full1166 Pango with real CancelAura/model restoration/no2513 bypass, Saptor refusal near580 simulated seconds and own learned success after600 ordinary seconds. No timer/cooldown edit or prior shared-five change. Format/manual only; uncompiled/unexecuted AFTER535, not all-five lifecycle/wall/hour-expiry/native/distribution proof. No runtime/data/own jobs.

## 2026-10-18 — Mycomancer664 landed after532; Explosive593 queued535

[Current proof/landing](../toy-effects14-coverage.md#mycomancer664-gate532-passed-and-landed-explosive593-queued535-2026-10-18): lead receipt plus ancestry confirms8af454985 on server masterd0e0d7fc3.532server315/0/1ignored673.31s/EXIT0/FMT0/CHECK0 includes exact Mycomancer learned-root CPU test. Native/cross-map/missing-bind/natural-recharge limits persist. Explosive38ced31bd separately queued535; no535result or landing claimed, no own jobs.

## 2026-10-18 — Explosive593 mixed hearthstone and Singed acceptance pending

[Exact acceptance and limits](../toy-effects14-coverage.md#explosive593-mixed-hearthstone-and-singed-cpu-acceptance-pending-after532-2026-10-18): test/specbb325c6a4 preserves actual236687/root1220729 ten-second bind252 plus queued self64→1220867/5000ms dummy4 Singed, shared2309 and Corewarden refusal. Ordinary child expiry tested without replacing/dropping child; native explosive/Singed presentation remains gap, no guessed damage. Original25/helper/Mycomancer unchanged. Format/manual only; uncompiled/unexecuted AFTER532, whose candidatefb2bd5a88 is not landing. No runtime/data/native/own heavy jobs.
## 2026-10-18 — Native follower utility conditions

[Dispel/control conditions](investigations/follower-data.md#native-dispel-and-caster-control-conditions--2026-10-18): effect38/native ordinary eligibility, patient-first filtering; native Crenna/Henry Magic cures and2negative/1retained controls GREEN. Native702 harmful=false isolated/routed as AURA_POSITIVITY_702; approved334275 fixture uses source area/range and old core DATA-RED, as does Bo's Leg Sweep. Corrected source-duration/category-CD wait gives genuine Meredy Poly RED. COPY refresh existing-table RED→GREEN; source-immutable SQL replay tests updated after core publication. New data b90 native8PASS/1fixture CreatureType failure; real template6 authority fixes that sparse fixture, giving0ffa24/24 core23+PolyPASS238.69s. Post-nav/jump-master6a979ee2f selected native32/0FAIL/1namedignoredPASS319.97s; producer3PASS39.833s, servercheck0/25.73s/zero warnings, fmt/diff0. COPY380f90da/delta00ddfc95/canonical-input2c836975 hashes verified; proofs retained across final docs-only changes.702 +three joint interrupt REDs remain ignored; lead540 GREEN/landedf318f3940 and independent canonical380f90da publication match delta00ddfc95 (12UPDATEs). Interrupt provider/removal reserved for joint9468 gate. Core530 landed4bc7cc15e with byte-identical lead publication2c836975; broader full-kit/live acceptance remains open.

## 2026-10-18 — Rays of Light2378

[Rays of Light](systems/pet-battle-engine.md#rays-of-light--2026-10-18): six interleaved24damage12/7/7 and23heal5/Variance20 rows; ONLY2378registry+boundedceil2damage/order2heal selection reuses existinghelpers/math. Actual8fa1d7f3dcore5+owned4450/15unsupportedRED→aa44158cdcore5+owned1GREEN; parameterized258/979controls, nine active pairs/asymmetricenemyhits/distinct3allyheals/caps/eachmiss+allmissheals/deadbothreserves/finalenemyfaintstillheals. Legal Remembered Riverpaw4450/15slot2power180/max929 deals120/Rabbit27 and heals100/200/300→150/250/350; no fixture correction or fabricatedstats. Integrated84395c5b7lib166/main41PASS/shared11acleanbeforeafter; fresh ten-table-reconciled70448census1850/2829=65.39%/1033graphs (+6/+1/no removals); packagecheck27.01s/no warnings/fmtPASS/manualchanged-lineRustreadability no new violations (metrics unavailable). Historical70391/1051/1762/otherrejections retained; onlydocs/censusafterproof. Native-after-landing and independent Retail rounding/timing/VFX remain unproved.
## 2026-10-18 — Main-thread economy fsync capture and bounded FIFO implementation

[FIFO cycle publication](systems/bot-economy.md#fifo-cycle-publication): ce0/protocol11a private owned-pressure stack proves main fdatasync→next_id→economy gatherer cycle; separate writer idle. Real FileBackend sync-held structural regression genuinely RED. Bounded high-water reservation/market receipts and expected-state conflict handling implemented; economic-vendor requests retained in owned FIFO until receipt resolution. Real concurrentbuyout RED erased11-coppermail; exact expected-mailbox guard preserves it and nextscheduledcollection. Final scoped coverage57economy(56+1fixturecorrection)/1admin/7peer; workspacealltargetscheck/fmt0/no warnings; independent verifierPASS. Whole-row inputs audited; leadgate pending, no performancePASS. MAIN_THREAD_ASSET_READS named/excluded; dormant A/B parked. Historical privateworld schema setup failure retained separately; current reproduction uses read-only SQLite backup/private mutable store.

## 2026-10-18 — Tidal Wave X2309 and Shadow Bolt Volley2523 native proofs

[Tidal Wave X](systems/pet-battle-engine.md#tidal-wave-x--2026-10-18) after516: legalSwoggy3570/4physicalslot3 bothskins51/56damage/Rabbit96/91;Scratch26/27/caller321→295→268. [Shadow Bolt Volley](systems/pet-battle-engine.md#shadow-bolt-volley--2026-10-18) after520: legal4949/15physicalslot2Modern134/Rabbit13/Scratch30/caller990→960;Foreverwinner147/Rabbit0/noScratch/retained960. Eight individually inspected1920×1080 originals in AgentShared2026-10-18/petbattle-effects, `{tidal-wave-x,shadow-bolt-volley}-{modern,forever}-{before,applied}.png`;all8phases2framedmodels/ExtractedOnly/tripwire0. GeometryTidal91/99onlyfills/timedfloats;Shadow92/98plus expectedScratchline/float versuswinnerOutcome/Close. CanonicalTidal7/2meshloader andShadow13assets/3publicationhashes/6meshloaderPASS. Onefresh6b6server fromlandedd467+Swoggyseeder;Shadowonlynew52c82seederbuild7.8s;shared11acleanbeforeafter. Leadexplicitlyaccepts frozenengineee951 after exactprotocol/petbattleUI unchanged versus14unrelatedUI-model changes andnetworkWorldServerInforeceive;fullpatches/actualhashes retained,sourceHEADnotruntimecohort. All3TidalhashesverifiedbeforeShadowseederoverwritespath;Shadowall3verifiedafterrun. Functional/visualduringst526,notquiet-hostperformance. Physicalforfeitconfirm/CloseexceptForeverwinnerClose;allownedPIDsabsent/sliceinactive/5961unbound/windowcleared. Coverage report owns exactreceipts/limits;native reserves/misses/objects/varianceendpoints/independentRetailtimingVFXunproved,CPUcontrolsseparate.
## 2026-10-17 — Normal-cast remaining-timer clamp

[Native pushback cap](systems/auras.md#normal-cast-remaining-timer-cap--2026-10-17): actual Omor30686/native damage at350ms, delivered by existing50ms event step at400ms, leaves3.1s remaining on3s bar (RED0/1/36.85s). TCSpell.cpp7994–8028 caps remaining time at original cast duration; fixed duration plus elapsed rewind implements that algebra without new state/wire fields. Two-hit/reduction/channel behaviour retained; late-hit controls observe actual deadlines. Initial6/6 GREEN, then fresh-master+approved9f fixture integration6ae25ad4b GREEN9/9/0ignored99.87s (501336ms wrapper), including exact active Omor Shield/source-repeat and Frostbolt ten-tick launch boundaries. Check/fmt pending; no imported-clock/RNG/actor-budget changes.
## 2026-10-18 — Skyriding native-child fixture regression

[Skyriding fixture](systems/mounts.md#skyriding-native-prerequisite-fixture--2026-10-18): exact landing-parent8707ac0ea21/21 versusae9a7a53c11/21 proves required-aura checks exposed omitted372773 Dragonrider Energy, not dormantfc03721/21 or later cursor capture. Loading native child lets normal406095 LINKED285 produce it; setup now asserts that aura, no direct insertion or production relaxation. At1342a9fb0 skyriding21+unchangedAkunda1 pass22/22,0ignored24.84s/176192ms; all-target check12.123s/0warnings and fmt pass. Existing gameplay assertions unchanged; no live/full-class claim.

## 2026-10-18 — Native follower spec-kit bridge

[Full-spec bridge](investigations/follower-data.md#level-gated-full-spec-bridge--2026-10-18): additive70448 starter-budget/level-gated priorities, native990 loadouts, COPY-only producer and literal source receipts. Initial role selection REDs reproduce four-slot limitation; native18/19 plus corrected sourced-WoG control pass. Separate urgent19750 restart RED proves the new picker repeatedly interrupted its own cast; identical admitted spell/target preservation committedb84d7744b. Native Demonology shards0→0 RED → kit-only sourced ActiveSpec3adafc8ee. Integratedfebf17169 native23 core+12 legacy defensive+9 CC =44/44 GREEN542.82s, zero ignored/failed; e29b77ff3 check0/2m08s/zero warnings, fmt0 and producer2/2PASS16.616s; nineteen proven tests enabled, behavior proof retained across attributes/docs only. Lead gate/publication pending; no canonical publication or tank movement/live/full-kit acceptance.
## 2026-10-18 — Shadow Bolt Volley2523

[Shadow Bolt Volley](systems/pet-battle-engine.md#shadow-bolt-volley--2026-10-18): active70448 turn4136/rows4902–4904 property222[15,100,30,0,0,0], no abilitystates. Actualc66b8d6e1 core4+owned4949/15 unsupportedRED→5fc44fa1e registryONLY/core4+owned1GREEN; existing24/222recipientmapping/variance math unchanged. Parameterize319/2309damage controls (not objectclear), nine active pairs/eachmiss/deadreserve/allies conserved; authored30endpoint76/103notdefault20's81/98. Legal Shadowflame Remnant4949/15slot2power178/max990 capped147/Rabbit0. Integratedlib164/main40PASS/shared11acleanbeforeafter; fresh ten-table-reconciled70448census1844/2829=65.18%/1032graphs (+3/+1/no removals); packagecheck14.25s/no warnings/fmtPASS/manualchanged-lineRustreadability no new violations (metrics unavailable). Historical70391/1762/other rejections retained; onlydocs/censusafterproof. Native-after-landing and independent Retail rounding/timing/VFX remain unproved.

## 2026-10-17 — Fel Devastation scheduled-heal baseline

[Vengeance212084](systems/auras.md#fel-devastation-scheduled-healing--2026-10-17): native19 selected+5 supplemental effects match read-only70448/runtime. Required BEFORE-hook existing geometry3/3PASS11.81s atb48121a3f; actual75deg/13yd/ConeToDest/Max0/current-facing and self-only heal retained. Genuine c8a421d56 RED6FAIL/2PASS21.12s observes missing scheduled212106, using actual320639 rank/five native10-Fury Shears and unmodified2s/200ms channel. New named periodic-only hook heals before default212105; no target/state/generic trigger/power/channel changes. FEL_DEVASTATION_UPFRONT_HEAL_SOURCE_CONFLICT remains deferred (SimC execute heal versus TC periodic-only), absence not certified. GREEN/controls/check/fmt/lead gate pending; full origin/flags/procs/haste/ability/spec excluded. Initial wrong enum import was compilation only, not featureRED. Prior Mistweaver landedce0fec81c afterst526/no attributable failures.

## 2026-10-17 — Expel Harm total-heal consequence

[Mistweaver322101](systems/spell-casting.md#expel-harm-total-heal-consequence--2026-10-17): native20 selected/23 supplemental effect rows equal read-only70448/runtime. Pinned SimC result_total includes overheal; native137024 supplies1.4%mana eligibility. Before product changes, actual115129 selector control passes15yd/not21yd (slot-B20yd, separate range10, native caster-destination0delay). Six genuine root missing-child REDs at34a501acc plus corrected15sec-repeat0-versus3 RED1b77586ae; initial repeatOnCooldown INVALID feature proof. Enveloping/Vivify/dead/noenemy controls pass. Named nonperiodic resolved-total callback uses existing native child helper, no state/generic extraction/trigger/clock/RNG changes. Native62ed3a285 produces17PASS/1bad wire-mana assertion (+25 native regen after cost, not product error); completion-cost plus wire full-overheal controls pass2/2 at1332bc603 after docs-only master8e07c0fc1 merge. Retain17 unchanged passing cases: scoped12native+7existingcontrols GREEN,0ignored/0warnings; check71.88s/fmt3.87sPASS, manualRustreadability inspected (metrics unavailable). No resource/regen changes or broad replay. Lead later confirms8f4ad420d LANDEDmasterce0fec81c after combined suite st526/no failures attributable to slice; excluded interactions/fullability/spec remain open. Prior Unholy3b29f4389 landed lead5151181+109/fmt.
## 2026-10-18 — BONESTORM!1761 native proof

[BONESTORM!](systems/pet-battle-engine.md#bonestorm1761--2026-10-17): after gate499(163+60/fmt), legalWhirly2986/4 physicalslot3 bothskins deals49/47enemy plus119selfcost; Scratch40both leaves399→240→81, corroboratingMAX399notcurrent240/enemy147. Four individually inspected1920×1080 `bonestorm30-{modern,forever}-{before,applied}.png` originals in AgentShared2026-10-18/petbattle-effects; all4phases2framedmodels/ExtractedOnly/tripwire0,geometry91/101onlyHPfill/timedfloat. LocalCASC15/15→lead15productfiles+fragment;16canonicalhashes+links/sevenexistingBLPhashes/actual8meshstrictloaderPASS. Reused frozenserver2c27/engineee951/shared11a; onlyprivateseeder36d4f45build6s/sharedcleanbeforeafter;all3artifacthashesunchanged. Physicalforfeitconfirm/Closeboth;ownedruntimestopped/5961unbound/windowcleared. Coverage report owns exact receipts/limits;native reserves/misses/passives/independentRetailrounding-timing/VFXunproved,CPUcontrolsseparate. Leadacceptednative;closingdocs-onlyhead doesnotchangeproduction orcensus.

## 2026-10-17 — Scourge owned-plague eruption

[Native55090](systems/spell-casting.md#scourge-strike-owned-plague-eruption--2026-10-17): authenticated70448 source25 effects match read-only runtime. Shared pre-mitigation numeric extractionbf728dc9a retains ordinary rounding/downstream order; BEFORE-eruption damage/absorb/crit/versatility34/34PASS13.22s. Genuine nativef1e0ffeaf RED6FAIL/4PASS25.13s observes absent1241167/1241171 child damage. Named own-plague consumer now precedes missing-Virulent application; no source clock/RNG/coefficients or generic hit/trigger changes. Native9cacd3193 GREEN10/10 in35.62s,0ignored/0warnings. Integrated59c68472e/master3db506c3a server-bin check24.91s/fmt2.21sPASS; affected12 cannot compile because master has two old zones::spawn_creature callers (14E0308), repair routed to lead. Owner repairefd627da1/lead511 integrated24b98c2ac; native10+controls2 PASS12/12 in53.31s (659.90s coldtotal),0ignored/0warnings,fmtPASS8.83s. Retain32 disjoint extraction controls and unchanged production bincheck; no fullsuite/redundant broad rerun. Manual changed-Rust readability inspected, metrics unavailable. Leadgate pending; spread/ties/Scourging/pets/fullspec remain open.
## 2026-10-17 — Shared source entry projection

[Entry projection](systems/boss-scripts.md#creature-entry-projection): current source/runtime-state fixtures reach8 genuine RED at1e10b0a51 (27.94s); setup-only addon/clock/master-API failures excluded. Shared initial/same-actor projection now implements represented source stats/model/pose/equipment/regen/string fields, retains origin/AI/home/threat, and preserves deferred dormant addon setup. Focused11/11 PASS18.56s; final affected68/68 PASS233.18s at9e83421de (55 pre-existing ignored), plus actual Morna/Drudge nonzero input1/1 PASS10.14s at65e035a2a on unchanged production code; finite5s347931 exact-deadline1/1 PASS17.53s at44cdfe55c, required packagecheck0/warning-free34.97s and fmt0. Independent landing pending. Original-entry respawn, SmartAI36, full TC fields and actual Herald choreography/native acceptance remain named gaps.
[Native55090](systems/spell-casting.md#scourge-strike-owned-plague-eruption--2026-10-17): authenticated70448 source25 effects match read-only runtime. Shared pre-mitigation numeric extractionbf728dc9a retains ordinary rounding/downstream order; BEFORE-eruption damage/absorb/crit/versatility34/34PASS13.22s. Genuine nativef1e0ffeaf RED6FAIL/4PASS25.13s observes absent1241167/1241171 child damage. Named own-plague consumer now precedes missing-Virulent application; no source clock/RNG/coefficients or generic hit/trigger changes. Native9cacd3193 GREEN10/10 in35.62s,0ignored/0warnings. Integrated59c68472e/master3db506c3a server-bin check24.91s/fmt2.21sPASS; affected12 cannot compile because master has two old zones::spawn_creature callers (14E0308), repair routed to lead. Owner repairefd627da1/lead511 integrated24b98c2ac; native10+controls2 PASS12/12 in53.31s (659.90s coldtotal),0ignored/0warnings,fmtPASS8.83s. Retain32 disjoint extraction controls and unchanged production bincheck; no fullsuite/redundant broad rerun. Manual changed-Rust readability inspected, metrics unavailable. Lead later confirms3b29f4389 LANDED515/1181+109/fmtOK; spread/ties/Scourging/pets/fullspec remain open.

## 2026-10-17 — Ordinary context refusal and fixture source repair

[Level context](systems/level-scaling.md#gate486-fixture-source-audit): gate486 exposed cached Lesser Voidwalker20 versus actual area279/CT11/native30 and unsupported Lucifron map409 difficulty3 versus actual spawn9/18. Three-case reproduction at7763d065a confirms both failures and missing-context panic. Spawn boundary now logs/refuses unresolved actors before entity creation and continues valid population; criterion below/equal/above coverage uses29/30/31, lifecycle uses source9. Focused3/3 PASS60.13s and affected54/54 PASS150.97s (55 pre-existing ignored), fmt0 at5a2c2d449; package check0/warning-free32.05s at docs-only41e030bf1. Requested mastere233d88f1 integration79876f34f retains FIFO selection/fence helpers, moves ground_map beside world_db_path and unions wiki entries; combined affected+focused56PASS150.66s/55ignored, check0/warning-free8.55s. LEAD486b LANDED799PASS/fmt0/Python no new baseline failures, including Nazan source data. No canonical or Herald lifecycle changes.
## 2026-10-17 — Agony ramp and native Sudden Onset

[Affliction](systems/auras.md#agony-ramp-and-sudden-onset-2026-10-17): active70448 Agony980 cap8, Sudden1260209 starts3/+10%; legacyWrithe196102 absentcurrentmembership. Genuine nativeeff2f2115 RED8/9 (timed/deadline15vs30,recast2vs1,initial1vs3) plus unchanged CorruptioncontrolPASS before named live-stack/post-hit/refresh-floor implementation. No source coefficients/clocks/RNG/generic resource/trigger change; A226 shard RNG and full-ability gaps stay open. Native9/9 GREEN8.95s at5b0ee370f plus retained controls10/10 GREEN8.87s at557fc91df; scoped check20.53s/fmt PASS warning-free (19/19,0ignored), manual changed-Rust readability inspected. Later master/comment/docs leave native9 logic unchanged; no redundant cohort replay. Independent gate pending. Prior Ebonbaa9b0124 landed with lead4931061/fmtPASS.

## 2026-10-17 — Ebon simultaneous source snapshot

[Ebon](systems/auras.md#ebon-might-baseline-2026-10-17): original native6 GREEN at224e0300; two same-frame ordinary HostClient casts additionally reproduce11.671872 versus12.72 before the first tick. FixedUpdate aura modifiers preceded normal Update CharacterStats production. Reuse the existing cached native producer before the Ebon snapshot; no formula duplication, fallback, timer or global schedule change. At06b2d6cd7 expanded Ebon8/8 passes12.93s, prior support4/4 +legacy Eruption2/2 +native timer1/1 pass (15/15,0 ignored); source coefficient/variance unchanged. Scoped server-bin check28.73s/fmt pass warning-free; independent lead gate pending.

## 2026-10-17 — Ebon Might baseline

[Ebon Might](systems/auras.md#ebon-might-baseline-2026-10-17): approved395152/self395296 baseline, native primary support/2-N split/owned1000ms updates and foreign-grant exclusion; native16114cc3 RED6 before implementation, GREEN pending. Source amounts/radius500/MaxTargets30 preserved; Eruption/passive flat extension retained, no generic540/party-only/optional/whole-ability policy. Prior Prescience/Sands slice landed after lead gate482 passed1053/fmt.
## 2026-10-18 — Queued effect64 captured destination

[Captured targets](systems/spell-casting.md#queued-effect64-captured-destinations--2026-10-18): gate513 Akunda regression reproduced by unchanged native learned-use/cursor/cooking fixture,0PASS/1FAIL40.11s/209208ms: child fire at casterx−8949.95 instead of parent cursorx−8947.95. Pinned TC copies destination and captures targets/itemLevel by value; server queued payload omitted its modeled destination. Minimal optional-point capture and fresh-child restoration added before validation/planning, with existing selected entity/item-level/order/cost/depth retained. No position fallback or clamp change. Unchanged Akunda GREEN1/55.42s, then current-master6114bad56 requested queued_trigger/power_drain/toys:: GREEN205/0FAIL/0ignored493.85s (614864ms wrapper). Fmt passes; final check pending. No clamp code is included.

## 2026-10-17 — Omor repeat measurement boundary

[Dispatch versus completion](systems/boss-scripts.md#omor-dispatch-clocks-and-pushback--2026-10-17): gate4851694PASS/1FAIL5.75s Go gap; instrumented native damage adds500ms pushback. Pre-queued master A/B independently shows the same completion-gap compression, so this is a pre-existing measurement flaw, not established queued-specific timer regression. Fixture uses actual dispatch observations with identical repeat bounds; first-Go/hound/shield/credit checks unchanged, deterministic native-pushback control added. Corrected targeted proof pending; no production/RNG/clock/tolerance changes.

## 2026-10-17 — Queued64 source publication and Execute override

[Native script boundary](systems/spell-casting.md#execute-effect64-script-override--2026-10-17): missing mandatory aura-table reproduction passes after lead-owned source publication; full stack redone on current master. First wide1584PASS/18FAIL/14ignored separates nine missing prepared-copy environment inputs, two native61781 exclusion fixtures, six native-delay/application/order fixtures and one real Execute script override. Pinned TC prevents Execute's default64 and directly casts260798 before the survivor refund; all original strict assertions remain. Recovery29/29 passes34.22s at71c0be65/protocola56688e, including queued0/1ms/order/payment/recipient controls; Finalf2be0f8b wide1606server+two single-target cases/0FAIL/17ignored1474.24s (1624361ms wrapper), workspace/all-target check164019ms/zero warnings and fmt/diff/manual Rust readability pass. Source DB remains published318fda95. No fresh-UDP/rendered/full13/40 claim.

## 2026-10-17 — Bounded Augmentation support stats

[Augmentation](systems/auras.md#augmentation-support-stats-2026-10-17): active70448 Prescience409311→410089 recipient crit and Shifting Sands413984 caster-mastery Versatility; exact spell-specific SimC76f8118 semantics, no generic540. Genuine native RED4→merged5ed1b656 GREEN4/4 in5.52s; initial functional observer mismatch corrected test-only, native zero-variance damage assertion tightened to exact. Docs-only0114bbf7 retains native timer control1/1, scoped server-bin check91s and fmt check pass warning-free; independent lead gate pending. Automatic targeting, E202/dummies, empower grants, Ebon and full-ability/legal/UDP acceptance remain open.
## 2026-10-18 — TC-backed reach expectations and stair map identity

[Gate527 correction](systems/navigation.md#gate527-expectation-and-fixture-correction): pinned a352b1fa uses3Dchase/shortening; legacy XZ-only reach expectations now measure3Dwith unchanged2yd and existing tolerances. Original stair test fails identically on692cd5862/79aee679f because target lacks InMap; test target receives actual map0 identity so subsequent comparison exercises navigation rather than lost-target stop. No production movement change or canonical asset edits.

## 2026-10-18 — Grounded3Dmelee reach

[Actual run24 regression](systems/navigation.md#grounded-melee-reach-agreement--2026-10-18) reproduces stationary pursuing across40 real-nav/ground/LOS steps: XZ2 but3D2.066. Shared reach predicate and3Dsegment/sphere ground-travel calculation retain2yd/smaller reach, real LOS and support/speed constraints; post-commit targeted GREEN and lead gate remain separate.

## 2026-10-18 — Native tile-corner corridor visibility

[Navigation root](systems/navigation.md#tile-corner-corridor-detours--2026-10-18): actual Stockade2yd LOS-clear crossings detour7–29yd despite a full native portal and clear mesh ray; generated floors reproduce other corner/orientation cases. Complete native visibility corridors may replace a longer A* portal-midpoint corridor without removing detail crossings or increasing area costs. No data rebake, direct movement substitute, reach/LOS tuning or runtime proof; post-commit gate remains separate.

## 2026-10-17 — Rain of Fire dynamic objects

[Ordinary visibility](systems/spell-casting.md#rain-of-fire-dynamic-object-visibility--2026-10-17): actual Lord Overheat86636 object/source5yd/9s/destination reaches missing-Replicate RED; caster-removal control passes. Actual object now uses shared geometry and ordinary same-copy interest/despawn, with no damage/kit/stat/timer changes. Postmerge source-bound server GREEN2/2/270.2s and check29.25s/fmt/0warnings pass; shared wire1 and engine UDP21/native projection4 pass separately. Joint landing/native escape proof pending; fixture movement is not claimed native input.
## 2026-10-17 — Dormant aura/addon deferral

[Cold authored spawn boundary](investigations/server-tick-lag.md#dormant-aura-initialization-and-addons-2026-10-17): requested5d201f1f2 actual Cenarion15184/nativeaddon18950 becomes GridDormant yet applies its effect, genuine RED; active guard and native Azure Sprout controls pass. Only aura-init/addon dormancy filters change; reset/health/clocks/population untouched. Matching plain-release baseline e4bdacb80/protocol019cd75b frozen before implementation; targeted44PASS/0FAIL/2existingignored atfc0372d93; native model-scale/Azure addon and patrol/escort/wake controls included, no warning. Check/fmt pass; gate502 landed507/0/50existingignored/fmt0. Both split-window frozen phases complete11/11×90s/population187770; steady20.012Hz/19.625ms→18.020Hz/36.508ms with unequal IO/memory pressure. Performance benefit remains unproven; setup-only missing-admin attempt retained, admin now frozen/hash-required. No baseline replay or new source fix.

## 2026-10-17 — Cold hydration and password-token FIFO

[Cold login boundaries](investigations/server-tick-lag.md#cold-selection-hydration-and-password-tokens): two genuine unwarmed-selection/password-login writer-hold REDs; hydration/token jobs use existing FIFO receipts, retaining cached achievement authority. Six timing sites converted to explicit release; migration/abandonment controls added. First GREEN attempt exposed only private-interface compilation errors, repaired without suppression; behavioral coverage93 distinct controls passes82latest+11retained (not repeated93), including five corrected receipt-aware fixtures and actual achievement/Stats/PvP consumers; lead gate pending. Preceding frozen plain11-peer90s proof accepted as bounded20Hz/15.575ms mean PASS, not causal A/B;~1GiB server swap is observation only.

## 2026-10-17 — FIFO reader coherence

[Reader audit and ordering proof](investigations/server-tick-lag.md#reader-coherence-follow-up): full gate exposed a durable news read racing queued writes. Requests/publications now read on the same FIFO; login/Guide use writer-fed character contributions. Raw migration helpers are unavailable in production. Original news assertions retained behind an explicitly controlled commit boundary; targeted re-gate/live acceptance remain open. Site live-copy capture explicitly excluded as `SITE_COPY_LIVE_REDB_SNAPSHOT`.

## 2026-10-17 — FIFO persistence core restoration

[Bounded restoration](investigations/server-tick-lag.md#fifo-persistence-core-restoration-2026-10-17): current runtime lacked earlier FIFO lineage; monthly/account/periodic/real-Guide held-writer controls reproduce250–253ms against50ms bound. Import only writer/cache ownership and required migration plumbing, retain current guild-news atomicity/evaluation cache and landed delta/clock publishers. Same-writer Guide/Tender settlement and Trading Post commit-gated requests now pass13 core/28 affected controls on integrated master b8b6c2f9d; corrected20-ACK same-frame/due-save/reopen control passes separately. Fury22's43 degraded windows are a second retained reproduction, not controlled A/B evidence. Broad lead gate, exact PvP stall attribution and post-landing release acceptance remain open; no other unlanded performance topics imported.
## 2026-10-17 — Natural first-center landing GREEN

[First center landing](investigations/follower-data.md#telash-first-center-landing--2026-10-17):0d000bd4 actual natural landing plus affected original phase2PASS44.03s after genuine1RED40.09s; self388008 normal8s/no airborne cast/no instantGo/retained shield-passive-gravity/barcontinuation verified. Exact70source/HEAD/protocol/published-input guards stable. Case enabled afterward; scoped1ae8e179 server check0/1m00s/zero warnings and fmt0, focused primary-source/readability proof pass; lead integration pending. Return/AfterCast/runes/counters/budgets/pixels remain separate; no gravity restoration invented.

## 2026-10-17 — Natural first-center landing RED

[First center landing](investigations/follower-data.md#telash-first-center-landing--2026-10-17):dcd7fefe actual natural phase/outbound/airborne/no-cast/exact-center prerequisites pass, then missing native3880088-second self CastState1FAIL40.09s. Exactsource/HEAD/protocol/published-input guards stable. Only TC133–144 first-center branch/source boolean added; EVENT_JUMP1004 shared without value/math change. GREEN pending; return actions/AfterCast/runes/counters remain separate and no gravity restoration invented.

## 2026-10-17 — Natural phase-hit outbound jump GREEN

[Telash natural phase hit](investigations/follower-data.md#telash-natural-phase-hit--2026-10-17): source387928effect0→fullmask387981 exact-center bridge d622ea9b passes actual natural callback and original Energyphase controls2/2 in57.23s; exact70source/HEADclean/protocol11a058cf/published5f/private62403 guards stable. Test enabled afterward; no primitive/class reruns. Scoped051a5432 server check0/5.12s/zero warnings and fmt0, focused direct primary-source/readability verification pass; lead independent integration pending. Natural landing/return/runes/counters/budgets/pixels remain separate.

## 2026-10-17 — Natural phase-hit outbound jump RED

[Telash natural phase hit](investigations/follower-data.md#telash-natural-phase-hit--2026-10-17):9937b8dd actual60→100Energy/Icy/channel/ordinary phase/cost/shield/gravity prerequisites pass, then missing outbound motion1FAIL27.52s (unchangedpose/progress0). Exact source/HEAD/protocol/published-input guards stable. Only sourced387928effect0→triggered387981 exact-center hit bridge added; GREEN pending. No manual jump/spell/hook or new stats/HP/resource priming; landing/return/runes/counters remain separate. Primitiveb72 gate head unchanged remotely.

## 2026-10-17 — Finite jump integration proof

[Jump source/runtime](investigations/follower-data.md#telash-jump-charge-source-producer--2026-10-17): nativepair2 retained5aa, affected22 seven passes plus corrected Feral740 one pass (native Mana accessor), unsupported-source17 one retained pass =11 distinct cases, not a fresh11-case execution. Parent191427/196884/189110 effects, cast success/clocks/native zero-cost resources remain while only unsupported254 displacement refuses; old placeholders named/flipped, Harpoon41/Gordoth3 preserved. Scoped server check0/38.20s/zero warnings and fmt0; exact70source/HEAD/clean/protocol11a058cf/5f/private62403 guards stable. Independent source/wiring/artifact PASS with explicit limitations. Full Telash callbacks/runes/ordinary Azure/pixels remain open.

## 2026-10-17 — Native jump GREEN and player displacement boundaries

[Jump source/runtime](investigations/follower-data.md#telash-jump-charge-source-producer--2026-10-17):5aa9f58a native713/714 launch/midarc/source-time/exact endpoints plus missing-input3PASS138.15s, retained actual-source unsupported1PASS. WorldData connection replaces failed relative startup reopening; initial fixture/compiler failures remain separate. Lead approves named displacement-only refusals191427/189110 progress curves and196884 parabolic/arrival trigger; preserve parent transform/damage/CC/bite/cooldowns/resources. Grepped1539 installed254 IDs against tracked test literals and flipped explicit placeholders, retained Harpoon41 and class effect assertions; regression/check/fmt pending, full Azure/pixels open.

## 2026-10-17 — Native jump planning and launch boundaries

[Jump source and runtime](investigations/follower-data.md#telash-jump-charge-source-producer--2026-10-17): producer510 published exact5f225535/SQL49b238f5; repair511 integrated. Native destination planning2RED60.53s precedes exactly approved TC SpellMgr4940–4956 effect0TargetA87 corrections for387981/388082. After correction, actual native2RED58.95s proves launch teleport after source/floor/LOS prerequisites; no synthetic movement/hook. Dedicated254 source-table handler and asynchronous motion options implemented, GREEN pending;41/Gordoth preserved. Initial test field-access compilation failure is not behavioral RED. Full Azure/pixels/callbacks/runes remain open.

## 2026-10-17 — Telash server jump-charge producer

[Server-source projection](investigations/follower-data.md#telash-jump-charge-source-producer--2026-10-17): genuine real-copy CLI mode RED→fba6fb2b7PASS10.725s including six legacy controls; exact34 source rows/ten fields/NULLs, duplicate-source atomic refusal and fresh-schema/excerpt-loader proof. COPY367→5f225535 with full sqldiff49b238f5 (one CREATE+34INSERTs only), quick_check=ok; canonical publication lead-owned/pending. Independent artifact PASS; full fresh import, native254 motion/callbacks and ordinary Azure/pixels remain open. Lead separately reports first Energy phase4105 gate504 landed1443/fmtOK, master950d25c7f.

## 2026-10-17 — Native field partial acceptance and hazard boundary

[Native fields](systems/spell-casting.md#aura-owned-native-fields--2026-10-17): retained5d native5PASS plus corrected-action936dc1421PASS37.90s, strict source/protocol/HEAD/published-input guards stable; no fresh6/6 claim. Passing fixtures enabled. Additional actual-target attachment/copy-departure and self/ally/other-copy controls are added but ignored pending scoped acceptance. Follower source-reach escape regression uses an actual native aura-created field, authored Garrick model/scale/reach and real Stockade navigation; its RED/GREEN and the hazard-consumer correction remain pending. No health/stat/geometry/default-reach changes.

## 2026-10-17 — Native Sprout initial periodic tick correction

[Native fields](systems/spell-casting.md#aura-owned-native-fields--2026-10-17): first5d4e3f6d packet5PASS/1FAIL121.30s, exact published inputs/source/HEAD guards stable. Failure was the fixture's zero-initial-tick expectation: actual70448 SpellMisc374523 Attributes5=524800/0x80200 includes TC `EXTRA_INITIAL_PERIOD`0x200. Production periodic behavior is correct and unchanged; corrected fixture pins the flag and expects1initial/1beforeperiod/2afterperiod/2afterexit, preserving no-reapplication and caster-filtered cleanup checks. Only failed case is rerun; five passing field/lifecycle/scale/curve cases are retained, not relabeled fresh6/6.

## 2026-10-17 — Rune startup and first natural Energy phase

[Coverage](investigations/follower-data.md#telash-first-energy-phase--2026-10-17): actual Rune startup0vs3 RED→7PASS/check/fmt at40f6e3977, lead497 landed1432 tests; source Rune data published3670 after494 with byte-identical five-insert re-run. First natural100Energy phase reaches genuine0vs1 Go RED51.96s, then4105c8f71 targeted10PASS106.76s/check6.59s/fmt0 with stable52-source/protocol/input guards; gate504 queued. Existing controlled-fixture health buffer is explicit; no ordinary budget, jump/rune interaction or pixel claim.

## 2026-10-17 — Native fields accepted and Rune source COPY prepared

[Fields](systems/spell-casting.md#aura-owned-native-fields--2026-10-17): genuine follower collision-envelope RED→six hazard PASS; requested fresh Azure/aura82PASS/check/fmt at931db8fe, with exact guards. [Rune producer](investigations/follower-data.md#telash-rune-source-projection--2026-10-17): source-preflighted CLI RED→real-copy process1PASS14.184s atf4e617611; d1→3670 COPY full-database delta is only five197204 content inserts, no canonical writes. Native Rune startup/phase/jumps and pixels remain separate open boundaries; lead owns publication.

## 2026-10-17 — Published Curve and native aura395 field implementation

[Native fields](systems/spell-casting.md#aura-owned-native-fields--2026-10-17): lead gate462 landed the maintained Curve publisher and published62817 readable rows; IDs121405/121406 remain unauthenticated/absent. Versioned private backup preserves143 Stockade/225 Azure actors. Four new native fixtures21f3045a compile and reach genuine shared missing-field ROOT failures87.99s; downstream scale/action/lifecycle assertions were not reached. Source-defined generic field implementation is committed before targeted GREEN/check/fmt/independent proof; no acceptance or native-pixel claim yet. This supersedes the earlier publication-pending state, without relabeling historical8f27/318 evidence.

## 2026-10-17 — Stomp stack expectation timing

[Native phase boundary](investigations/follower-data.md#stomp-expected-stack-timing--2026-10-17): unchanged current-master repro matches451; log-only diagnostic proves9 actual live stacks/5 corpses versus stale14 expectation. Test now samples Stomp-time stacks and independently accounts for all native Sprouts. Focused matching/nonmatching2PASS72.31s/check0 2m51s/fmt0, strict8f27/twenty-seven-source/HEAD/clean guards stable; original tolerance and production mechanics unchanged.

## 2026-10-17 — Native Curve type publication

[Curve input contract](../db2-active-inputs.md#native-curve-type-publication): maintained DB2 import/schema now retain installed Curve types/flags and build receipts. Process65651dac RED successfully imports but omits the table (1.197s); Curve-schema/active-root process3/3 passes4.106s at37a87ceb; COPY-only real-data receipt remains pending. Source encrypted exclusions are explicit in the contract, never default rows; scale consumer/native field fixture remain blocked/ignored until publication.

## 2026-10-17 — Native Fissure Sprout hit

[Ordinary Fissure lifecycle](investigations/follower-data.md#native-fissure-sprout-hit--2026-10-17): actual20s parent completion/real qualifying Sprout still-alive RED73.59s→native self-death/owner-stack GREEN1/54.27s, exact8f27/sixteen-source/HEAD guards stable. Initial silent-trigger SpellGo assertion invalidated and attempted implementation reverted before genuine RED; no generic cast-publication change. Test enabled; ordinary/cancellation and Brand/Stomp5 pass, corrected direct entry/front/copy/destination control1PASS58.82s/check0 2m27s/fmt0. Direct fixture samples immediate hit before unrelated cross-copy summon cleanup; production unchanged. Final exact8f27/seventeen-source/HEAD/clean guards pass; no fresh6/6 rerun claim. Visual159239/full Azure remain open.

## 2026-10-17 — Power-drain CalcValue source correction

[Power drain](systems/spell-casting.md#native-power-drain8--2026-10-17): original117 fixture expectation missed ExpectedStat base evaluation, CalcValue rounding and the initial base-variance phase; raw-base118 is not the full native result either. Native70448 zero class/coefficient fields still enter the pin's ExpectedStat else branch. Real UnitLevel10 yields3405=100,46453=278/floor80, and16094's native auto-DPS base284. Freshmaster887760ff genuineRED6FAIL/2PASS73.91s →8fd5f604/b8589ad6 GREEN9/9/60.29s/206.409s total, including metadata refusals and signed native-variance endpoints290/384/491. Hash-verified70448 selected inputs match runtime rows; old six passes remain historical, not source-parity proof.
## [2026-10-17] systems | Recuperate effect terminology correction

Corrected [immediate self hits](systems/spell-casting.md#self-hits-alongside-destination-missiles-2026-10-17):1231411 effect179 is `CREATE_AREATRIGGER`, properties38187, not a scene. Timing proof and runtime behavior unchanged; area-trigger behavior/visuals remain outside that proof.
## 2026-10-17 — Dead-caster native channel coverage

[Spell casting](systems/spell-casting.md#dead-caster-native-channel-coverage--2026-10-17): native55224 has1100ms prepare then8000ms channel/aura12. Concrete actual-death fixtures measure natural dead-start completion, caster-death interruption and Ghost-release interruption, including functional Stunned refusal and aura cleanup. Baseline9c13da6b already passes3/3 in21.80s (196.412s wrapper), so this is tests/docs-only coverage, no production fix or fabricated RED. Masterc23d4110 integrated433e8ae9 adds only unrelated124 pull dispatch/Omor tests; exact native cases and channel/life/Repop paths unchanged, so retain baseline3 and prior runtime221 within their scopes without replay. Workspace/all-target check37.538s/0warnings and fmt pass. Main-thread readability/command gate for medium test-only work; no new independent verification claim.

## 2026-10-17 — Dead-start cast progression

[Spell casting](systems/spell-casting.md#dead-start-cast-progression-and-death-interruption--2026-10-17): native5 and actual DeathPlugin create a dead player; active35058/passive53008 bars cancel incorrectly, NPC death fails immediate interruption, and post-corpse DEAD incorrectly consumes a new spell event. RED3pass/4fail6.07s. Server-only native spell snapshot now records the last observed life bit; NPC death/removal/despawn interrupt and DEAD actor events suspend. Initial7/7 GREEN12.10s; subsequent merged compilation finds two new-master tuple accesses, corrected without behavior changes. Source review yields genuine spirit-release bar RED (failed-release control passes) and SmartLifetime corpse-removal RED on actual native Brown Ram spawn/factory with unchanged source clocks. Missing Erland fixture is retained as invalid proof, not RED. Added source phase cancellation; current8bcb9970/master6c0cadc9/protocole9b80b8 passes merged221/0/0 in100.09s (329.193s wrapper): eleven new cases,205 prior and five named integration controls. Workspace/all-target check122.640s/0warnings and fmt pass; independent artifact/source/wiring/readability repair verification passes with command proofs main-owned. Dungeon cancel is source/code wiring only; Stockade control does not independently prove active-bar interruption. Failed-release control preserves this project's existing error contract, not whole TC Repop parity. Original cast times/radiusB/variance/resource budgets and original seven test bodies retained; no manual dead-state injection or corpse-delay extension.

## 2026-10-17 — Dead-start versus death-during-cast source qualification

[Spell casting](systems/spell-casting.md#non-instant-progression-qualification--2026-10-17): five read-only probes reconciled with primary files. Already-dead eligible casts progress on player/CORPSE event updates; living→dead interrupts existing generic/channel casts regardless of passive/AllowCast flag. Creature DEAD does not update Unit/events. Newly cached Spell.h confirms IsInterruptable is execution reentrancy, not cast-bar interruptibility. Runtime's unconditional health cancel cannot model both cases. Native35058 requires its actual RadiusB13, not A0; no runtime change/test/census promotion. Existing205+1 proofs retained, non-instant acceptance open.

## 2026-10-17 — Ordinary dead-caster eligibility

[Spell casting](systems/spell-casting.md#ordinary-dead-caster-admission-and-execution--2026-10-17): source passive/AllowCastWhileDead exceptions were missing at both admission and execution. Actual5 kills NPC before15007(active flag),199324(passive without flag) and direct15007 execution, RED2pass/3fail7.28s. Both boundaries now share the source predicate; living aura/unexempt dead116 controls retained. Mergede075b0cf/master42a7ab51/protocol1911f73 passes205/0/0 in44.11s (139.238s wrapper); existing unexempt queued-death control separately1/1 in5.71s. Check19.966s/0warnings, fmt and independent read-only artifact/source/wiring/readability pass; RED fixture byte-identical. No raw state/budget/row/clock/RNG edits. Instant eligibility is not cast-bar death progression or full ability/passive lifecycle proof.

## 2026-10-17 — Creature bool-trigger dead caster

[Spell casting](systems/spell-casting.md#creature-bool-trigger-dead-caster-admission--2026-10-17): TC permits a dead caster for a triggered cast without aura origin; bool FULL_MASK entry supplies no aura. Actual5→dead NPC→116/205406 gives RED2pass/2fail10.64s, while living bool116/dead ordinary116 controls pass. Removed only the unconditional bool-route living gate; metadata/recipient predicates and all other exemptions remain. Merged1a33d5c9/master1e782e6e/protocol523d4f9 passes200/0/0 in72.31s (148.091s wrapper): four new plus196 prior, including actual reflection and repaired resurrection controls. Workspace/all-target check15.734s/0warnings, fmt and independent read-only artifact/source/wiring/readability pass. Reviewer formatting hypothesis falsified by actual fmt check; RED fixture byte-identical. No raw health/stat/Mana/native flags/rows/clock/RNG/budget edits or whole-spell/death-trigger claim.

## 2026-10-17 — Normal explicit AllowDeadTarget

[Spell casting](systems/spell-casting.md#normal-explicit-allowdeadtarget-admission--2026-10-17): actual native5→corpse→9454 exposes unconditional dead-target refusal in player requests and ordinary/triggered creature routes, RED2pass/3fail12.93s. Normal validator now uses native dead-target eligibility while retaining living resurrection rejection and later predicates. All five native cases pass in merged192/3; the three failures were legacy synthetic7328 fixtures missing source corpse metadata. Replacing them with actual Redemption preserves native cost/time/range and requires the source out-of-combat precondition (common harness defaults combat true). Corrected3/3 pass5.37s;192 unchanged results retained, no195 replay. Native2008 living-friendly rejection separately passes1/1 in46.85s. Check19.945s/0warnings, fmt and independent artifact/source/wiring/readability pass. New native proof has no raw row/flag/health/mana/clock/RNG/budget edits or9454 queue parent. Legacy synthetic172 acceptance fixture remains explicitly non-native.

## 2026-10-17 — Shared full-trigger dead recipients

[Spell casting](systems/spell-casting.md#shared-full-trigger-dead-recipient-admission--2026-10-17): actual Death Touch5→present corpse→full-trigger Flame Shock188389 reproduces illegal admission and aura. Source IsAllowingDeadTarget predicate now gates explicit unit life; reflection/relation and other eligibility remain unchanged. Freeze9454 admission/persistence control corrected to match distinct native flags and passes before production. Merged2a90014e/masteraae714c5 passes183/0/0 in52.75s, including actual Frostbolt reflection and all165 previous controls plus5 script/9 missile cases. Workspace/all-target check182.777s/0warnings and fmt pass; independent read-only artifact/source/wiring/readability passes. Raw target-flag/corpse-selector branches have source/code proof, not separate runtime fixtures. No raw health/mana/row/flag/clock/RNG/budget edits.

## 2026-10-17 — Creature-script triggered aura metadata

[Spell casting](systems/spell-casting.md#creature-script-triggered-aura-metadata--2026-10-17): distinct creature bool-true branch bypassed all six implemented fields, native RED2pass/6fail9.38s. Shared caster/target checkers now surround existing recipient admission before success/launch; real producer matrix and generic-stun-exempt Frostbolt controls pass in RED. Merged ce17d43e/master d1957002 passes165/0/0 in49.88s plus5 creature-script controls in11.65s; workspace/all-target check10.323s/0warnings and fmt pass. Independent read-only artifact/wiring/source/readability passes; main confirms the actual diff and byte-identical RED fixture. No row/clock/budget/NoOp changes, no whole-ability/flag/legal-build claim.

## 2026-10-17 — Excluded caster aura-type admission

[Spell casting](systems/spell-casting.md#excluded-caster-aura-type-admission--2026-10-17): actual853 zero-amount stun supplies native449285 excluded12. RED1pass/2fail17.32s proves wrong ordinary error priority and incorrect full-trigger admission; cancellation/native expiry control passes. Shared caster field predicate and metadata-before-generic-CC priority pass current157/0/0 in98.03s (four new+153 adjacent), workspace/all-target check18.988s/0warnings, fmt and independent artifact/wiring/source/readability. That proof did not cover the separate creature-script triggered shortcut; its subsequent qualification is recorded above. Required caster types, midcast/26-positive/whole-ability and legal-build acceptance remain separate.

## 2026-10-16 — Queued effect64 recipient admission

[Spell casting](systems/spell-casting.md#queued-effect64-recipient-admission--2026-10-16): actual470411→188389 queued child and actual Death Touch5 reproduce payment and aura on a present corpse, RED0pass/2fail33.06s. Cost-retaining child now runs maintained normal recipient validation before cost/payment/hit planning; generic full-trigger/reflection helpers unchanged. Current9c01c758/master53a59762/protocol6b06680 passes merged128/0/0 in27.84s (314.078s wrapper), two new+126 prior; workspace/all-target check39.710s/0warnings and fmt pass. Independent artifact/wiring/readability inspection's returned-recipient finding was corrected, then main verified current128/check. Prior missing-PvP-bracket protocol compile log retained as invalid proof. No raw health/mana/clock/budget edits.

## 2026-10-16 — Queued effect64 payment phases

[Spell casting](systems/spell-casting.md#queued-effect64-payment-phases--2026-10-16): source clears child power/reagent ignore bits while native470411 retains parent resources via Attr6=0x20. Actual Flame Shock chain RED1pass/3fail23.12s exposes both wrong payment phases and vanished-target payment; generic full-trigger cost bypass control passes. Shared payment boundary and distinct cost-retaining child route pass merged126/0/0 in58.89s (358.280s wrapper), four new+122 adjacent; workspace/all-target check92.274s/0warnings, fmt and independent artifact/wiring/readability pass. Prior missing-concentration-protocol compiler log retained as invalid proof; landed protocol5e4af62/server9eb0b706 cohort integrated756e8352. No native rows/stat/mana/clock/RNG edits.

## 2026-10-16 — Native queued effect64 integration

[Spell casting](systems/spell-casting.md#queued-effect64-and-required-caster-trigger-integration--2026-10-16): actual native0/1ms parent calls expose inline-child RED0pass/2fail11.81s. Native-delay FIFO callbacks now defer64 without clock edits or early aura activation; required-caster guard and exact retained positive/negative fixtures restored. Current merged106 batch105 passes; its sole due-order fixture used an NPC owner-only log. Test-only correction to the actual player owner's receipts passes1/0/0 in11.00s. Preserve105 unchanged+corrected1 rather than rerunning106; required producer/Stack Timer positives are restored. Workspace/all-target check223.375s/0warnings, fmt and independent artifact/wiring/readability pass.32/140, native64 costs/reagents and general launch-phase parity remain separate.

## 2026-10-16 — Triggered excluded-caster eligibility and required-ref blocker

[Spell casting](systems/spell-casting.md#triggered-excluded-caster-aura-eligibility--2026-10-16): actual146481→146480/146543 RED retained byte-identically; shared exclusion predicate wired before triggered launch. Current merged77/0/0 in45.36s includes two new plus75 prior cases; workspace/all-target check198.441s/0warnings, fmt and independent read-only artifact/wiring/readability pass. Complete required-caster guard on separate branch fails four positive native chains. Subsequent five-way source qualification corrects the initial registration hypothesis: native64 queues children with0/1ms delay at launch, whereas Rust casts inline before parent publication. Pinned EventProcessor callbacks are non-reentrant; application/activation separation alone does not justify a new identity store. Retain failing fixtures/branch, no marker/pregrant/early activation or pending bypass; required field remains open.

## 2026-10-16 — Triggered explicit-target aura eligibility

[Spell casting](systems/spell-casting.md#triggered-explicit-target-aura-eligibility--2026-10-16): native412677→412680 lifetime RED2pass/1fail11.95s exposes forbidden refresh; full-mask flag claim corrected against source. Explicit target checker wired before triggered launch, relation behavior untouched; merged75/0/0 in25.73s includes three new and72 adjacent cases. Workspace/all-target check26.990s/0warnings, fmt and independent read-only artifact/wiring/readability pass. Caster-trigger/auto-self/state/full flag gaps remain open.

## 2026-10-16 — Native target aura-type admission

[Spell casting](systems/spell-casting.md#native-target-aura-type-admission--2026-10-16): ordinary scripted-unit required69/39 and excluded485 RED2pass/5fail6.21s precede the shared type consumer. Native shield, recipient and expiry controls; no invented indicator client ability or unhandled effect registration. Merged master0bf173f4 as2db43289; current66/0/0 in28.63s includes seven new and59 adjacent/LOS controls. Workspace/all-target check51.794s/0warnings, fmt and independent read-only artifact/readability verification pass. Player/completion type transitions remain source/shared-wiring proof; caster/area/full-trigger/health-state boundaries remain open.

## 2026-10-16 — Health aura-state source qualification

[Investigation](investigations/health-aura-state-admission.md): two waves of five bounded read-only explorers reconciled against primary files/native70391. Exact thresholds, seven IDs, packet-before-unit/queued-after-own-unit phases and passive gain/loss qualified; incorrect numeric, missing-data, write-through and broad fractional-stats claims rejected. Missing pinned map/session/opcode sources cached with SHA receipts. No health code or runtime proof; integration/ownership/passive-source seams remain open while independent ledger work continues.

## 2026-10-16 — Native required caster aura-spell admission

[Spell casting](systems/spell-casting.md#native-required-caster-aura-spell-admission--2026-10-16): actual Voidform/Sheilun casts and two-second cloud loss RED3pass/6fail9.05s precede the shared caster identity check. Positive heal, recipient/expiry/NPC/full-trigger controls included. Initial23/24 exposed a fixture's incorrect empty-GCD expectation; the actual native205406/category133/1500ms packet is now asserted exactly, without production changes. Current merged52/0/0 passes19.79s, cold rebuild416.895s/0warnings; workspace/all-target check132.981s/0warnings and fmt/manual Rust readability pass; canonical source-table publication remains lead-owned and absent at final read-only inspection. Target322109→1221044 notification producer remains unqualified and is not invented.

## 2026-10-16 — Native excluded aura-spell admission

[Spell casting](systems/spell-casting.md#native-excluded-aura-spell-admission--2026-10-16): shared caster/target exclusion references now checked at normal admission/completion; native4pass/5fail RED precedes implementation, actual450ms blocker gain/removal included. GREEN37/0/0 in8.88s includes nine new and28 native controls; source Stealth-child closure retained. Master9aca3c1a integrated; current merged37 passes7.71s, workspace/all-target check27.129s/0warnings and fmt/manual Rust readability pass. Target-midcast/charge/GCD lifetime details remain source/code proof, distinct from the actual caster450ms fixture. Required refs/states/types and other implicit targets remain open; no protocol or native-row edits.

## 2026-10-16 — Native aura restrictions metadata loader

[SpellInfo](systems/spell-info.md#native-aura-restrictions-metadata--2026-10-16): dropped metadata/schema RED0/4→GREEN4/0/0, retaining twelve native state/spell/type IDs through the production store. Difficulty0, sparse absence, orphan and missing-source boundaries covered; source publication precedes deployment. Merged39 native controls pass11.45s after owned-slot maintained import and exact10675-row equality; fmt/readability and workspace/all-target check159.051s/0warnings pass. State production/admission and canonical publication remain open.
## 2026-10-17 — Queued64 child NoPower qualification

[Affordability fixture](systems/spell-casting.md#queued64-child-affordability-qualification--2026-10-17): prior queued64 stack plus effect8, unchanged enemy3405 exhaustion before native470411→188389 dispatch, living recipient and FULL_MASK control. No raw resource/health/clock edits; first baseline019005d6 already17/17 GREEN54.47s/289.908s total, exact regen-control equality, no production fix or invented RED.

## 2026-10-17 — Native power drain8

[Power drain](systems/spell-casting.md#native-power-drain8--2026-10-17): pinned primary-power/integer/actual-removal/Amplitude implementation, unchanged native3405/46453/16094. Four real missing-drain REDs and two exclusion guards; stale-protocol compiler failure is not RED. Native pools/costs retained; merged26f5332a native GREEN6/6 in18.34s/146.040s total, no whole-ability acceptance.
## 2026-10-17 — Sprout death stack consumption

[Native Sprout deaths](investigations/follower-data.md#sprout-death-stack-consumption--2026-10-17): original-caster-first fourteen-child RED→GREEN1/45.34s; actual owner stack consumption, final replicated removal and duplicate-corpse exclusion. Timer/amount assertions and integration gates pending.
## 2026-10-18 — Tidal Wave X2309 bounded recipient

[2309 source/proof](../pet-battle-effect-coverage.md#tidal-wave-x--implementation2026-10-18): source70448 three14-pointAquatic24rows then existing85state69 clears bothteamsobjects. Parameterize319controls/renamefamilyteam_wave, not duplicate; genuine554a81c3ccore4+ownedSwoggy1unsupportedRED→one-line991c225c2registry→core4/owned1GREEN.9active/eachmiss/deadreserve/bothteamRocket294purgedespiteallmisses/Healthy577preserved; legalSwoggy3570/4power60/max321deals56/Rabbit91/alliesconserved. All10sourcefixturesreconcile. Integrated741b4c231lib+maincompileblocked14upstreamE0308stalespawn_creatureOption<Entity>callersinauras/tests.rs+smart_escort_tests.rs,noopetbattlepatchthere. Independentfinal lib163/census1841/2829=65.08%/1031graphsPASS/shared11acleanbeforeafter (+5/+1/no removals),otherrejections/70391retained;mainstillrequired/notnarrowed. Normalpackagecheck85c7783a71m10/no warnings/workspacefmtPASS/shared11acleanbeforeafter;gate511landsowningcallerrepair,b3cdc75d3mergesONLY2testfixfiles;onlymissingmain39PASS/shared11acleanbeforeafter,lib/census/check/fmtretainedwithoutduplicates. Native-after-landing pending. Related649assetprep18:leadpublishedGraves10assets+2aliases+fragment;all13hashes/privatecanonicalslotlinks/freshheadlessstrict3meshloaderPASS, notlivebattleproof.

## 2026-10-17 — BONESTORM!1761 thirty-percent cost

[1761 source/proof](../pet-battle-effect-coverage.md#bonestorm1761--implementation2026-10-17): source70448 three13-point24rows then30%62casterMAX, only1761registry/existing649guard extensionc8a1a99f9. Genuine065586471core5+ownedWhirly1unsupportedRED→core5+owned2GREEN (Graves649 retainedcontrol). Parameterizeexisting4controls/newlastenemyfaintcase insteadofduplicatecases;9active/eachmiss/deadreserve/allmisscost/MAX10003notcurrent5000orenemy4711/type controls. Lastenemy1HP damage1 then self3000/Winner0/caster7000. LegalWhirly2986/4power60/max399 deals52/Rabbit95,pays119/self280.1762/50%deferred becauseonlyownerBoneshard1963 lacks source states, no fabrication. Integrated5fd087534lib163/main38PASS/shared019cleanbeforeafter;freshall10table-reconciled70448census1836/2829=64.90%/1030graphs(+4/+1/no removals),historical70391/1762/otherrejectionsretained. Changed-lineRustreadabilityclean;a7020be50packagecheck23.83s/no warnings/workspacefmtPASS/shared019cleanbeforeafter. Onlydocs/censusafterintegratedproof; native-after-landing pending,ordinaryselfaccuracy/lowHPpassives/Retailtimingunqualified.

## 2026-10-17 — BONESTORM bounded self-cost recipient

[649 source/proof](../pet-battle-effect-coverage.md#bonestorm--implementation2026-10-17): active70448 three13-point24 rows then10%-maximum-health62. Core4 actualREDd1454c008; firstownedfixture assumes wrongpower21, corrected via namedstates to23/max214 in2f6030ee0 and genuineownedroutingRED before production. Only649registry+62casterrecipient in a9062da8b; core4/owned1GREEN all9active/eachmiss/deadreserve/allmissstillcost/casterMAX10003notcurrent3000orenemy4711/untypedcost1000vsstrong/weaktypes. LegalGraves1639/2slot2 deals27/Rabbit120,pays21/self193. Noopcode/math/generalrouting/passiveredesign; ordinaryselfaccuracy/low-healthpassive/Retailtimingunqualified. Integrated466cfbee3lib162/main37PASS/shared019cleanbeforeafter;freshall10table-reconciled70448 census1832/2829=64.76%/1029graphs (+4/+1/no removals),historical70391/allotherrejectionsunchanged. Changed-lineRustreadabilityclean;3b58231cd packagecheck54.03s/no warnings/workspacefmtPASS/shared019cleanbeforeafter. Onlydocs/censusafterintegratedproof. Gate495 lands162+59/fmt, then native2026-10-18 bothactualskinslegalGraves1639/2physical649:Modern26enemy/Rabbit121+21selfcost/Scratch44/caller214→149;Forever29/Rabbit118+same21/Scratch38/retained149→90. FourindividuallyinspectedfullHDoriginalstoday18,all4phasestwoframedmodels/ExtractedOnly/tripwire0,geometry91/101onlyHPfills/timedfloats. LocalCASCGraves10assets+2aliases+fragmentleadpublication13hash+slotlinks/actualstrict3meshloaderPASS. Freshactual2c27server/seeder+ee951engine/shared11a cleanbeforeafterall3builds,11avs019DOCSONLY;all3hashesunchangedthroughparallelTidalCPU/native. Bothphysicalforfeitsclose/allownedruntime stops/5961unbound/sliceinactive/windowclear. Native reserves/misses/selfavoidance/lowHPpassives/independentRetailroundingortimingunproved,CPUcontrolsseparate.

## 2026-10-17 — Magma Wave bounded recipient

[319 source/proof](../pet-battle-effect-coverage.md#magma-wave--implementation2026-10-17): active70448 three7point damage rows then existing85state69 object clear; one-line41737ef9 registry addition only. Actual651ccfd7core4+owned1 routingRED,core4GREEN coversall9active/eachmiss/deadreserve/both-team Rocket294 removal despite allmisses/Healthy577preservation. Owned MoltenHatchling428/4power47 expected23 incorrectly omits Elemental-v-Critter. Test-onlye6568abf correction yields15/Rabbit132, integrated5ee64433lib158/main36PASS/shareda566cleanbeforeafter. Freshall10table-reconciled70448 census1828/2829=64.62%/1028graphs (+5/+1/no removals), historical70391unchanged/otherrejectionsretained. Packagecheck429b17e0 passes1m23/no warnings/workspacefmtPASS/shareda566cleanbeforeafter; changed-line readability no violations, onlydocs/censusafterintegratedproof. Gate479 lands158+58/fmt, then strictboth-skinlegalMoltenHatchling428/4 physical319 deals17/16damage/Rabbit130/131; own295→267→237 afterScratch28/30. FourindividuallyinspectedfullHDoriginalstoday17,all4phasestwoframedmodels/ExtractedOnly/tripwire0;geometry94/100onlyHPfills/timedfloats. Existingcanonical6hashes+links/actual1meshstrictloaderPASS,no publication. Actualfrozenaee2server/cf1engine/shareda566 +aee2428seeder/shared019; leadapprovescoherentreuseafterwiremove,neverlabelworkingc444asbinary. All3hashesunchangedthroughparallel649/1761CPUgates/native;firstPSI3.14HOLDbeforestartup,retryADD0.88thenfirstlaunchsucceeds. Bothphysicalforfeitsclose/runtime stops/5961unbound/windowclear. Nativeobjects/reserves/misses and RetailtimingVFX remainunproved,CPUcontrolsseparate.

## 2026-10-17 — Primal Cry paired failure chains

[Bounded920/154](../pet-battle-effect-coverage.md#primal-cry--implementation2026-10-17): active70448 damage/slow pairs,25% slow/four authored rounds, legal Meadowstomper1446/2. Actual unsupported-routing core4/owned1 RED; old parallel base then hits already-fixed GroupTooLarge consumer drift (no PvP patch). Aligned routing passes all-hit/refresh/owned but exposes real sticky whole-turn failure bleed in miss/dead controls; only920 pair starts reset in fb8d918e. Corrected core4/owned1 GREEN,774 preserved/2064 retained negative, duplicate negative consolidated. Integrated5cd78051 lib154/main35 pass;049a40c6 packagecheck3m08/no warnings/fmt pass, clean shared2a1ed035 before/after both commands. Fresh all-ten-table-reconciled70448 census1823/2829=64.44%/1027graphs (+6/+1/no removals), historical70391 untouched. Gate465 lands154+57, then both-skin legal Meadowstomper1446/2 physically920:28damage/Rabbit119,154slow3rounds/CD3, own211→182→152 after29/30Scratch. Four individually inspected fullHD originals today17; all4phases two framed models/stricttripwire0; geometry93/105 onlyretainedfills/timedfloats. Existingcanonical4hash+links/1meshloaderPASS, no publication. Actualnativeb87013ad/cf1c4d6f7/shareda566 cleanbeforeafter; initial2a1→a566 wiremove forces changedserveralignment/rebuild, seeder exactrange retained with unchangedPetJournal. Both physicalforfeits close/runtime stops/5961unbound/windowclear. Native reserves/misses/refresh/expiry and independent Retail timing/VFX remain unproved.

## 2026-10-17 — Rapid Fire two-wave recipients

[Bounded774](../pet-battle-effect-coverage.md#rapid-fire--implementation2026-10-17): active70448 six rows repeat the same three enemy slots for points4 then3;2064/920 remain rejected. Actual core4/owned1 routing RED precedes minimal productione04f5e10 and six focused pet GREEN cases, including independent wave misses/first-wave faint without retarget and real Peddlefeet122/10. Three Hunter controls are separate. Integrated69653970 lib150/main34/check23.64s/no warnings/fmt pass; fresh reconciled70448 census1817/2829=64.23%/1026 graphs (+6/+1/no removals), with2064/920 still rejected. Subsequent changes docs/census only. Lead gate453 lands150+56 before both-skin legal Peddlefeet122/10 native27+19/Rabbit101 and36critical+20/Rabbit91; capped ordinary Humanoid17/18 recovery preserves653HP. Four inspected fullHD originals under `/syncthing/AgentShared/2026-10-17/petbattle-effects/`; all four phases two framed models/strict tripwire0, geometry91/101 only enemy fills/timed floats. Existing8 canonical assets/5-mesh loader pass, no publication. Frozen7e4d7acc/275d6f804/shared3084 hashes unchanged through parallel Primal tests. Both physical forfeits close, owned processes stop/window clears; native miss/first-wave faint/reserves and Retail timing/VFX remain separate.

## 2026-10-17 — Starfall/Moon Tears recipient routing

[Bounded258/979](../pet-battle-effect-coverage.md#starfall-and-moon-tears--implementation2026-10-17): exact active70448 seven-row graphs, source-school distinction and existing Moonlight reuse. Actual routing RED4 plus legal Cosmos owned RED1 precede9b958668 slot/cap/dead/miss/owned GREEN; first weather case exposed only a fixture9-versus-existing8 remaining assumption, corrected test-only9c88c925. Corrected focused weather1/1 passes ataf89ac7c; retain unchanged core4+owned1 GREEN at9b958668. Integrated5d23a60d lib145/main33/check1m36/no warnings/fmt pass; fresh reconciled70448 census1811/2829=64.02%/1025 graphs (+14/+2, no removals). Only subsequent docs/census changes; no redundant broad rerun. Lead gate438 lands145+55 before both-skin native Cosmos1972/1:9 damage/9 healing×3/increased reserve bars/Moonlight8. Four individually inspected fullHD originals under `/syncthing/AgentShared/2026-10-17/petbattle-effects/`; all phases two framed models/strict tripwire0, geometry91/110 only HP fills/timed floats. Existing7 canonical assets/3-mesh strict loader pass, no publication. Both physical forfeits close, owned processes stop/window clears.979 native and independent Retail timing/VFX remain unproved. No other ambiguous selectors admitted.
## 2026-10-18 — Mycomancer664 triggered hearthstone acceptance pending

[Exact acceptance and limits](../toy-effects14-coverage.md#mycomancer664-triggered-hearthstone-acceptance-pending-after-st526-2026-10-18): test/spec37a6c0598 adds learned264367/actual1299014 ten-second root→1286909/252 saved-bind, shared2309/1800000ms charge and Corewarden refusal. No child-direct substitute or change to25 direct cases/helper. Format/manual readability only; no own compilation/test job, behavior pending lead gate afterst526. No runtime/data/native or missing-bind/cross-map/natural-recharge claim.

## 2026-10-18 — st526 Toy scope landed

[Landing and limits](../toy-effects14-coverage.md#st526-toy-scope-landed-2026-10-18): lead confirms server masterce0fec81c contains feature38d087230/docsfaab75f7e; both ancestry checks pass. Toy successor scope complete with194/0; eight server failures belong to other heads. No native/all1166 gameplay or whole-workspace green claim. No own runtime/data/test jobs.

## 2026-10-18 — st526 Toy CPU cases pass; full server remains RED

[Exact current proof matrix](../toy-effects14-coverage.md#st526-server-result-toy-cpu-acceptance-passes-full-workspace-remains-red-2026-10-18): candidate2ec2e80bc combines feature38d087230/fix31f7e9216; server3909 passed/8 failed/124 ignored2770.80s/EXIT101/FMT0. Toy namespace194/0 includes Royal618/shared-five, old19/Oomgut,21 casters,13 objects,17 stationary and25 hearthstones; Akunda cursor/cooking/lifetime and drift controls now pass. Seven missing-tdb_jump_charge_params startup/orientation failures and Slagmaw119433 damage assertion keep overall integration RED. Engine phase pending; no own runtime/data/test job or native/full1166/landing certificate.

## 2026-10-18 — Royal Visage618 misc0 acceptance pending after513

[Exact pending acceptance](../toy-effects14-coverage.md#royal-visage618-misc0-acceptance-pending-execution-after513-2026-10-18):0305a0e91/ce32419fb adds actual245631/1239480/236348/misc0/display121918 learned-use/source-deadline/ordinary ten-minute expiry/restoration/aura-removal/health and thirty-minute recovery-refusal test. Old19 misc8/helper, Oomgut and shared-five baseline unchanged. Format/manual readability/diffcheck pass; no local compile/execution, successor AFTER513 required. No native/full-reuse/selection/quest certificate, runtime/data change or own heavy jobs.

## 2026-10-18 — Shared2513 five-identity baseline pending after513

[Exact new baseline matrix](../toy-effects14-coverage.md#shared2513-five-identity-baseline-new-acceptance-pending-after513-2026-10-18):1c143bea6 adds separate learned-use/model/hour-duration/deadline/health/ten-minute-category/shared-sibling-refusal cases for Pango/Saptor/Masquerade/Idol/Badge. Old19 registry/helper and Oomgut unchanged. Format/manual readability/diffcheck pass; no local compile/execution, successor AFTER513 required. No cancel/hour-expiry/natural-reuse/distribution/native proof, mixed Memento admission or runtime/data change; queued64 repair remains assigned elsewhere.

## 2026-10-18 — Full1166 toy metadata availability, not runtime proof

[Exact availability matrix](../toy-effects14-coverage.md#full1166-metadata-availability-exact-absences-not-failure-verdicts-2026-10-18): all1166 runtime IDs inventoried, ten extras excluded.29 source ItemSparse/runtime-content absence sets match;15 lack TriggerType0; Rezan272071/Stone462924 lack SpellMisc, unnamed1271857 lacks name/misc/effects; zero dangling ItemEffect links. No invented mechanics, automatic importer/code failure inference or revoked existing behavior proof. No runtime/data/native change or own heavy jobs;513 remains integration RED.

## 2026-10-18 — Toy gate513 remains RED at queued destination boundary

[Exact gate matrix](../toy-effects14-coverage.md#gate513-acceptance-helpers-pass-queued-trigger-destination-regression-keeps-integration-red-2026-10-18):311 passed/1 failed/1 ignored914.05s/EXIT101, client9/FMT0. Current19-disguise-plus-Oomgut/21-caster/13-object/17-stationary/25-hearthstone acceptance helpers completed OK; integration failed because Akunda child fire used caster instead of explicit parent cursor. New queued64 payload/fresh-child reconstruction drops parent `cast_destination` while keeping item level. Existing assertion not weakened; runtime fix proposed to lead, not implemented here. No new toy landing/native/full1166 certificate or own heavy rerun.

## 2026-10-18 — Toy tail metadata reconciliation, not runtime completion

[Exact proof matrix](../toy-effects14-coverage.md#tail-reconciliation-and-source-only-names715720-metadata-proof-not-runtime-completion-2026-10-18): all151 authorized IDs after checkpoint559/228966 accounted for in31 active70448 source capsules, no missing/unexpected/provenance/order gaps. Source1176/runtime1166 and same ten excluded extras unchanged; final six named boundary-only, no import/admission/acceptance. Toy hash matches checkpoint559. Lead513 queued7099f3a38 covers current acceptance/fixture correction; no result/new toy landing. Metadata accounting does not certify whole1166 review/behavior/native presentation; no own heavy jobs or runtime/data/native change.

## 2026-10-18 — Toy raw source710–714 last Full1166 IDs, not runtime completion

[Exact source matrix](../toy-effects14-coverage.md#source710714-last-full1166-ids-weighted-badge-windwell-and-three-excluded-source-only-names-2026-10-18): last Full1166 IDs280419/280541 preserve Badge weights4/2/1/2/1/shared2513 and missing Windwell271783/base0/both87 targets. Three following names280820/280821/280822 explicitly SOURCE-ONLY/OUTSIDE1166, no import/admission/acceptance. Metadata-tail inspection does not certify1166 behavior/landing. Lead513 queued7099f3a38 after separate master compile repair efd627da1 covers all current acceptance and fixture correction; no result/new toy landing observed. No runtime/data/native change or own heavy jobs.

## 2026-10-18 — Toy source705–709 mixed state and qualified movement boundaries

[Exact source matrix](../toy-effects14-coverage.md#source705709-mixed-memento-mask-qualified-swim-retained-venomous-banner-and-story-view-boundaries-2026-10-18): preserve Memento56+370/shared2513, unaudited Mask, qualified water/8993 swim with literal58/base1200 versus descriptive300%, existing Venomous Banner fixture/explicit time-leap scope and Storytime260/2565/7. No guessed curse/speed normalization or generic presentation substitute. Metadata only; no runtime/data/native change or own heavy jobs, lead successor pending.

## 2026-10-18 — Toy source700–704 exact identity and recipient boundaries

[Exact source matrix](../toy-effects14-coverage.md#source700704-unnamed-homebind-metadata-current-camp-recall-shared-idol-corpse-haunt-and-targeted-star-boundaries-2026-10-18): unnamed276370/1299513 retains10s252/shared2309 but lacks item identity; Lightveil retains actual current camp/9076, Idol shared2513 short recovery, Jaktu corpse226/15s/trigger0 and Otoola friend/foe missile32→4. No invented name/coordinates/child, borrowed helper or self-recipient substitute. Metadata only; no runtime/data/native change or own heavy jobs, lead successor pending.

## 2026-10-18 — Toy source695–699 pending Corrosive acceptance

[Exact source matrix](../toy-effects14-coverage.md#source695699-pending-corrosive-victory-and-mixed-effigy-short-category-mask-stand-companion-command-boundaries-2026-10-18):ad2f092f7 appends275988/1298433/1800000ms to unchanged caster helper, now21 identities/old20 retained; format/manual readability/diffcheck pass, behavior pending successor. Preserve Effigy1250/transform/200ms area chain and NPC0.6, Mask shared2513 shorter recovery, Stand47/138/base0 and Companion150 critter authority. No source-only admission, runtime/data/native change or own heavy jobs.

## 2026-10-18 — Toy raw source690–694 pending Pearl acceptance and explicit exclusion

[Exact source matrix](../toy-effects14-coverage.md#source690694-pending-pearl-disguise-throne-loot-bot-kill-rain-and-one-excluded-source-only-identity-2026-10-18):83c3de3d2 appends Full1166 Pearl274921/1295623/265732/ten displays/5min/1hr to unchanged disguise helper, now19 rows plus separate Oomgut20 identities; format/manual readability/diffcheck pass, behavior pending successor. Raw block has four runtime identities plus275039 Curse of the Aqir, named SOURCE-ONLY/OUTSIDE Full1166 without import/admission/acceptance. Throne/base0, resource/corpse loot bot and player-kill rain remain gaps. No runtime/data/native change or own heavy jobs.

## 2026-10-18 — Toy source685–689 shared category and exact placement boundaries

[Exact source matrix](../toy-effects14-coverage.md#source685689-shared-saptor-category-missing-naptent-retained-umbral-banner-and-scripted-starfish-boundaries-2026-10-18): Saptor shares2513 with Pango, keeps one-hour aura/ten-minute recovery and122805 weight0.1. Missing Nap Mat and target22/base0 Tent remain gaps; existing Umbral Banner fixture retains explicit time-leap expiry scope. Starfish retains77 plus permanent child4/PreventionType4. No generic prop/bonus/independent timer, runtime/data/native change or own heavy jobs; lead successor pending.

## 2026-10-18 — Toy source680–684 mixed props, camera and category timing

[Exact source matrix](../toy-effects14-coverage.md#source680684-mixed-props-staff-image-camera-guides-and-short-recovery-pango-boundaries-2026-10-18): missing Hourglass/Bench retain16 payloads and distinct cancellation/aura shapes; Staff source remains instant despite channel flavor. Camera retains293/60/321 and mutual164 guide removals. Pango's one-hour aura/shared2513 ten-minute recovery remains unverified, not forced into long-recovery helper or labelled runtime unsupported. Metadata only; no runtime/data/native change or own heavy jobs.

## 2026-10-18 — Toy source675–679 game, inventory, mixed rainbow and hologram boundaries

[Exact source matrix](../toy-effects14-coverage.md#source675679-black-hole-game-key-inventory-rainbow-chain-retained-pillow-and-dual-hologram-boundaries-2026-10-18): actual430 feeding game and all-shard conversion remain gaps; rainbow retains241/56misc12/42/409/proc2000ms, child launch and both164 removals. Existing11-second Pillow fixture retained; both259655/259656/base0 stacking holograms missing, holiday recovery unresolved. No generic success/free key/pure disguise/static gnome, runtime/data/native change or own heavy jobs; lead successor pending.

## 2026-10-18 — Toy source670–674 pending Corewarden acceptance

[Exact source matrix](../toy-effects14-coverage.md#source670674-pending-corewarden-hearthstone-and-corpse-ritual-mail-viewer-current-location-travel-boundaries-2026-10-18):f30ff0f2c appends actual slot1 265100/1273401/source10s/shared2309 to unchanged saved-bind helper, now25 hearthstones/old24 retained; format/manual readability/diffcheck pass, behavior awaits lead successor. Preserve corpse25 ritual/-100/two9000 payloads, missing257683 mail/base0/shared2066, Brann233/base550 and qualified current-Abundant3+77 travel. No guessed bodies/destinations or runtime/data/native change; no own heavy jobs.

## 2026-10-18 — Toy source665–669 pending Dominating acceptance

[Exact source matrix](../toy-effects14-coverage.md#source665669-pending-dominating-victory-and-missing-metadata-qualified-flare-corpse-flagrod-boundaries-2026-10-18):9470e4859 appends264413/1271973/1800000ms to unchanged caster helper, now20 identities/old19 retained; format/manual readability/diffcheck pass, behavior awaits lead successor. Preserve unnamed264368/1271857 metadata gap, qualified32/64/32 flare/missing257541/base0, enemy-corpse22/93 flag and Rod25/395/shrink/two5000 payloads/permanent corpse transform. No invented actor/eligibility/timing, runtime/data/native change or own heavy jobs.

## 2026-10-18 — Toy source660–664 pending Preyseeker acceptance

[Exact source matrix](../toy-effects14-coverage.md#source660664-pending-preyseeker-hearthstone-and-minipet-leash-tap-state-mixed-mushroom-triggered-homebind-boundaries-2026-10-18):3b3f52940 appends263933/1270814/source10s/shared2309 to unchanged saved-bind helper, now24 hearthstones/old23 retained; source PreventionType4 retained without broader prevention certificate. Format/manual readability/diffcheck pass; behavior pending lead successor. Target90 leash, tap-state lens, missing mixed76/179 mushroom and actual Mycomancer1299014/64→1286909/252 remain gaps. No substituted child root, runtime/data/native change or own heavy jobs.

## 2026-10-18 — Toy source655–659 pending Naaru acceptance

[Exact source matrix](../toy-effects14-coverage.md#source655659-pending-naaru-hearthstone-and-skip-rock-mixed-mushroom-retained-catch-periodic-voidwalker-boundaries-2026-10-18):8ead527f5 appends263489/1270583/source10s/shared2309 to unchanged saved-bind helper, now23 hearthstones/old22 retained; format/manual readability/diffcheck pass, behavior awaits lead successor. Preserve skip-rock nonzero trigger/base1, qualified50+179 mushroom/missing404325/40621, retained3300ms Valdekar fixture and periodic100ms/trigger0/three-voidwalker multiplicity. No guessed mechanics, runtime/data/native change or own heavy jobs.

## 2026-10-18 — Toy source650–654 mixed effect and recipient boundaries

[Exact source matrix](../toy-effects14-coverage.md#source650654-mixed-parachute-cycle-script-retained-wine-photo-and-ash-boundaries-2026-10-18): parachute preserves five root slots/two139/negative98/area8537/category1470; Cycle preserves three script base points; Wine's existing19-caster identity retained. Photo preserves nonzero trigger→250 and proc101/mask0/8; Ashen Horn preserves both140 edges including35/395 child. No new acceptance, runtime/data/native proof or own heavy jobs; successor pending after cancelled509.

## 2026-10-18 — Toy source645–649 pending Lightcalled acceptance

[Exact source matrix](../toy-effects14-coverage.md#source645649-pending-lightcalled-hearthstone-and-jumpable-monument-gold-fountain-boundaries-2026-10-18):57aa5b0c5 appends257736/1261979/source10s/shared2309 to unchanged persisted-bind helper, now22 hearthstones/old21 retained; format/manual readability/diffcheck pass, behavior pending lead successor. Three missing jumpable monuments retain base0/distinct casts; missing255051 gold fountain retains64→425841/effect65/120s/category59 with actual payment contract unresolved. No collision-free substitute, guessed multiplicity/cost/reward, runtime/data/native change or own heavy work.

## 2026-10-18 — Toy source640–644 pending Dredger acceptance

[Exact source matrix](../toy-effects14-coverage.md#source640644-pending-dredger-disguise-and-linked-companion-mixed406-boundaries-2026-10-18):2cc3d9ec3 appends256893/1260864/254071/four displays/10min/15min to unchanged disguise helper, now18 rows plus separate Oomgut19 identities; format/manual readability/diffcheck pass, behavior awaits lead successor after cancelled509. Exodar linked428 plus repeated495 edges, missing traveling cat and Seed406/20/target0 versus Steward406/294/self remain exact gaps. No static actor/pure-transform substitute, runtime/data/native change or own heavy jobs.

## 2026-10-18 — Toy source635–639 pending Potatoad acceptance

[Exact source matrix](../toy-effects14-coverage.md#source635639-pending-potatoad-disguise-and-pet-command-bundle-egg-arcantina-boundaries-2026-10-18):e29972e08 appends252265/1254376/251887/five displays/10min/20min to unchanged disguise helper, now17 rows plus separate Oomgut18 identities. Format/manual readability/diffcheck pass, behavior pending lead successor.509 cancelled before start for separate master compile interaction; not a toy RED. Pet90/87 move/stay, missing bundle251069/base0, unaudited Egg and distinct Arcantina script payloads1255786/1246474 remain gaps. No runtime/data/native change or own heavy jobs.

## 2026-10-18 — Toy source630–634 exact mixed contracts

[Exact source matrix](../toy-effects14-coverage.md#source630634-broker-disguise-shared-grafts-mixed-stall-and-updraft-boundaries-2026-10-18): Cartel Xy misc0 transform has no encoded creature; unaudited Void/Light marks share category2433/3000000ms recovery. Missing249056 outdoor stall retains effect164→1221382 removal edge; updraft retains105/104/106/506. Metadata only, no fabricated actor, ordinary trigger substitution, independent graft timer or cosmetic movement success.509 queued; no new acceptance/runtime/data/native proof or own heavy jobs.

## 2026-10-18 — Toy source625–629 boundaries and Cold Ones range correction

[Exact source matrix](../toy-effects14-coverage.md#source625629-goggles-distinct-carried-crates-wormhole-and-strategy-game-boundaries-2026-10-18): metadata preserves viewer goggles233/3569/247, distinct crate3393/3392 poses, missing248783 random-travel wormhole and permanent430 strategy game with separate two-second298624 trigger.507 failed234951 because a five-yard fixture point exceeds authored range169/max3. Test-only172cb1782 corrects that point, not runtime authority; format/manual readability/diffcheck pass. Lead509 queued covering all post507 acceptance; no result/landing/native proof or own heavy jobs.

## 2026-10-17 — Toy source620–624 pending POST/Cosmic hearthstones

[Exact source matrix](../toy-effects14-coverage.md#source620624-pending-postcosmic-hearthstones-and-balloonscaled-spirephase-guise-boundaries-2026-10-17):d981a866d adds245970/1240219 and actual slot1 246565/1242509 to unchanged10s/saved-bind/shared2309 helper, now21 hearthstones/old19 retained; format/manual readability/diffcheck pass, behavior AFTER507. Missing controlled slot6 Brewer balloon246042 and actual Phase Diver state remain gaps; scaled Spire131280/0.5 fixture retained. No inferred mail service, shortened cast, slot0 substitute, independent timer or phase immunity.507 prior19 scope pending final; no runtime/data/new native or own heavy duplicate.

## 2026-10-17 — Toy source615–619 pending Sea-Blessed Shrine acceptance

[Exact source matrix](../toy-effects14-coverage.md#source615619-pending-sea-blessed-shrine-and-kill-procscripted-memoryrolling-snowballroyal-visage-boundaries-2026-10-17):4a0133a68 adds245942/1240082/246037/display129958/cursor/180s as17th stationary actor; old16/helper retained, format/manual readability/diffcheck pass, behavior AFTER507. Explicit time-leap expiry not relabelled ordinary50ms/wall proof. Exact kill-proc42/trigger0 voice, script-only K'areshi appearance, missing controlled rolling snowball/holiday recovery and Royal Visage misc0 remain gaps.507 earlier16/18/12 scopes pending final; newer17stationary/19caster/13object unverified. No runtime/data/new native or own heavy duplicate.

## 2026-10-17 — Toy source610–614 pending Etheric Victory acceptance

[Exact source matrix](../toy-effects14-coverage.md#source610614-pending-etheric-victory-and-portalvoidsnowmanmixed-transmorpher-boundaries-2026-10-17):b7df42b22 adds244470/1237584/1800000ms as19th audited caster identity; old18/helper unchanged, format/manual readability/diffcheck pass, behavior AFTER507. Missing shared Dornogal portal244139, actual Void presentation, mixed2933212756 snowman with holiday recovery and Brann245374/65/77 service remain exact gaps. Preserve NPC2.2 metadata and description-only1237986 reference without player-scale/root-trigger invention.507 earlier scope still pending final; no runtime/data/new native or own heavy duplicate.

## 2026-10-17 — Toy source605–609 service/appearance/ride/paddle/pennant gaps

[Exact source matrix](../toy-effects14-coverage.md#source605609-courier-mailfinger-ribbonoutdoor-balloonskiffpennant-boundaries-2026-10-17): missing242136 Courier mail/category2066 and243246 outdoor balloon ride, finger-ribbon art, one-hour skiff104+250ms226/jump paddling and nonzero Astral pennant remain gaps. No inert actors, ring stats, flat130% buff or sibling presentation reuse.507 running without final aggregate/EXIT/FMT/landing;603 thirteenth object remains AFTER507. No new acceptance/runtime/data/native proof or own heavy duplicate.

## 2026-10-17 — Toy source600–604 pending Dastardly Banner acceptance

[Exact source matrix](../toy-effects14-coverage.md#source600604-pending-dastardly-banner-and-cratejugglingretained-scaled-banner-podium-boundaries-2026-10-17):3eabc6183 adds239007/1227030/527088/type5/display102616/size1/cursor/180s as13th transmitted object; old12/helper unchanged, format/manual readability/diffcheck pass, behavior AFTER507. Distinct carried crate3185 and actual juggling/count/remove203/reapply64/mixed293406 gaps retained. Existing scaled Flame banner and exact grounded static Podium20min CPU/historical native source-frame proof not relabelled as new native wall-duration/recharge certificate.507 queued, no runtime/data/new native or own heavy duplicate.

## 2026-10-17 — Toy source595–599 pose/fountain/paired-bobber gaps

[Exact source matrix](../toy-effects14-coverage.md#source595599-flarendofountain-and-paired-fishing-bobber-override-boundaries-2026-10-17): Flarendo's own5000ms pose, fountain36404 and Rocket520304/520305/Goblet520306/520311/Wellington520308/520309 paired258 overrides remain exact gaps. Runtime only520305 replacement exists, type17/display101984/size2;35591 generic bobber is not replacement art. Preserve both records/131474 references without immediate Fishing casts, guessed pair selection/cast conversion or flavor catch bonus.507 queued76a8426bf, no result/new acceptance/runtime/data/native proof or own heavy duplicate.

## 2026-10-17 — Toy source590–594 recipient/paddle/shop/homebind/pose gaps

[Exact source matrix](../toy-effects14-coverage.md#source590594-ally-returnpaddleoutdoor-stallmixed-homebindtorq-pose-boundaries-2026-10-17): true friendly-player sawblade return, waterwalk+250ms periodic raft/jump paddling, missing controlled outdoor stall237883, explosive homebind252+child1220867 and Torq5000ms pose remain exact gaps. No guaranteed return, flat130% speed, generic vendor, ignored child or invented image-file output.507 queued76a8426bf; no new acceptance/runtime/data/native proof or own heavy duplicate.

## 2026-10-17 — Toy source585–589 pennant/cartel banner gaps

[Exact source matrix](../toy-effects14-coverage.md#source585589-nonzero-prized-pennant-and-four-missing-cartel-banners-2026-10-17): Prized nonzero permanent pennant and distinct missing Steamwheedle508828/Bilgewater508826/Blackwater508827/VentureCo508830 cursor banners remain gaps. Item category1246/180s shared metadata retained, not independent timer proof or invented cartel qualification.507 queued76a8426bf including type correction/577; no log result, new acceptance/runtime/data/native proof or own heavy duplicate.

## 2026-10-17 — Toy source580–584 boundaries;505 compile correction

[Exact source matrix](../toy-effects14-coverage.md#source580584-mixed-shardpermanent-posesrepeated-jump-procinteractive-tea-boundaries-2026-10-17): no-template mixed transform/scale, permanent circlet/reading, two distinct jump-proc edges and missing interactive tea506612 remain gaps.505 compiled no runtime tests: E0277/EXIT101 from registryu32 creature versus aurai32 misc.76a8426bf changes only that column type, preserving16 rows/literals/helper/assertions; format/manual readability/diffcheck pass, next lead gate pending including577 twelfth object. No manufactured runtimeRED/GREEN, deduplication, copied Tea149865, runtime/data/native change or own heavy duplicate.

## 2026-10-17 — Toy source575–579 pending Cold Ones rack acceptance

[Exact source matrix](../toy-effects14-coverage.md#source575579-pending-cold-ones-rack-acceptance-and-mixed-chairdevicecratehomebind-boundaries-2026-10-17):0e4c5ed9b adds234951/1215364/505415/type22/display97591/scale0.75/cursor/180s as12th transmitted object; old11/helper retained, format/manual readability/diffcheck pass, behavior pending head AFTER50536909d8f3. ChairControl1+mixed aura, area36000 step-on goblination, carried crate3141 and homebind252+private-conversation-cancellation298 remain exact gaps, not generic source substitutes. Rack object spell1215373 is audited can aura, not trigger edge/inventory item.505 queued through Golem573, no result/landing inferred; no runtime/data/native proof/own heavy duplicate.

## 2026-10-17 — Toy source570–574 pending Golem acceptance

[Exact source matrix](../toy-effects14-coverage.md#source570574-pending-golem-self-transform-and-retained-bannersunnamed-seasoncarried-crate-boundaries-2026-10-17):06d44ef20 appends233202/473407/234479/six valid models/instant1800s/3600s as16th disguise plus Oomgut17IDs; old15/helper unchanged, format/manual readability/diffcheck pass, behavior pending lead covering head. Existing Prized/Astral source-banner cases retained.232308/233151 missing metadata/model and Hallowfall self293/3118/1 carried-crate appearance remain gaps; no invented season, possession actor or inventory supplies. No runtime/data/new native proof/own heavy duplicate;498 landed remains scoped older11 disguises.

## 2026-10-17 — Toy source565–569 seasonal banner boundaries

[Exact source matrix](../toy-effects14-coverage.md#source565569-existing-seasonal-banners-and-unnamed-season-four-metadata-gap-2026-10-17): four existing Tempered/Prized/Unbound/Forged actor cases retained, property64/front2yd/60s/source600s recovery.232304 exists in Toy with literal `[Season]` source description but no70448 ItemSparse/runtime content_item or233147 creature-model; no invented name/display or dropped catalog row. Shared fixture's faction-refusal controls are only war banners, not these seasonal identities. No new behavior/native/eligibility/recharge certificate, runtime/data or heavy gate;498 remains landed.

## 2026-10-17 — Toy source560–564 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source560564-retained-balloonsvictory-and-random-delverecipient-spotlightlocal-disguise-boundaries-2026-10-17): existing Balloon source actor and Explosive Victory caster cases retained without duplicate tests/count expansion. Delve-O-Bot mixed179/trigger/root with missing232267, random Bountiful selection/travel/completion reset, actual target57 Spotlight and Undermine8306/aura243/misc35 consequences remain gaps. Block metadata explicitly70448/key88d5e8ef05ec5a8446cdec93d142d7c6; no retroactive70391 proof.498 landed/master8707ac0ea; no new runtime/data/behavioral/native proof or own heavy duplicate.

## 2026-10-17 — Toy source555–559 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source555559-trialblingobserver-spidersqualified-return-homescreen-boundaries-2026-10-17): missing avoid-hit Trial229693, permanent Coldflame attachment, observer233/base550/7825/240 spiders, qualified Threads252/9 homebind and permanent screen2243/7 retain action/eligibility/presentation gaps. Threads item faction/reputation/PlayerCondition0 does not establish qualification authority or account/character/per-use scope; no guessed renown threshold or unrestricted Human-helper certificate. No global body/screen mutation, inert trial, new runtime/data/native proof or own heavy duplicate.498 landed/master8707ac0ea.

## 2026-10-17 — Toy source550–554 pending Arachnoserum acceptance

[Exact source/current scope](../toy-effects14-coverage.md#source550554-pending-arachnoserum-acceptance-and-pennantlinked-carrystone-metadata-boundaries-2026-10-17):c3fe0958c adds228705/462934/229772/sixteen valid models/instant10min/1h as15th disguise plus Oomgut16IDs; old14 data/cast phase unchanged.119763 NPC scale0.4 retained, not player-scale instruction. Format/manual readability/diffcheck pass, newer head required. Nonzero Strategist pennant, outdoors linked Lil Wick/Control4 carry and stone slow90/periodic action remain gaps;70391 root462924 lacks SpellMisc, no default cast/all-profile claim. Lampyridae six-attachment CPU witness retained.498 landed/master8707ac0ea; no further runtime/data/own heavy duplicate.

## 2026-10-17 — Toy source545–549 pending Shroud;498 landed

[Exact source/current scope](../toy-effects14-coverage.md#source545549-pending-shroud-acceptance-and-selected-petpaid-elixirguard-radiuspennant-boundaries-2026-10-17):7463a1090 adds226810/459531/228081/three valid models/source1.5s/20min/1h as14th disguise plus Oomgut15IDs, unchanged cast_ms phase/old13 data; format/manual readability/diffcheck pass, newer head required. Selected minipet90 leash, paid50silver Noggenfogger effect, actual City guard detection visualization and nonzero pennant retain action/recipient/lifecycle gaps; no generic aura/free outcome/guessed radius/stealth. Lead confirms498 LANDED/master8707ac0ea and local c013 ancestry exits0; new14disguise/11object/18caster scopes unverified. No further runtime/data/own heavy duplicate.

## 2026-10-17 — Toy498 scoped PASS;source540–544 pending disguises

[Current scope/source matrix](../toy-effects14-coverage.md#lead498-scoped-passnewer-expansions-still-pending-2026-10-17):498c013 PASS306/0failed/1ignored545.19s/client9/EXIT0/FMT0,5m37s/no warnings; named invisible-stalker ignored, landing not inferred. Corrected11-disguise/473,19 hearthstone,16 stationary/16 caster/10 object scopes pass. Newf44658cf6 adds Kobyss41-model/source1.5s/30min and Pileus29-model/instant10min, both1h recovery, now13disguise+Oomgut14IDs; old11 literal values/order/cast0 retained in count-free slice, actual pending-cast phase added. Format/manual readability/diffcheck pass; newer head required, as for new18caster/11object scopes. Reading/nonzero pennant/enemy-player frontal corpse flag remain exact gaps. No runtime/data/native/full-pool claim or own heavy duplicate.

## 2026-10-17 — Toy source535–539 pending Toxic Victory acceptance

[Exact source matrix](../toy-effects14-coverage.md#source535539-pending-toxic-victory-acceptance-and-owned-pet-cookingfinerydyeingrole-dummy-boundaries-2026-10-17):896be829e adds225547/443689/source1800s as18th existing caster identity, format/manual readability/diffcheck pass, future head AFTER498c013 required. Own non-combat pet150/candle/395 cooking, missing Finery Chest452682, periodic dyeing wheel and missing outdoors60s Icarus damage-and-heal/opposing-role dummy retain exact recipient/action/lifecycle gaps. No generic light/inert chest/five-item yield/damage-only dummy or flavor-derived qualification. No further runtime/data/new native/own heavy duplicate;498 corrected Claw/Stasis/Stone individual ok lines, aggregate/landing pending.

## 2026-10-17 — Toy source530–534 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source530534-layered-spinself-memorytorchmixed-levitationmoving-hologram-boundaries-2026-10-17): Ravager slow33/nested5s10s30s periodics/7s stun/parent removal, personalized Memory Chord with literal model weights, unaudited permanent torch, hover106/featherfall105/waterwalk104/animation1151 and missing forward horse225414 retain mixed action/appearance/lifecycle gaps. No cosmetic-only control, removal-as-recast, generic self copy, cooking/fire-flight guess or stationary horse admission. No further runtime/data/new proof/own heavy duplicate;498c013 in progress.

## 2026-10-17 — Toy source525–529 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source525529-permanent-podrandom-wormholeseedrecipient-gift-boundaries-and-retained-camera-2026-10-17): permanent224771 Pod/three models, RequiredSkill2875/rank1 random Khaz Algar wormhole/missing223342, mixed seed/33467 area and friendly missile1.5 Hat→60s aura plus recipient223316 item remain gaps. Same-name physical223316 is distinct from toy223312, DurationInInventory60/stack1/MaxCount1; no caster-only creation/count2 assumption. Existing Filmless Camera actual attachment/cancel CPU scope retained without native photography proof. No further runtime/data/new proof/own heavy duplicate;498c013 in progress.

## 2026-10-17 — Toy source520–524 pending Cherry Trail acceptance

[Exact source matrix](../toy-effects14-coverage.md#source520524-pending-cherry-trail-acceptance-and-puntablesopt-in-fireworkscookingwater-blaster-boundaries-2026-10-17):4b21ddb7e adds220777/447446/source600s to unchanged caster helper, now17 identities; format/manual readability/diffcheck pass, newer head AFTER498c013 required. Missing puntable-box creature237586 cannot use same-number Bonfire GO; fireworks preserve attack-opt-in viewer audience/event recovery, missing Lantern452051 preserves actual cooking, wild type22 water display retains unimplemented445930 refusal. No inert substitutes, new native/runtime/data proof or own heavy duplicate.

## 2026-10-17 — Toy source515–519 pending colorful-chair acceptance

[Exact source matrix](../toy-effects14-coverage.md#source515519-pending-colorful-chair-acceptance-and-wingtwo-person-cracker-boundaries-2026-10-17):66ca57ee0 adds218112/441839/type7/434317/89277/size1.5/source2yd/cast3s/300s to transmitted-object helper, now11 identities, old ten unchanged. Format/manual readability/diffcheck pass, newer head AFTER498c013 required; seating/native/recharge remain separate. Distinct eight-hour/20s-period Yu'lon/Niuzao wings and paired Cracker/one-recipient surprise retain exact action/cadence/eligibility gaps. Draconic Banner existing seasonal CPU scope retained. No further runtime/data/own heavy duplicate.

## 2026-10-17 — Toy496 ordering corrections;source510–514;498 queued

[Current proof/source matrix](../toy-effects14-coverage.md#gate496-fixture-order-failures-corrected498-pending-2026-10-17):496 FAILED303/2/1ignored536.82s/client9/EXIT101/FMT0,2m16s/no warnings.450 saw active GCD,473 saw combat after restored Frostbolt; lead-approvedc013eafdd test-only natural1500ms wait and pre-Frostbolt recovery check preserve all assertions without resets/clears.498c013 queued including503/missile filters.16 stationary/16 caster/10 object/18 hearthstone scoped successes are not aggregate/landing proof. Source510–514 retains critter128/beam/figurine, missing interactive duck egg, unnamed216887 faction chest and distinct occasional Xuen/Chi-Ji eight-hour/20s-period/category2272 wings. No new runtime/data/native/own heavy duplicate.

## 2026-10-17 — Toy source505–509 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source505509-mount-randomizationquest-creditcolored-parasolfallen-player-burial-boundaries-2026-10-17): per-mount customizable-skyriding appearance randomization plus actual quest80201 completion, Jade/Crimson/Ebony conditional105/409/312/431982 payloads and fallen-player burial remain gaps. Colored remove164 fields all target white431949, not their own roots; literal mismatch retained without repair/recursive casting. Gravestone217846/117236 does not certify corpse eligibility or permit living/self transform substitute. No further runtime/data/new proof/own heavy duplicate;496 marks expanded disguise and Stasis failed, final diagnostics pending.

## 2026-10-17 — Toy source500–504 pending Stone acceptance

[Exact source matrix](../toy-effects14-coverage.md#source500504-pending-stone-of-hearth-acceptance-and-pennanttableconditional-parasol-boundaries-2026-10-17):d68256325 adds212337/401802/source10s/saved-bind/shared2309/one1800000ms charge and Flame refusal, now19 hearthstones; unchanged helper/old18. Format/manual readability/diffcheck pass, newer head AFTER4964df7af2e6 required. Permanent nonzero pennant, missing interactive Hearthstone table and conditional Parasol226→featherfall105/can-turn409/animation312/parent-removal164/child aura remain gaps, not furniture/always-on featherfall/recursive recast. Retained Abyss Horn visual-only CPU witness does not create gameplay water. No further runtime/data/new native/own heavy duplicate.

## 2026-10-17 — Toy source495–499 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source495499-observerparty-private-perspectivefixed-gilneaswater-boat-boundaries-and-retained-banner-2026-10-17): Eyes periodic child preserves caster-and-summons target120 and both perception pairs; Date Modulator is outdoors party/raid-private machine plus effects, not global/owner-only perspective. Tess's252/17+77 goes to fixed Gilneas, not current inn bind. Four-seat permanent Control4 Love Boat requires water/radius20yd/holiday recovery; base430489 stays literal, not count/automatic child. Exact Dreaming Banner seasonal CPU row retained, no fresh native/recharge certificate. No further runtime/data/new proof/own heavy duplicate;4964df7af2e6 running.

## 2026-10-17 — Toy source490–494 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source490494-monumentraid-aurapennantmoving-trailowl-restsleep-boundaries-2026-10-17): missing Murkmorpher212150/interaction plus actual raid-area65/425841, permanent nonzero Verdant pennant, Socks500ms periodic child→1s persistent-area27 trail, Owl Post resting and missing213113 Leafbed usable sleep remain gaps.425843 description reference is not an executable root edge; no automatic model change, caster-only/deduplicated trail or static bed admission. No further runtime/data/new acceptance/native proof/own heavy duplicate;4964df7af2e6 lead-owned pending.

## 2026-10-17 — Toy source485–489 boundaries;496 queued

[Exact source matrix](../toy-effects14-coverage.md#source485489-festive-friendgrowthracial-homebind-boundaries-and-retained-interactive-banner-2026-10-17): linked Festive Pepe model114425/scale0.2/probability0, complete three-friend and fast-growth contracts remain unaudited. Chilling Banner is actual type22 object with424023/3600s caster aura use witness, not inert type5/native/access/expiry certification. Draenic Hologem's explicit draenei/lightforged restriction stays a gap despite source race halves-1/PlayerCondition0 and normal shared2309 homebind encoding; no invented mask or Human-helper certificate. No further runtime/data/own heavy duplicate.4964df7af2e6 queued with toys/toy/mail/summon/missile/fmt; local16 proof stays8687f84c5.

## 2026-10-17 — Approved cursor-flight fix; all16 local GREEN

[Current scoped proof](../toy-effects14-coverage.md#approved-cursor-flight-fix-complete16-row-local-green-2026-10-17): lead approved only87 in destination whitelist, removing honest ignore and local complete fixture.8687f84c5 does exactly that, preserving all16 identities/assertions; PASS1/0failed/0ignored47.72s,3m43s compile/no warnings. All seven minis now satisfy real pre/post-flight actor ordering, with old nine scenery rows retained. Format/fmtcheck/manual readability/diffcheck/exact pushed readback pass. Other new tests only compiled/filtered, not behavior-certified. Historical492 RED retained; successor aggregate/landing lead-owned/pending. No deployment/data/producer/engine/protocol/native/full-height/wall-lifetime/recovery claim or further own heavy duplicate.

## 2026-10-17 — Toy source480–484 pending Flame acceptance

[Exact source matrix](../toy-effects14-coverage.md#source480484-pending-flame-return-home-acceptance-and-unnamed-hatrideprivate-weather-boundaries-2026-10-17):2302a9e1b adds209035/422284/source10s/saved-bind/shared2309/one1800000ms charge and Deepdweller refusal, now18 hearthstones; format/manual readability/diffcheck pass, newer covering proof pending. ItemSparse208880 stays unnamed; periodic drake hat/Brewfest, Control5 gryphon ride and Control0 rolling-barrel ride retain exact gaps, no static substitutes. Dreamsurge331 is target-private weather488/density1 plus screen2078/7 and zone-weather restore, not phase/global weather/screen-only behavior. No production/data/native/own heavy duplicate; cursor87 runtime proposal awaiting approval.

## 2026-10-17 — Toy492 reproduced cursor-flight RED; approval proposal

[Exact current matrix](../toy-effects14-coverage.md#gate492-cursor-flight-red-runtime-proposal-awaiting-approval-2026-10-17):4920c77 FAILED286/1/1ignored544.23s/client9/EXIT101/FMT0,2m14s/no warnings. Maiev201931 actor before source flight actual1/expected0 is a real runtime failure. Lead-approvedde2724abe preserves all assertions/16 identities under explicit RED ignore; later mini rows not reached and no fixed/aggregate/landing claim. Corrected387,393,402,419 individually ok. Add87-only destination-whitelist proposal is supported by70448 literal effect32/87 roots/speeds and pinnedTCa352b1fa exact destination distance/speed/HasDst; approval pending, production unchanged. New445/450/454/457/473/477 tests remain unverified after492. No own heavy duplicate/runtime/data/native changes.

## 2026-10-17 — Toy source475–479 pending Deepdweller acceptance

[Exact source matrix](../toy-effects14-coverage.md#source475479-pending-deepdweller-return-home-acceptance-and-mounted-pennantmirrormemoryring-boundaries-2026-10-17):eea1ff744 adds208704/420418/source10s/saved-bind/shared2309/one1800000ms charge and Path-of-Naaru refusal, now17 hearthstones; unchanged helper/old16. Format/manual readability/diffcheck pass, newer head beyond4920c77 required. Mixed mounted/400ms periodic/keybound pennant with group8051, authentic zero-entry man'ari Mirror, group8008-qualified memory-disc service and party rings/spacing/multiplicity/five-minute children/holiday recovery remain gaps. No runtime/data/native/model guessing or own heavy duplicate.

## 2026-10-17 — Toy source470–474 pending Stasis acceptance

[Exact source matrix](../toy-effects14-coverage.md#source470474-pending-stasis-acceptance-and-torchritualfollowerrandom-wolf-boundaries-2026-10-17):52e9f7c80 adds actual learned Stasis208415/419271/stun12/source60s/300s recovery, real Frostbolt Stunned refusal and post-expiry restored cast, replicated removal/full-health/model/retained cooldown. Two intentional replies are asserted, not cleared. Actual Retail authority is content_item/item_sparse/effect joins. Torch includes kill-credit229063 plus actual ancient-device actions; incomplete ritual, Boffins real follow actor and random-volume wolf remain gaps. Format/manual readability/diffcheck pass; newer head beyond4920c77 required. No production/data/native/time-freeze/movement-input proof or own heavy duplicate.

## 2026-10-17 — Toy source465–469 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source465469-plattertiny-rocksvehicle-detectionhourglass-action-boundaries-and-retained-banner-2026-10-17): mixed platter/script/service and tiny-rock base4/special flags retain missing-body/action gaps. Idol296 is actual vehicle8196 with500ms periodic child and four ordered repeated script recipients, not generic bird tracking. Minute Glass override2600 grants six actual60/120/300/600/1800/3600s mixed self-aura/area29579 actions; no root-only timer shortcut. Exact Smoldering Banner existing seasonal CPU witness retained, no new native/full-recovery claim. No runtime/data/new proof/own heavy duplicate;4920c77 running.

## 2026-10-17 — Toy source460–464 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source460464-pennantmannequingrainrelative-coordinatepocketwatch-boundaries-2026-10-17): permanent nonzero Crimson pennant, mannequin script/model selection, missing403749 grain crate, permanent Thinking Cap periodic226/1500ms plus relative-coordinate/re-equip/thought contract and nonzero hour Pocketwatch remain gaps. Literal w2/w1-minus50000 display retained; no guessed map axes, decorative-only cap, grain infection or server-clock shift. No runtime/data/new proof or own heavy duplicate; saved4920c77 log is compiling without behavioral result.

## 2026-10-17 — Toy source455–459 pending Naaru acceptance

[Exact source matrix](../toy-effects14-coverage.md#source455459-pending-naaru-return-home-acceptance-and-explosive-showcritterpennantservice-boundaries-2026-10-17):1ce4b65a0 adds206195/412555/source10s saved-bind/shared2309 with one1800000ms charge and Ohn'ir sibling refusal to unchanged helper, now16 hearthstones. Format/manual readability/diffcheck pass; newer covering head beyond4920c77 required. Three-area explosive show plus child411846, real Fyrakk critter action, permanent nonzero seasonal pennant and missing205879 Warpweaver/transmog service remain exact gaps; no multiplicity collapse, cosmetic critter shortcut or inert service admission. No runtime/data/native changes or own heavy duplicate.

## 2026-10-17 — Toy source450–454 pending Claw/Nightmare acceptance

[Exact source matrix](../toy-effects14-coverage.md#source450454-pending-clawnightmare-banner-acceptance-and-wisdomaudioattraction-boundaries-2026-10-17):1c5afda02 adds205904/410931/205296/four-display600s appearance/category2177 one-hour recovery to disguise fixture, now11 rows plus Oomgut12IDs, and206008/411653/402852/type5/82612/size1/180s as tenth transmitted cursor object. Original10/nine identity rows unchanged; format/manual readability/diffcheck pass, newer covering gate beyond4920c77 required. Senegos yes/no action/missing body, week listener-only sniff sound suppression and friend-compulsion area28942 remain gaps; no invented answer distribution, global mute, arbitrary teleport or generic aura admission. No runtime/data/producer/native/pool/full-recharge proof or own heavy duplicate.

## 2026-10-17 — Toy source445–449 pending Cinder acceptance

[Exact source matrix](../toy-effects14-coverage.md#source445449-pending-cinder-acceptance-and-drumritualglitterlava-pack-boundaries-2026-10-17):184f9874d adds205418/409891/source600s to caster fixture, now16 exact identities; old15 and assertions unchanged. Format/manual readability/diffcheck pass, newer behavioral head beyond4920c77 required. Mixed45s drum/special flags and10s ritual/Control1 faction35 retain missing actors and actual action gaps; friendly glitter395/create-properties28856 is not a visual-only dummy. Lava Pack override2525 grants410609→410641 via actionbar metadata, with missing PowerDisplay127 and actual toss/restore gaps. No runtime/data/producer/native admission, guessed power/damage or own heavy duplicate.

## 2026-10-17 — Toy source440–444 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source440444-mixed-prisonhornloamm-travel-boundaries-and-retained-friend-effects-2026-10-17): Titan prison full mixed control and unaudited Obsidian horn remain gaps; Mortar's existing selected-friendly cast/delivery and Fist's recipient/compressed-day aura proof retained without hostile/facing/native/recharge extrapolation. Mitts409147 preserves10s252/1/17 plus script77, source RequiredAreas7894→Loamm14520/Zaralek14022/map2454, not Area7894; actual qualified tunnel destination/transfer/persistence unverified. No invented coordinates, saved-bind substitute or description-only Loamm exclusion. No runtime/data/new acceptance/own heavy duplicate;492 remains pending.

## 2026-10-17 — Toy source435–439 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source435439-linked-hologramstone-smashholiday-basket-boundaries-and-retained-bear-2026-10-17): Lady of Dreams360s and Timeless One300s linked holograms retain distinct stands/mixed child auras and literal source text, not stand-only or model-name substitution. Stone Breaker's two root children,191/dummy/script and both28357 area edges remain an unresolved smashing contract; misc1300/700 not guessed counts. Bear204405 exact CPU/earlier both-skin model evidence retained without new pixels/recovery; Basket204675 caster aura/cancel witness does not certify mounted Dragonriding presentation or Noblegarden cooldown. No runtime/data/new admission/proof or own heavy duplicate.4920c77 queued.

## 2026-10-17 — Toy source430–434 boundaries;492 queued

[Exact source matrix](../toy-effects14-coverage.md#source430434-cookingforced-formclanlinked-hologram-boundaries-and-retained-will-2026-10-17): missing runtime386682 cooking brazier, ForceCast2→402636→202340/88121 transform, nonzero chosen-clan banner and linked428/403712 mixed summon+self-aura hologram retain exact action/selection/lifecycle gaps. Hraxian204220/403491 is an existing15-caster484 accepted row, not new art/expiry/reuse proof. Root/child hologram text discrepancy preserved without model substitution or guessed scale. No runtime/data/producer/new acceptance/native changes or own heavy duplicate. Lead492 queued0c77bfdaa for corrected same-clock regen and actual actor-phase tests;489 remains failed.

## 2026-10-17 — Toy489 observable assertion corrections

[Current proof matrix](../toy-effects14-coverage.md#gate489-assertion-corrections-behavioral-proof-pending-2026-10-17):0c77bfdaa corrects387 health equality against untargeted same-state/same-clock ordinary regen, requiring positive regen without timer/health suppression. Mini destination-child casts do not emit an independent SpellGo;16-row fixture now observes zero actors before flight and exactly one afterward. `missiles::DEST_TARGETS` excluding87 suggests an actual source-flight gap; not behaviorally reproduced yet and no runtime fix/borrowed GREEN. Saved489c043 FAILED284/2/1ignored577.08s/EXIT101/FMT0;393/402 individual ok, no landing/native certificate. Format/manual readability/diffcheck pass; newer lead-owned behavioral gate pending, no own heavy duplicate. Earlier queued/live statements are historical.

## 2026-10-17 — Toy source425–429;489 failed assertions

[Exact source matrix](../toy-effects14-coverage.md#source425429-pending-murlocegg-acceptance-and-retained-bannercabersnow-boundaries-2026-10-17):10e8faaad adds Murloc202862/399265→399253→200972/112325 and Whelp Egg202865/399268→399257→200975/112328 with own speed30/18yd roots;16 stationary-source identities, original14 unchanged. Local format/manual readability/diffcheck pass; expanded fixture/newer covering head pending. Thundering Banner exact seasonal row retained; mixed caber/area/script/nonzero dummy and snow transform/191/ground area/370 payloads remain gaps. Inspected489c043 FAILED284passed/2failed/1ignored577.08s/EXIT101/FMT0:387 health52 versus37 is ordinary regen;408 child-event assertion expects a SpellGo not emitted by destination child path. Actor-phase/source-flight diagnosis and test corrections open;393/402 individual ok do not make gate pass. No own heavy duplicate/runtime/data/native admission.

## 2026-10-17 — Toy source420–424 pending three-mini acceptance

[Exact source matrix](../toy-effects14-coverage.md#source420424-pending-three-mini-acceptance-and-fishingcompass-boundaries-2026-10-17):53f4ae444 adds Huntress202851/399232→398245→200961/112316, Tauren202856/399247→399246→200966/112319 and Ghoul202859/399262→399250→200969/112322. Each own speed30/valid18yd cursor root and actor/state/owner/map/stationary/compressed300s expiry assertions; original11 rows preserved, now14. Source child399250 Speed1 retained, no invented second missile. Format/manual readability/diffcheck pass; later lead-owned covering head required. Fishing39/permanent mixed channel/garbage placement and unaudited hold-out compass contracts remain gaps. Live489 marks387Wand FAILED,393Ohnir/402Soulshape ok; final details/result pending. No own heavy duplicate, runtime/data/producer/native/tier/full-height/wall-time/minute-reuse certificate.

## 2026-10-17 — Toy source415–419 pending Sneed/product acceptance

[Exact source matrix](../toy-effects14-coverage.md#source415419-pending-sneedproduct-acceptance-and-bobberfurbolgscreen-effect-boundaries-2026-10-17):d3bed536f adds Sneed speed30/valid18yd cursor→398246→200350/114420 as eleventh stationary-source row, preserving Maiev12yd via consolidated root/distance data.50f7b11fc adds actual learned202309/398572/source3s CreateItem24→one persisted202310,300000ms recovery/exact refusal; existing279 timing/quantities/187342 retained with generic item counter. Format/manual readability/diffcheck pass;11-row/two-create-test proof requires a head after489c043. Bobber258 compatibility, nonzero Furbolg dummy, ScreenEffect260/2088/7 and product202310 stack1/MaxCount1 repeat/use enforcement remain boundaries; no wrong count2, runtime/data/native/producer admission or own heavy duplicate.

## 2026-10-17 — Toy source410–414;484 landed /489 queued

[Exact source matrix](../toy-effects14-coverage.md#source410414-riches-trailally-stormenemy-corpse-flagkiteshades-boundaries-2026-10-17): mixed periodic riches/set1072/minimum-speed437, friendly chasing storm, source-centered enemy-player corpse/front flag and unaudited kite/shades retain exact action/recipient/lifecycle gaps. No guessed cadence/currency/corpse-template or zero-dummy admission. [Inspected484 receipt](../toy-effects14-coverage.md#lead-integration-receipt-2026-10-17) supersedes older pending484 entries:283passed/0failed/1ignored408.90s, client9passed, EXIT0/FMT0,4m12s compile/no warnings;15caster/nine-object explicitok, lead confirms LANDED/masterec0863223. Ignored test is named invisible stalker. New387Wand/393Ohnir/402Soulshape/408Maiev queued489 throughc043f0f8c, result pending. No runtime/data/new native/full-toy certificate or own heavy duplicate.

## 2026-10-17 — Toy source405–409 pending Maiev acceptance

[Exact source matrix](../toy-effects14-coverage.md#source405409-pending-maiev-missile-child-acceptance-and-cloakcrystalbg-flarechallenge-dummy-boundaries-2026-10-17):58a5fabd1 adds actual Maiev missile32/speed12/valid12yd cursor→395720→199047/112284/scale1, parent-before-child events and actor/horizontal destination/owner/map/non-pet/non-following/stationary/compressed-expiry assertions. Old nine identity rows unchanged; two old cursor selectors independently verified87/0 before strengthening horizontal checks. Ten-case future-head proof pending, local format/manual readability/diffcheck pass; no native/tier/full-height/wall-lifetime claim. Targeted cloak, unaudited crystal, BG/faction/area180s flare lock and outdoor opposing-role damage/heal dummy remain gaps. No runtime/data/producer admission or own heavy duplicate.

## 2026-10-17 — Toy source400–404 pending Soulshape recovery acceptance

[Exact source matrix](../toy-effects14-coverage.md#source400404-pending-soulshape-recoveryrefresh-acceptance-and-controlmusicgangholiday-boundaries-2026-10-17):1f028916a adds actual200960/392700/100634/source600000ms appearance/300000ms recovery test, with immediate/near-five-minute refusals, natural reuse while active, refresh, ordinary expiry/native-display/health restoration and exactly two recovery/two intended failure replies. No manual reset; sourceAURA_UNIQUE clear matches existing refresh rule. Format/manual readability/diffcheck pass; not covered by484afad, future head required, no selected-personal-Soulshape/native certificate. Control3 boat/player restriction, mixed music/no-actions/mount state, linked gang follow/attack and Darkspear Dash dance recovery remain gaps; no runtime/data/new admission or own heavy duplicate.

## 2026-10-17 — Toy source395–399 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source395399-invocationeggscloak-servicerandom-talismanalcohol-boundaries-2026-10-17): invocation/script base1200, three eggs/named Rowdy-Onion-Blitz versus script base4, cloak service, literal392200/392209 random-Talisman references and alcohol100/base5 retain distinct action/state/cleanup gaps. Existing25204/109750 and source child transforms do not prove hatchlings, cloak access, executable base fields or visual-only alcohol; invisible11686 weight0 retained. No producer/runtime/data/new positive/native claim or own heavy duplicate.484's older head does not cover new387/393 fixtures.

## 2026-10-17 — Toy source390–394 pending Ohn'ir acceptance

[Exact source matrix](../toy-effects14-coverage.md#source390394-pending-ohnir-return-home-acceptance-and-peltcatchorbeight-trigger-boundaries-2026-10-17):00ee4d3e3 adds learned200630/391042/source10s252/savedbind/shared2309one1800000mscharge, now15hearthstone variants. Format/manual readability/diffcheck pass; future covering head beyond484afad required, not yet passed. Pile/orb runtime identities absent, bouquet missile/cursor-centered friendly1yd catch unresolved, and Palooza retains all eight effect-index trigger/misc pairs and distinct area-triggers26829–26836, including both misc300 children. No inferred must-be-outside-Assembly gate, guessed timing, one-actor substitute, runtime/data/new native or full-recharge claim.

## 2026-10-17 — Toy source385–389 pending mixed Wand acceptance

[Exact source matrix](../toy-effects14-coverage.md#source385389-pending-mixed-chewed-wand-acceptance-and-zombiecagerodpelt-boundaries-2026-10-17):27772d6ad adds actual learned200249/389324 source1700ms self56+confusion5/10000ms test with damaged37 health, cast blocking, ordinary expiry/native-display and casting restoration/replicated-aura removal/1800000ms recovery refusal. Format/manual readability/diffcheck pass; not covered by484afad, future behavioral proof pending. Runtime16372 weights1:1/build12340 versus pinned2:1/build69497 preserved as named profile gap, no producer fix/native probability claim. Zombie406 keybound/speed, Control4 cage, selected-player Dust Pile/misc9 and absent-runtime pelt remain gaps; no simple-disguise/target substitution or runtime/data/new native admission.

## 2026-10-17 — Toy source380–384 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source380384-compasslavaunnamed-moneyspeechleap-boundaries-2026-10-17): north-pointing compass, outdoors Dragon-Isles lava immunity, unnamed200142/20g and once-plus-three missile children, mixed base7 speech and caster-back leap/area-trigger odor retain named gaps.200142 lacks ItemSparse despite Toy/Item/on-use rows;381637/381644 absent runtimeTDB, no guessed name, payment bypass or child deduplication. No generic cosmetic/runtime/data/new positive/native certificate or own heavy duplicate.484 throughafad56176 still owns pending15-caster/nine-object proof, unchanged code in this block.

## 2026-10-17 — Toy source375–379;478 landed /484 queued

[Exact source matrix](../toy-effects14-coverage.md#source375379-pending-survey-tools-acceptance-and-follydummyseatsled-boundaries-2026-10-17):afad56176 adds Survey206590/type5/10103/scale1/cursor87/300s as ninth transmitted-object case, preserving old eight. Format/manual readability/diffcheck pass; lead484 explicitly queued throughafad56176 for15caster/nineobjects, not passed. Folly group-private actor proof retained separately from friends' challenge; outdoor fish dummy,78/radius1yd beanbag seat and mixed linked/passenger Tug Sled remain gaps. [Inspected478 receipt](../toy-effects14-coverage.md#lead-integration-receipt-2026-10-17) supersedes older pending478 entries:283passed/0failed/1ignored672.55s, client9passed, EXIT0/FMT0,1m24s compile/no warnings; three tents/all14hearthstones/ten-disguise+Oomgut explicitok, lead confirms LANDED/mastera2ddee2cb. Ignored test is named invisible stalker. No native/full-recovery/runtime/data/admission claim or own heavy duplicate.

## 2026-10-17 — Toy source370–374 pending banner acceptance

[Exact source matrix](../toy-effects14-coverage.md#source370374-pending-bluebronzegreen-acceptance-training-gap-and-retained-soup-proof-2026-10-17):73494c432 adds distinct Blue381321/77021, Bronze381319/76419 and Green381320/77020 actual cursor87/type5/0.5/180s cases, preserving old five rows. Eight-object plus15-caster successor coverage after478 requested, not passed; format/manual readability/diffcheck pass. Training196394/outdoors/combat gap retained without inert import. Soup381360/type8/focus4/31442/7353 retains exact seven-item CPU buff1/60s/trigger/audience/cooking proof, not representative Little Wickerman pixels as Soup-native certification. No runtime/data/new admission or own heavy duplicate.

## 2026-10-17 — Toy source365–369 pending Black Dragon acceptance

[Exact source matrix](../toy-effects14-coverage.md#source365369-pending-black-dragon-acceptance-and-seatteared-banner-boundaries-2026-10-17):b53bd5e5b adds distinct existing Black381318/type5/71263/0.5/cursor87/180s as fifth transmitted-object case; old four unchanged. Format/manual readability/diffcheck pass, expanded five-object+15-caster successor coverage after478 requested, not passed. Existing learned Red381317 missing-template/no-cast refusal retained, not positive/banner-copy proof. Caster-ground S.E.A.T. seating and distinct Dragon/Whale Bone type10/Data10=394316/394315 seated-drinking85/226 action/service gaps preserved. No invented mana/seat/refill/loot, inert props, runtime/data/new native proof or149865 producer change.

## 2026-10-17 — Toy source360–364 and pending335/361 acceptance

[Exact source matrix](../toy-effects14-coverage.md#source360364-pending-reliquaryduck-acceptance-and-bannersledvacuum-gaps-2026-10-17):361431949735 adds existing Reliquary380639/type5/76510/0.6/cursor87/180s as fourth actual transmitted-object case, preserving old three rows and asserting concrete owner/map/facing/expiry. Existing cosmetic.rs already audits335383065;49e9c2d95 adds exact194056/permanent0/cancel/unchanged-health-model/no-actor as15th caster row. Format/manual readability/diffcheck pass; lead accepts335/current tip into gate after478, expanded four-object coverage requested, not passed. Missing380638 sibling, controlled sled, distinct Lucky Duck385941 and rectangle-target130 vacuum action remain gaps; no runtime/data/new admission or own heavy duplicate.

## 2026-10-17 — Toy source355–359 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source355359-taivan-buffpillow-boundaries-and-retained-statue-proof-2026-10-17): Taivan195387/nearby scaled384543 remains an action/stat/recipient gap; description reference is not a root trigger or flat percentage. Ornate statue194793/108683/0.7/CDI0.3 retains earlier CPU/both-skin proof without a fresh native certificate. Three pillow roots keep target78/radius1yd/3s/5min:Purple376979/type7/74543/size3 exists in runtimeTDB,380634/380635 do not. Placement initialization/seat/identity gaps stay explicit; no zero-foot guess, copied geometry, inert import, runtime/data/new positive claim or own heavy duplicate.

## 2026-10-17 — Toy source350–354 and329 runtime-table correction

[Exact source matrix](../toy-effects14-coverage.md#source350354-private-weathershellrafteasel-boundaries-and-retained-cooking-proof-2026-10-17): private outdoor weather, mixed shell/action/animation, water/jump-paddle raft and easel movement/pose remain gaps; Maruuk retains exact source16 cooking CPU scope without per-toy native pixels. **Correction to earlier325–329 entry:** summoned-object loader reads runtime `tdb_gameobject_template`, which contains offering377204/type3/display74995 and cooking379147/type8/display75339; earlier offering absence inference from base/content tables was wrong and is withdrawn. Offering interaction gap unchanged. No runtime/data/new admission or own heavy duplicate.

## 2026-10-17 — Toy source345–349 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source345349-slidewyrmholerecalibrationprivate-environmentperception-gaps-2026-10-17): mixed slide/periodic child, engineering2827/rank1 Wyrmhole random/triangulated travel, recalibrator area-trigger27706, outdoors party/raid-private environment Title55/rawsignedflags and observer-scoped gnoll perception remain distinct gaps.195667/194993 runtime content absent; existing198156 census stays blocked. No invented tick/destination/model pools, public/owner-only privacy substitute, inert imports or new positive/native proof. Lead478 owns expanded acceptance verification; no heavy duplicate.

## 2026-10-17 — Toy source340–344 and inspected477 proof

[Exact source matrix](../toy-effects14-coverage.md#source340344-mailboxwhelpsholidayfeast-boundaries-and-retained-sign-proof-2026-10-17): Ohuna Control1/Title5 mail/recall, mixed five-whelp movement, Murglasses holiday recovery and four-model appreciative-critter feast remain gaps;341 retains exact historical sign CPU proof, not native certification. No runtime/data/inert imports or own heavy duplicate. [Gate477 receipt](../toy-effects14-coverage.md#lead-integration-receipt-2026-10-17) now independently inspected: server280passed/0failed/1ignored445.11s, client9passed, EXIT0/FMT0,2m58s compile/no warning lines; caster14/all13hearthstones/nine-disguise+Oomgut explicit ok. Ignored case is named invisible-stalker test.478 expanded cases remain queued, not passed.

## 2026-10-17 — Toy lead477 landed /478 queued

[Current receipt](../toy-effects14-coverage.md#lead-integration-receipt-2026-10-17) supersedes earlier pending477 entries below: lead reports477 LANDED/“280 passed”/master15bef300d throughd4a4a716c (14caster/13hearthstone/nine-disguise+Oomgut10-ID revision). Per-fixture details requested, not independently inspected here. Expanded three-tent/14-hearthstone/ten-disguise+Oomgut11-ID head28f1ce365 is explicitly queued in478, not yet passed; no own heavy duplicate or new native/full-recovery claim.

## 2026-10-17 — Toy source335–339 pending linked-tent acceptance

[Exact source matrix](../toy-effects14-coverage.md#source335339-pending-linked-tent-acceptance-and-napseat-boundaries-2026-10-17):28f1ce365 extracts existing Gnoll fixture into four identity parameters and adds Market/Expedition source3s/15minlinked/permanent-child actual-use cases. Real SpellStore replaces the old selective setup only inside this helper to include new roots/children. Three source placement/model/rotation/refresh/owner/cancel/death tests await new-head proof; format/manual readability/diffcheck pass. Duck Lovie nap/pose and both cushion presentation/interaction remain gaps; runtime cushions absent despite TDB109046/109053. No runtime/data/native/natural-expiry/commerce certificate or own heavy duplicate.

## 2026-10-17 — Toy source330–334 pending Timewalker acceptance

[Exact source matrix](../toy-effects14-coverage.md#source330334-pending-timewalker-acceptance-and-prismtentseatpall-boundaries-2026-10-17):4ee545b9f adds actual learned193588/375357/source10s/savedbind/shared2309one1800000mscharge, now14hearthstone variants. Format/manual readability/diffcheck pass, explicitly covering new-head behavioral proof requested;477 throughd4 only has13. Existing prism activation/block/cancel and tent source-owner/refresh/cancel/death witnesses retained without new native/natural-expiry certificate. Beanbag seating remains blocked; permanent target25 Pall108530 recipient/corpse lifecycle remains a gap. No runtime/data/new admission or own heavy duplicate.

## 2026-10-17 — Toy source325–329 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source325329-audiolaunchwisdomanimationoffering-boundaries-2026-10-17): surrounding audio509, mixed98/base1200 launch, wisdom189951/properties5348/Title27,312 animation set61 and ground377204/type3 offering retain named action/presentation gaps. Runtime wisdom/GO templates absent despite TDB107044/74995; no inert import, invented stealth/physics/loot/advice or new positive/native proof. Exact cast/duration/item-versus-inherited recovery preserved; lead-owned pending acceptance gates not duplicated.

## 2026-10-17 — Toy source320–324 pending Harpy acceptance

[Exact source matrix](../toy-effects14-coverage.md#source320324-pending-harpy-acceptance-and-fruitcorestar-mode-gaps-2026-10-17): eb02827f3 adds actual learned191891/371470→existing187854/display104851/source600000ms/item1800000ms to10disguises+Oomgut11IDs. Format/manual readability/diffcheck pass; new-head covering behavioral gate requested,477 throughd4 does not contain322. Fruit periodic child, four living-core model attachments/positioner and paired star mode/catch/knockback contracts remain gaps. NPC model1.5 is retained without inventing player-scale responsibility; no runtime/data/native admission or own heavy duplicate.

## 2026-10-17 — Toy source315–319 pending Cube acceptance

[Exact source matrix](../toy-effects14-coverage.md#source315319-pending-cube-acceptance-and-cachesongminesspecimen-gaps-2026-10-17): d4a4a716c adds Cube185567/105515/source900000ms/3600000ms recovery to now9disguises+Oomgut10IDs; old eight duration values/windows preserved via per-case duration. Format/manual readability/diffcheck pass; explicit lead477 queued, no new behavioral pass claimed. NPC model1.25/probability0 is preserved without inventing player-scale responsibility: source/repo transforms hand off the chosen display ID. Cache/trap, mixed Jiro song, six mine steps and specimen placement remain gaps; no runtime/data/native admission or own heavy duplicate.

## 2026-10-17 — Toy source310–314 pending acceptance

[Exact source matrix](../toy-effects14-coverage.md#source310314-pending-batonsreturn-home-acceptance-and-seated-orb-gap-2026-10-17): test/spec01de71c2e adds Batons900000ms caster row (14IDs) and three exact source10s/shared2309 actual saved-bind/refusal/return cases (13hearthstones). Format/manual readability/diffcheck pass; explicitly covering lead477 throughd4a4a716c is queued with result pending, no borrowed earlier result or own heavy duplicate.312 sitting/orb action stays unproved; no runtime/data/canonical, native/cross-map/full-recharge or new eligibility certificate.

## 2026-10-17 — Toy source305–309 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source305309-seasonalflowercooking-boundaries-and-pending-insoles-acceptance-2026-10-17):305 seasonal/285-child music,306 observed content-scale8.82/build12340 versus TDB1/build69497 plus SmartAI Grow49801, and307 nonzero-base/caster-hosted cooking remain gaps.308 retains exact earlier area/native scope, not a new calendar certificate.309 test/spec8a986da00 adds permanent188699/45416 to the caster fixture13IDs at8a986da00 (later14 at01de71c2e,309 unchanged); fmt/manual readability/diffcheck pass, explicitly covering lead477 throughd4a4a716c owns pending behavioral proof. No fake RED, runtime/data change, native/recovery/calendar/transmog certificate or own heavy duplicate.

## 2026-10-17 — Toy source300–304 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source300304-scoutingslime-trail-boundaries-and-retained-seasonal-banners-2026-10-17): Outland/two-island scouting mutations and Boots'180s/10ms-period→1s persistent child/moving-trail/pledge-OR-renown80 remain gaps.303/304 retain exact source9 actor103372/103384,2yd placement/owner/map/expiry CPU rows; no new current/native certificate, invented discovery set/unconditional trail timing, runtime/data change or generic script admission.

## 2026-10-17 — Toy source295–299 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source295299-exact-scouting-map-discovery-boundaries-2026-10-17): five source2s/self-script3 scouting maps retain distinct flight-point AND map-reveal contracts for Broken Isles, Draenor, Pandaria, Cataclysm-established maps and Northrend. Exact authored nodes/fog/eligibility/persistence remain unresolved; no name-to-all-nodes guess, generic cosmetic/no-op admission, player-progress mutation or positive certificate.

## 2026-10-17 — Toy source290–294 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source290294-retained-banner-proof-and-wingsridemetadatascouting-gaps-2026-10-17):290 retains source9 actual banner/destination/owner/map/expiry CPU scope;291 covenant-OR-renown wings,292 outdoor Control4 sphere,293 unresolved187861 metadata/seed action and294 real flight/map discovery remain gaps.293 Toy/Item/on-use rows exist without current ItemSparse/runtime item, so no synthetic name/template or guessed Flags2 policy. No runtime/data change, new admission or fresh positive certificate.

## 2026-10-17 — Toy source285–289 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source285289-crittercooldownstagefriendscontainment-boundaries-2026-10-17):285 covenant OR renown80/critter action,286 racial recovery,287 Control4 stage,288 three repeated child casts and289 Control4 imprisonment remain unproved. Preserve literal359469/173426 base fields rather than inventing child/count semantics, and do not collapse repeated triggers or source model scales. Existing103058 is not action acceptance; no runtime/data admission or new positive proof.

## 2026-10-17 — Toy source280–284 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source280284-controlled-cagestewardrealmcustomizer-boundaries-2026-10-17): Control4 personal/deluxe cages, pledged steward-companion transform, linked shadow realm and complete Racer customization graph remain unproved. Existing101685/101682/98480/source-selected11686 bodies do not establish control/recipient/child cleanup; no zero-weight visible realm substitute. No runtime/data admission or new positive certificate.

## 2026-10-17 — Toy source275–279 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source275279-offering-kit-targeted-acceptance-and-vespertransport-boundaries-2026-10-17): Harmony/Clarity/Faith retain distinct missing-runtime actors/scales plus caster dummy slots; Whistle retains fast/tired transport/area script gaps.279 source creates exactly one existing187342 after2s with30s item recovery; actual-use/inventory/reuse test/spec04a47a43 and fixture fix62db4212 pass1/1 zero ignored31.60s,1m30s compile/no compiler warnings, fmt/readability/diffcheck pass. Lead476 landed277 passed/master0b17f866f; no own repeat. Initial end assertion incorrectly rejected the two intended OnCooldown replies, now asserted exactly without clearing/suppression. No wrapping/gifting assumption, source actor substitution, runtime/data admission or native certificate.

## 2026-10-17 — Toy source270–274 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source270274-mirrorfatemixed-guiseshadowdead-party-boundaries-2026-10-17): mirror vehicle7544/periodic-zero-child copying, rune fate, Changeling transform/-45/312, player-target shadow child and fresh-dead party35 judgment remain unproved. Existing176673/101182/101766 is not complete mixed-graph acceptance; shadow child source33559 has weight0 versus39810 weight1. No generic actor/transform/corpse-output substitute, runtime/data admission or new positive proof.

## 2026-10-17 — Toy source265–269 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source265269-pet-chaseflightdestructiondragmixed-shade-boundaries-2026-10-17): mixed pet chase, permanent minipet payload, Minigrue toy destruction, controlled linked dragging and shade transform/screen/periodic children remain gaps. Runtime179941/39353 and180048/97411 exist; child180054 source repeats display101526 at distinct1.8/3 scales with3/1 weights. No payload/scale discard, actor/transform shortcut, runtime/data admission or new positive proof.

## 2026-10-17 — Toy source260–264 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source260264-paired-trophyoracledanceleashanima-service-boundaries-2026-10-17):260 requires both same-difficulty trophy actors,261 authored future consultation,262 nearby melody/dance,263 permanent minipet linking and264 outside-Torghast service.264 TDBvisible52318 has probability0 versus invisible11686/scale2 probability1; neither runtime publication nor service is proved. No one-slot/visible-body/no-op substitute, new runtime/data admission or positive certificate.

## 2026-10-17 — Toy source255–259 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source255259-returnmusicdisturbanceminipet-leash-boundaries-2026-10-17):255 cursor missile→ten-second gormling/run-back,256 ten-minute rooted harp music,257 pipe script/sound,258 casing disturbance and259 permanent minipet leash remain gaps. TDB94221/100008 are not runtime-published or action-proved. No static actor/caster attachment/silent-script substitute, new runtime/data admission or positive certificate.

## 2026-10-17 — Toy source250–254 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source250254-stackingdeath-viewminipetchasemulti-slime-boundaries-2026-10-17): coordinated steward stacking, seven-day viewer233 spirit-healer replacement, slime-minipet party appearance, mixed script/dynamic-ground laser chasing and exact count4/five-model slime lifecycle remain gaps. No caster/global-model/single-actor substitute, runtime/data admission or new positive certificate.No repeated gate.

## 2026-10-17 — Toy source245–249 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source245249-critterpassengerreflection-gaps-and-retained-hearthvesper-proof-2026-10-17):245 half-critter destruction,247 Control0 wounded-ally carrying and248 missile→reflection-summon lifecycle remain gaps.246 Hearth cooking/versatility and249 Vesper stationary actor retain earlier exact CPU scopes, not fresh native/audio certificates. Stretcher/reflection TDB models are absent runtime; no invented outcome/vehicle/model or inert admission.No runtime/data change or repeated gate.

## 2026-10-17 — Toy source240–244 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source240244-dominated-helm-targeted-acceptance-and-coinlaunchlinkedcovenant-gaps-2026-10-17):240 test/spec c8feec2e adds existing175221/nine-display transform/expiry/recovery acceptance; two disguise/Oomgut tests/nine IDs pass91.61s after21m25s compile, no compiler warnings, fmt/readability/diffcheck pass. Lead package gate472 landed276 passed/masterca715f278; no own repeat. Coin target action, combined health sacrifice/self-launch/geography, linked Krexus child and Kyrian pledge remain gaps. Existing175442/display99668 is not linked lifecycle proof. No runtime/data or new native admission.

## 2026-10-17 — Toy source235–239 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source235239-adviceridingsentryatonement-boundaries-2026-10-17): minute-period advice with trigger0, Journeyman-gated wheelbarrow riding, unwanted-entity warning and allied atonement remain unproved. Exact TDB175142/96515/1.15,175551/94282/1,175219/94615/0.4 are absent from runtime content; no inert-actor admission or invented vehicle ID. No runtime/data change, new positive proof or repeated gate.

## 2026-10-17 — Smelly Jelly234 acceptance and source230–233 gaps

[Exact source matrix](../toy-effects14-coverage.md#source230234-smelly-jelly-acceptance-and-hovermailteamoving-area-gaps-2026-10-17):234 test/spec e651544c adds existing174859/five-model actual learning/ten-minute restoration/one-hour recovery. Two disguise/Oomgut tests/eight IDs99.85s after5m51s compile, shared225/234 package/tests check2m25s/no compiler warnings, fmt/readability/diffcheck pass. Hover meditation, delayed friend-mail, unpublished type10 Sinvyr tea service and moving Bonestorm area action remain gaps. No runtime/data admission, manufactured RED or new native proof.

## 2026-10-17 — Toy source225–229 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source225229-necrolord-source-acceptance-and-retained-cookingquill-scope-2026-10-17):225 test/spec e2d1b991 adds exact learned bind/shared-charge acceptance; ten variants pass52.90s after6m04s compile, shared final check2m25s/fmt/readability pass.226 linked Cookpot/versatility and227 quill retain earlier bounded CPU proof;228 pledge and229 pledge/Maldraxxus recovery remain gaps.229 runtime173467/display94055 exists; no missing-template claim, runtime/data change or new native certificate.

## 2026-10-17 — Toy source220–224 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source220224-conditional-fall-and-reagentrandom-action-boundaries-2026-10-17): black-parasol negative generic-tick/refusal proof is not a successful falling contract. Purple/green next-fall consumption, combined Plume slow-fall/turn/area and book vellum/random-action remain unproved. Exact38682/count1 reagent retained; no invented outcome pool, new runtime/data admission or repeated gate.

## 2026-10-17 — Toy source215–219 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source215219-retained-vessel-proof-and-stackbobberareavehicle-gaps-2026-10-17):216 retains earlier actual learning/use/aura/cancel CPU proof;215 stack-conditioned shoulders/proc,217 bobber override,218 forced-cry area action and219 mixed vehicle/motion/party-devouring lifecycle remain gaps. Slime166086/display100400 already exists in runtime content. No new runtime/data admission, native certificate or repeated gate; Tea Set149865 remains lead-required.

## 2026-10-17 — Toy source210–214 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source210214-visualmotionreturnclasscovenant-boundaries-2026-10-17): multi-visual eyes, combined water-walk/hover, player throw-return, class-armor payload and advertised Night Fae pledge acceptance remain gaps. Condition0/shared2309 does not establish or negate all eligibility; no invented gate or borrowed nine-hearthstone certificate. Also name204's source35 precisely as party-target. No runtime/data change, new admission or duplicate gate.

## 2026-10-17 — Toy source205–209 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source205209-areavisioncompanion-boundaries-and-retained-aqir-proof-2026-10-17):205 creates area trigger19143, not a generic totem NPC; vision233, Control1/Title2 scale2 apple and minipet-target leash remain gaps.209 retains the earlier actual missile/selected-target/15s-aura CPU scope; no fresh native proof, runtime/data admission or duplicate gate.

## 2026-10-17 — Toy source200–204 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source200204-mogu-source-acceptance-and-companionequipmentgame-boundaries-2026-10-17):201 exact Mogu pool/lifecycle test/spec924bcde11 passes two affected tests/seven IDs107.53s after one14m03s compile; native check2m58s/no compiler warnings and fmt/readability pass. Companion-control description reference is not an executable child edge; headgear/shaving/skull-game actions remain gaps. Existing163441/95179 half-scale content is not missing publication or successful interaction. No production/data change or fabricated RED/native proof.

## 2026-10-17 — Toy source195–199 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source195199-motionfactionappearanceaction-boundaries-2026-10-17): Nomi self-knockback/safe-fall, N'lyeth language/faction/self-sacrifice and Scroll/cocoon/shadow appearance actions remain gaps. Preserve actual on-use314988 versus auxiliary314407/TriggerType7. No ordinary cosmetic/actor/transform substitutions, data/runtime admission or redundant gate.

## 2026-10-17 — Eternal Traveler190 source acceptance

[Exact190–194 matrix](../toy-effects14-coverage.md#source190194-eternal-traveler-and-mountcontrolportal-boundaries-2026-10-17): Eternal Traveler actual-learning/saved-bind/shared2309 acceptance brings nine variants9/9/79.23s; native check13.81s/no compiler warnings and fmt/readability pass at9e56fbe2a. No production/data change or manufactured RED. Wild Holly mount/holiday, RC Control3 and Shadowlands portal actions remain gaps; visual/cross-map/full-recharge proof separate.

## 2026-10-17 — Toy source185–189 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source185189-midday-source-acceptance-and-interactive-boundaries-2026-10-17): Jar187 test/spec9ec632d3a proves exact day/night screen IDs, duration and cancel-preserved item recovery; four affected tests57.78s after one cold26m32s compile, native check3m04s/no compiler warnings and fmt/readability pass. Fish-kicking/Gral/memento actions remain gaps; earlier Sealamp CPU/native/body/scale/state-kit/expiry scope retained without new certification. No production/data change.

## 2026-10-17 — Toy source180–184 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source180184-retained-midnightsign-proof-and-equipment-gaps-2026-10-17): retain earlier Shadescale learned-use/aura/cancel and both warning-sign actor/native expiry scopes without claiming native midnight or sign message carving/extended visibility. Dining/naga-equipment procedural payloads remain gaps. No runtime/data admission, native recertification or duplicate gate.

## 2026-10-17 — Memory178–179 lifecycle acceptance

[Exact175–179 matrix](../toy-effects14-coverage.md#source175179-retained-props-collection-gap-and-memory-transforms-2026-10-17): existing runtime Unshackled/Ankoan transforms gain actual-learning/source-pool/ordinary600000ms restoration and inherited3600000ms recovery acceptance. Eight tests/12IDs120.25s, native check33.49s/no compiler warnings and fmt/readability pass at59d1527ce. No production/data change or fabricated RED. Heart/Trophy earlier scopes retained; Mechagon collection action/native/all-model proof remain separate.

## 2026-10-17 — Toy source170–174 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source170174-standardinsigniaequipmentlinked-hologram-boundaries-2026-10-17): mixed Alliance Standard, sanctuary script routing, arm-equipment procedural and linked-hologram child remain gaps. Judgment154818/92155 exists in actual runtime content tables; absent base-table rows do not establish missing publication. Corrected earlier base/content misclassification; child behavior remains unproved. No data/runtime admission, borrowed prior native proof or redundant code gate.

## 2026-10-17 — Holographic165 and banner boundaries

[Exact165–169 matrix](../toy-effects14-coverage.md#source165169-holographic-hearthstone-and-banner-boundaries-2026-10-17): Holographic165 actual learned-use/saved-bind/shared-charge acceptance brings eight variants8/8/62.43s; fmt/readability and native package/tests check1m09s/no compiler warnings pass after capacity admission. Rustbolt333298 and mixed Horde War Standard remain gaps; source20 War Banner native/combat-removal proof retains its revision scope. No runtime/data admission or whole-Toy/native certificate.

## 2026-10-17 — Toy source160–164 boundaries

[Exact source matrix](../toy-effects14-coverage.md#source160164-robotteleportwhistle-boundaries-and-retained-ocean-proof-2026-10-17): Blingtron7000 missing actor/service, Kul Tiras/Zandalar script destinations and whistle transport remain unresolved. Ocean Simulator retains earlier revision-scoped simulation/native/PCM proof, not new current-master certification. Source/doc inspection only; no runtime/data admission or redundant gate.

## 2026-10-17 — Actual killed-creature level21

[Native78/statistic60938](systems/dungeon-achievements.md#actual-killed-creature-level21): TC inclusive actor GetLevel versus actual killed NPC GetLevel, positive21/0 plus complete Demon30/3. Production content view corrects legacy Imp4165 to actual tuned90; approved native41820/actor19,20,21 fixture gives genuine equal-level RED133f48a7. Native/group-own-level/copy/nonDemon/refused/dead/idle/rawpage/reopen/foreign GREEN; missing-level/type/actor/nonzero-offset catalog controls pass. Required integration partitions169/0 at4eadb217e plus73/0/1existing ignored/site1 at e3cb255c (242 unique server tests); no duplicate broad run. Current-master integrationaff7afe17 native/catalog2/2/43.14s, workspace/all-target check/no Rust warnings and fmt/diff pass. Compiler50+census3, all243 inventory rows exact, same7input hashes and only+60938stat1845; ordinary5565 unchanged. Bounded independent artifact/readability PASS, explicit expectation rows remove the accepted arithmetic-naming issue. No template edits, metadata fallback, acquisition/new eligibility policy, UI/fresh-UDP or remote-CI claim.

## 2026-10-17 — Literal world-state source boundary

[Constant67](systems/dungeon-achievements.md#constant-world-state-expressions67): enabled signed-literal comparisons only;21 exact results/10true, false3785 remains impossible. Full-companion native fifth-cast RED at ea92c91a; explicit decoded-source compiler context corrects initial overbroad admission, Python54/54. Structural census+21ordinary/+1stat; native GREEN2bcef1424/4 proves both full-five-spell grants/reopened dates plus false/pin/Gilneas controls. Integrated4bcd410a affected workspace240/0/1ignored+site1, one-hour-later relog dates, workspace/all-target check/no Rust warnings and3repo/native-Godot fmt/diff pass. Independent boundedModeB/readability PASS; missing Mephistroth/Warfront/raid availability unclaimed.
## 2026-10-17 — Level-based pet maximum health

[Bounded state2](../pet-battle-effect-coverage.md#level-based-maximum-health--implementation2026-10-17): explicit actual level1..25,5/5/10HP per level, existing percentage/floor/cap and owned-health persistence boundary. Core/owned unsupported-state RED and missing-metadata no-RNG RED precede GREEN; obsolete576 negative removed with remaining negatives retained. All ten fixture tables match active70448; fresh1797/2829/1023 census preserves historical70391. Library139 unchanged passes plus corrected negative1/1, wild/PvP29 plus real-ground configured achievement1/1, check51.00s/no warnings and fmt pass. Initial failures remain documented rather than claiming one all-green broad run. Native proof remains ordered after lead landing.
## 2026-10-17 — Nazan CT0 source data

[Nazan source data](systems/boss-scripts.md#ramparts-herald-identity-policy--2026-10-17): actual-copy `herald-nazan` CLI missing-profile REDb4a524732→source base0/Heroic2/models/addon projection. CT0/cache bounds1 stay input, not native-level acceptance; no Normal1, canonical placement/binding or original `herald` scope change. New/Herald/Omor3/3 at0a3ab4b24 (27.783s); final Nazan1/1 at7fe38b45e (26.460s), warning-free with ResourceWarning escalated. Five source-drift refusals preserve file bytes; repeat projection/unrelated data preserved. Runtime use/morph and encounter remain open.

## 2026-10-17 — Ordinary creature source context

[Ordinary source context](systems/level-scaling.md#ordinary-source-context--2026-10-17): sourced17536 ordinary map/actual-area/missing-context genuine3RED/2PASS→strict5/5. Actual importer provenance preserves authored fixed stats; the Alterac fixture now supplies real ground. Production-order deferred ground publication reaches genuineRED then strict1/1 at1050147f before ignore removal. Enabled7 context/source/startup cases and affected regressions pass54/54,177.24s at33dcb7b6 (55 pre-existing ignored). No canonical Nazan spawn or temporary-selection change; runtime source preparation and same-actor morph remain open.

## 2026-10-17 — Herald actor preparation

[Herald source actors](systems/boss-scripts.md#ramparts-herald-identity-policy--2026-10-17): actual-copy CLI missing-profile REDd46482dd→17537 base0/Heroic2 and21252 base0/Normal1/Heroic2 projection479979bd. First attempt preserves correct source gold, exposes fixture zero-gold mistake; independently audited concrete base/Heroic gold expectations and byte-identical refusal pass1/1 in12.947s at252a23c9, warning-free; unchanged OmorPASS retained. Nazan, unrelated difficulties, population, bindings and group357 unchanged. Context tuning and encounter runtime/native proof remain open.

## 2026-10-17 — Omor coincident source angles

[Coincident geometry](systems/boss-scripts.md#coincident-omor-geometry): actual30637 FALLING/30638 damage and ordinary30686 casting precede zero-horizontal-motion RED0/2/126.60s at8a24ba48. Pinned Position.h atan2(0,0)=0 requires+X reach projection/retreat; replace zero-vector normalization in those paths. Strict GREEN2/2/85.93s atf44f6a66 precedes ignore removal; ordinary affected9/9/160.75s atb2f0572f retains nonzero/range/shield controls; no authored data/timing changes.

[Bounded state2](../pet-battle-effect-coverage.md#level-based-maximum-health--implementation2026-10-17): explicit actual level1..25,5/5/10HP per level, existing percentage/floor/cap and owned-health persistence boundary. Core/owned unsupported-state RED and missing-metadata no-RNG RED precede GREEN; obsolete576 negative removed with remaining negatives retained. All ten fixture tables match active70448; fresh1797/2829/1023 census preserves historical70391. Library139 unchanged passes plus corrected negative1/1, wild/PvP29 plus real-ground configured achievement1/1, check51.00s/no warnings and fmt pass. Initial failures remain documented rather than claiming one all-green broad run. Lead gate426 lands140+54 before native. Legal Perky Pug250/7 then proves both skins464→499/Healthy9→464 on nine ordinary passes; Modern HP464→473→230 and retained Forever230→365→117. Six inspected fullHD PNGs at `/syncthing/AgentShared/2026-10-17/petbattle-effects/`; all phases two framed models/strict tripwire0, geometry93/104/101 only HP fills/timed floats. Existing7 canonical assets/1mesh loader pass, no publication. Both physical forfeits close; owned processes stopped/window cleared. Ancient Blessing/Knowledge native casts and Retail percentage ordering remain separate gaps.
## 2026-10-17 — Koranthal destination-delay assertions

[Koranthal timing](systems/boss-scripts.md#koranthal-destination-delay--2026-10-17): gate413 root is stale immediate timing after masterc99830b5, not OnSpellCast. Valid protocol e9b80b8 A/B each retains2 old failures while actual5s boundary passes. Restore diagnostic bypass; strengthen original tests with early exclusion/exact owned119964 impact/HP and unchanged absolute event clocks. Native data/runtime untouched; corrected original2 GREEN2/2/25.05s at4076e04a/protocol e9b80b8, pushed immediately; lead gate428 subsequently verifies4076 base334/0/12ignored1150.80s, test EXIT0/FMT0; later angle changes have separate proof.

## 2026-10-17 — Omor caster hold/retreat

[Caster movement](systems/boss-scripts.md#omor-caster-hold-and-retreat--2026-10-17): complete pinned UnitAI/MovementDefines/ChaseMovementGenerator establish non-melee ranged attack,100+both-reaches tolerance and contact retreat. Actual20yd approach/no3yd retreat RED2d93e936→strict GREEN2/2/86.49s at51cc638d; direct grounded driver only. Ordinary Omor10/source-range2/Gargolmar4/knockback9 GREEN25/25/539.12s ataa75ea23 includes numeric endpoints; obstacle-generated paths/100ms/native chase parity remain open.

## 2026-10-17 — Omor Shield OnSpellCast

[Shield cast emote](systems/boss-scripts.md#omor-shield-cast-emote--2026-10-17): natural31901/aura/no-emote RED91b61927→strict GREEN1/1/53.34s at15260692. Common successful-cast hook covers ordinary and both triggered routes, rejected requests stay silent; other-copy/repetition controls pass. Ordinary Omor8/missile9 integration GREEN17/17/291.23s atbf090cf6; summon-chance/slay/caster100yd remain open.

## 2026-10-17 — Lock-On target charge

[Bounded77](../pet-battle-effect-coverage.md#lock-on77--implementation2026-10-17): target-worn300/one-round duration, ordinary second-use direct blast, GM722 remains gated. Actual unsupported77 RED5→GREEN6 plus Rocket4/Wind-Up1 regressions pass;1779/2829 rows/1017 graphs. Authored wild-PvP29 and both strict skins pass enemy lock/no damage→145/143 blast/expiry, six inspected fullHD originals and canonical3-mesh loader. Final lib132/check1m02/no warnings/bothfmt pass; prior unchanged UI12 retained. Two fixture namespace compile mistakes corrected with actual library import; failed logs preserved. No cross-caster sharing or full timing/VFX claim.

## 2026-10-17 — Life Exchange health redistribution

[Bounded67](../pet-battle-effect-coverage.md#life-exchange67--implementation2026-10-17): direct active HP averaging/caps/miss conservation, no invented attack-power or callback behavior. Actual unsupported67 RED3→GREEN4;1778/2829 rows/1016 graphs. Owned/wild-PvP28 and fresh native builds pass. Tormented Spirit strict loader passes but its scale1.0 humanoid fails Modern viewport framing before cast/capture; camera/scale contract preserved, legal native subject switches to Sprite Darter87/15/slot2 with existing canonical6/6 closure. Sprite replacement owned1/1 and both actual strict native skins pass shared98/108, Scratch28/30, retained70→78 HP; four inspected fullHD originals/all four saved phases have two framed models. Final lib126/check1m25/no warnings/bothfmt pass; unaffected wild-PvP27 + replacement Sprite1 and unchanged UI12 scopes retained. Independent Retail rounding/feedback remains unproved.

## 2026-10-17 — Launch Rocket setup and launch

[Bounded75](../pet-battle-effect-coverage.md#launch-rocket75--implementation2026-10-17): exact293/effect508/aura294, existing charged direct-damage path. Unsupported75 RED3→GREEN4;1777/2829 rows/1015 graphs. Authored owned-team/wild-PvP27 and both strict native skins pass setup/launch win, six inspected fullHD originals/two framed models throughout. Final lib122/check55.43s/no warnings/bothfmt pass; wild-PvP27 and unchanged UI12 scopes retained. Initial duplicate caller `--locked` failed before compilation; corrected check passed, both attempts logged. No generic object-sharing/timing/VFX claim.
## 2026-10-17 — Guild effect source authority

[Mobile/resurrection audit](systems/guild-perks.md#mobile-and-resurrection-authority-audit--2026-10-17): existing authenticated faction1168 disproves the missing-reputation-provider assumption. Retrieved and pinned contemporary Mobile and historical resurrection fork candidates; neither establishes Mainline cast/use, resurrection qualification or ghost-stacking policy. No production behavior or effect completion claimed.

## 2026-10-16 — Armageddon team damage and sacrifice

[Armageddon1025](../pet-battle-effect-coverage.md#armageddon1025--bounded-proof2026-10-16): actual team-route rejection RED1→GREEN8, ordinary typed all-enemy routing and bounded self-kill/Failsafe suppression. Authored Rascal-Bot owned loadout covered;1776/2829 rows/1013 graphs. Both-skin strict native61/58 damage/caller321→0/forced reserve/actual use, eight fullHD originals and canonical loader pass. Final lib118/wild-PvP26/UI12/check1m24/fmt pass without warnings; fixed-protocol setup cleaned after proof.
## 2026-10-17 — Guild invitation appearance delivery

[Invitation appearance](systems/guild-lifecycle.md#invitation-appearance-delivery): real optional registered design uses one delivery loader without changing invitation storage. Request/reopen controls and independent368 gate pass. Both-skin actual UDP/native proof covers four invitations/physical declines and both optional states with identical artifacts, source-qualified paid private setup and four inspected full-HD originals; persisted-invitation replay remains a separate backend proof boundary.
## 2026-10-17 — Omor sourced pull124

[Omor](systems/boss-scripts.md#omor-orbital-hooks--2026-10-17): actual periodic30638 damage precedes physical no-reversal REDbf5659cd. Source handlerb509bd01 passes strict movement1/1/58.80s; enable only after this receipt. Ordinary Omor7/shared knockback9 pass16/16/435.67s at73932f34, including actual-child3D-distance/reach/no-teleport controls; map543 obstacle/Detour/dynamic collision and full native proof remain open.

## 2026-10-17 — Omor Orbital source hooks

[Boss scripts](systems/boss-scripts.md#omor-orbital-hooks--2026-10-17): native543 floor/parent publication and real30637 FALLING prerequisites reach two source REDs at860f0dfa. Port cast-local singleton continuation, exact2D selection, selected threat reduction and Whip's falling-player periodic replacement. Corrected Whip damage observer has genuine default-path RED95f9a4fa; explicit strict GREEN3/3 passes89.14s at8d16bd8e before ignore removal. Server pull124/native obstacle/full encounter proof remains separate.
## 2026-10-17 — Toy source155–159 boundaries

[Exact source matrix](systems/toys.md#source-equipmentlightoceangift-boundaries-2026-10-17) retains equipment/light/banner/gift gaps and distinguishes the unpublished Ocean's invisible source body from actual fishing interaction. No new runtime/data admission, native proof or redundant code gate.

## 2026-10-17 — Azerite151 actual-ground/recovery acceptance

[Source150–154 packet](systems/toys.md#azerite-real-ground-recovery-and-interaction-gaps-2026-10-17): existing151 behavior gains real Northshire and natural600s item reuse proof; four source/control tests and check/fmt pass. Matching-game/rally/jukebox/secret-fish actions remain explicit gaps; no policy, data or native presentation claim.

## 2026-10-17 — Toy source145–149 gaps retained

[Exact source matrix](systems/toys.md#proceduralcontrolscript-and-tea-set-gaps-2026-10-17) records blight procedural coating, Control4 spinners, Highborne script ambiguity and missing149865 Tea Set as gaps. No producer/runtime/canonical change or positive Tea Set acceptance;135's existing proof remains separate.

## 2026-10-17 — Brewfest140 and corpse/appearance boundaries

[Source140–144 packet](systems/toys.md#brewfest-hearthstone-and-corpseappearance-boundaries-2026-10-17): seventh actual holiday saved-bind/shared-charge case passes; exact enemy-corpse selector93, mixed Transmorpher action chain and dead-target soul action remain gaps. No new admission, canonical mutation or native presentation inference; exact proof/build scopes remain in the packet ledger.

## 2026-10-17 — Goldtusk135 and Fire Eater139

[Source135–139 packet](systems/toys.md#goldtusk-table-and-fire-eater-source-contracts-2026-10-17): genuine importer/actual-use omission→one authentic table admission, new Fire Eater saved-bind case and retained bowl/refusal controls. Final9/9, importer3/3, check/fmt clean. Copy receipt, later lead publication and read-only acknowledgment are recorded in coverage; the agent did not publish canonical data. No RC/native/food/ground-collision parity inference.

## 2026-10-17 — Toy source130–134 retained boundaries

[Exact source matrix](systems/toys.md#source-companionhealercorpsemusic-boundaries-2026-10-17) distinguishes pet-riding, procedural weapon, healer service and fallen-ally actions from generic handlers. Music Box149830 remains unpublished; its actual TDB chest model is not an invented replacement or music proof. Read-only evidence/docs only, no new admission or test recertification.

## 2026-10-17 — Noble Gardener125 and Citizens126 boundary

[Source124–129 packet](systems/toys.md#noble-gardener-and-source-anchor-boundary-2026-10-17): real Noble Gardener saved-bind/shared charge passes; Citizens Personal Anchor remains an explicit no-partial-effects refusal. Six bounded contracts/check/fmt passed at the retained pre-Scarab f9 scope and replayed byte-identically; no invented summon lifecycle, native presentation or new admission.

## 2026-10-17 — Scarab scatter oracle correction

[Gate412 source-ground metric](systems/toys.md#scarab-source-scatter-metric-2026-10-17): unchanged merged artifact fails2/30; actual778027 outer-ring RED confirms the3D-versus-horizontal mismatch. Keep5.001, exact first destination and separate secondary ground-height assertions. Six focused controls pass; final tightened two-case check/check/fmt pass. Runtime and prior native proof scopes unchanged; lead owns the next merge gate.

## 2026-10-17 — Dragon costume lifecycle118–123

[Six source costumes](systems/toys.md#dragon-costume-simulation-lifecycle-2026-10-17) pass actual learned-use/fifteen-minute ordinary expiry and natural recovery/reuse. Six new cases plus two related transform controls pass8/8 across72.48s; check/fmt clean. No production change; expected-refusal fixture failure is not runtime RED, native presentation remains open.

## 2026-10-17 — Toy source boundaries89–93

[Exact retained gaps](../toy-effects14-coverage.md#source-scriptcompanionheadwear-boundaries8993-2026-10-17): Poking child279545, three owned-critter150 roots and Brazier Cap conditional procedural payloads. [94–98 continuation](../toy-effects14-coverage.md#source-actionprocedural-boundaries9498-2026-10-17) retains nearby reaction, spectral/headwear procedural, corpse ritual and return/catch gaps. [99–103 matrix](../toy-effects14-coverage.md#source99103-existing-contracts-versus-missing-actions-2026-10-17) retains historical Cooler/Gong proof and exact missing Kovork/procedural gaps. No runtime admission; merged55-case proof unchanged.

## 2026-10-17 — Oomgut source lifecycle

[Actual learned use](systems/toys.md#oomgut-source-transform-lifecycle-2026-10-17) proves existing transform/ordinary ten-minute expiry and longer recovery without a production change. New plus existing transform2/2 and bounded source-contract14/14 pass; check/fmt clean. Native/all17/wall-clock claims remain excluded; RC105–106 and procedural107 remain gaps.

## 2026-10-17 — Source disguise recovery

[Three existing transforms](systems/toys.md#source-disguise-recovery-precedence-2026-10-17): actual learned-use proves source models, ten-minute ordinary expiry and item/spell recovery precedence. New three-ID test passes; final comparison with Oomgut passes2/2 at69.43s, check/fmt clean. No production change or native/weighted/full-recovery claim; Pennant110 script gap remains.

## 2026-10-17 — Lunar/Peddlefeet learned use

[Two new holiday cases](systems/toys.md#lunarpeddlefeet-hearthstone-learned-use-2026-10-17) prove existing persisted-bind/shared-charge path without production changes; bounded5 tests/seven IDs pass105.95s, check/fmt clean. Native/procedural/cross-map/recharge claims excluded; hiding114/gift115 remain named gaps.

## 2026-10-17 — Forlorn/charge merge

[Focused merge proof](../toy-effects14-coverage.md#forlorncharge-master-integration-2026-10-17): preserve effect96 movement-duration hit ordering and destination-only flight/front47; merged55/55 pass with actual protocol master. Lead published matching Forlorn template receipt; no new full-suite/native-parity claim.

## 2026-10-17 — Flashback source screen effect

[Flashback88 boundary](../toy-effects14-coverage.md#flashback88-screen-effect-boundary-2026-10-17): retain actual ScreenEffect1473 and chapel9587 source fields without inventing world phasing; actual learned use/native presentation remain open. Docs-only; retained150/49 proofs unchanged.

## 2026-10-16 — Eligible victim class52

[Victim class reward](systems/dungeon-achievements.md#eligible-victim-class52): native missing2359 and account max-vs-sum RED; batched35/53/52 with exact1..13 catalog support. Full thirteen-class GREEN2/2; structural5530/1841 adds245 only.
## 2026-10-16 — Miss-only pet self outcomes194

[Haymaker762 / Gift989](../pet-battle-effect-coverage.md#miss-only-self-outcomes194--implementation2026-10-16): bounded actual-miss gate and self selector, preserving block/success/unrelated-miss separation. Real RED2→GREEN8 includes owned loadouts/persisted injury. Both-skin actual self-stun/59–68 healing, successful-hit controls, ten fullHD originals and canonical strict readback pass. Final integrated lib111/wild-PvP25/UI12/check2m05/fmt pass without warnings.
## 2026-10-17 — Sprout native spawn aura

[Source boundary](investigations/follower-data.md#sprout-native-spawn-aura--2026-10-17): genuine eaad1801 native374161 RED38.63s follows all fourteen ordinary child/owner/map prerequisites. Prepared import preserves actual `npc_ley_line_sprouts`; missing runtime factory/non-boss branch, not data identity, is the cause. Minimal script binding/JustAppeared self-aura at39107add passes native1/32.80s with exact48e6/eight-source/frozenHEAD before-after; enabled only after GREEN. Next two native ArcanePower14/unowned92 cases reach genuine2RED55.66s atf15b03a3 after actual self-aura/summoner/capacity99 prerequisites (e87E0596 was test compile-only, not RED). Minimal source374736/native92 path at6b0ab342 passes unowned92 control but exposes real1vs14 native aura count (1PASS/1FAIL55.81s);9e helper-name compile failure was not behavior proof. Core beneficial-other-caster replacement lacked TC's native shared stack-slot rule. That source rule now retains actual caster/capacity without manual stacks or data/stat/geometry changes; c7aa2385 native owned14 GREEN1/53.95s plus native stackcap1/10.93s and periodiccoexist1/9.09s, strict48/ten-source/frozenHEAD guards. Unowned92 priorPASS retained; power tests enabled only after proof, replicated real-child caster/stacks14 check added. OrdinaryStomp276 relative-budget genuineRED835e53d0 is8631vs32797.8 at14stacks; minimal native mechanic consumer99cca08a passes matching Stomp and nonmatching Brand within34AzurePASS/0ignored507.23s/fmt0, exact48/twelve-source/HEAD stable. No damage/stat/HP override. Invalid --bin/check helper combination failed before compiling; corrected96da2b88 check0/zero compilation warnings1m39s, same12source/exact48DB/HEAD stable; independent source and full artifactPASS. No repeated Azure/fmt gate. Diagnostic registry276 classification is a separate source-observed follow-up. ArcanePower death cleanup/aura395/full lifecycle, rune197204/jumps713714 remain open;803defbf clock/child LANDED409/225 targeted.

## 2026-10-17 — Leymor first Fissure clock

[Source boundary](investigations/follower-data.md#leymor-first-fissure-clock--2026-10-17): e2a87993 reaches genuine missing20s386660 CastState RED27.99s after native floor/templates/fourteen ordinary Sprouts/living-victim/no-early-cast prerequisites. Minimal source event/victim cast/48s after-execution repeat passes native1/32.44s at31b0bbb8, with strict48e6 before/after and four source-native-root SHA matches; first-clock test enabled only after GREEN. Next ignored31sBrand/45sStomp cases pass real prerequisites/firstFissure/living victim then reach genuine missing CastState/Channel RED2/63.46s at828aafd4. Minimal source self-cast clocks/existing48s repeat reach native cast/channel at f733258d;2FAIL61.73s is the fixture's owner-only CastSuccess count for an unowned self-cast boss, not missing clocks. Fixture-only correction observes native visibility-broadcast self SpellGo; no production routing/HP/geometry change. Corrected8823a417 native2PASS48.35s/strict48/four SHA matches; both tests enabled only after GREEN. Next ignored cases reach genuine missing Brand374582 field and natural Stomp374731 damage RED2, with cancellation controlPASS (1PASS/2FAIL66.11s at e2febba9). Minimal nonexclusive Brand once-effect and existing natural-channel-finished callbacks at4a73c1cb pass native3/55.33s: two preserved parent auras, one native field/damage per recipient/750ms expiry, natural finish damage once and cancellation none. Strict48e6 before/after/six same-source native-root SHA matches captured before edits; no native data/timing/amount override. Three tests enabled only after GREEN; next Sprout374161 spawn-aura case ignored pending native RED. Final eaad1801 Azure29PASS/1nextignored116.19s,scopedcheck0/zero warnings2m43s,fmt0 and exact48e6/eight-source hashes/frozen HEAD before-after pass; independent source/readability PASS, final artifact follow-up pending. Separate next Sprout374161 native0/14 self-aura RED38.63s follows all owner/map prerequisites; no Sprout AI production yet. Native374582 is AT25390/750ms plus area damage, not an invented knockback vector; sprout lifecycle remains separate. Source394154 target110 and sprout lifecycle remain unported; no timing, damage, geometry or data override. Frozen Icy candidate eec25f11 remains separate; its final independent artifact follow-up now passes.

## 2026-10-17 — Telash startup prerequisites

[Source boundary](investigations/follower-data.md#telash-startup-prerequisites--2026-10-17): independent initialEnergy60 test reached genuine native RED (missing pool, native100 preflight passes). Native max100/source unit_class4 retained. Missing runtime rune197204 and jump-charge parameters are named gaps, not invented fixtures; native primary-pool binding exposes actual61.5vs60 from unauthored generic regeneration; authored-flag consumer preserves the controlled path and passes native2/2 at933bb833. Both regressions enabled after GREEN;958289d1 affected19+17resource+11guardian cases pass47, check/fmt pass with zero warnings and confirmed48e6 before/after. Nextnative389453 periodic+2/sec test reaches genuine60vs62 RED31.32s; effect0/PERIODIC_DUMMY hook passes native1/1 at37f4821e (30.49s, strict48e6 before/after), with no baseline/tick/capacity override. Enabled only after GREEN; periodic2b839e99 controls20/2nextignored, check/fmt and strict48e6 before/after pass. Nextautomaticengagement/3.5sFrost tests reach genuine RED2/22.78s after floor/living-victim prerequisites, and remain ignored until GREEN. Ordinary389453 activation/first3500ms Frost clock passes native2/2 at0d4e813a31.57s (strict48e6 before/after), then both tests are enabled. Affected73be52b7 has21PASS/one old baseline-control collision (legitimate scripted+2); fixture-only389453 cancellation isolates that control and passes1/1 at5d4a96e0 (31.55s/fmt0). Composite22-pass coverage retains21 unaffected cases/check0 without rerun, with strict48e6 before/after. First-rescheduled14.3s Icy case reaches genuine missingCastState RED16.09s after actual prerequisites;14500ms→firstFrost+10800ms/random-player ordinary cast is committed for targeted GREEN, with native child damage still unproved. Actual first-Icy cast/target Go succeeds but native387152 damage fails;15f0d3f7 traces two child rejections at unsupported selector76 (18.20s EXIT101). Sourced live-channel destination resolution at7950b87f passes native Icy1/15.30s, destination1/10.55s and Penance3/8.44s, with strict48e6 before/after and seven source/native-root SHA matches. Icy enabled only after GREEN; e2a87993 affected23/1nextignored75.79s,check0/zero warnings4.93s,fmt0 and exact48e6/seven SHA matches pass. Frozen source/readability inspection passes; test-only885a8220 disambiguates destination priority and passes focused1/6.22s plusfmt0/strict48 hashes. Retained Azure23/Penance3/check gives27 affected passes across explicit scopes, not a fresh27-case run. Final artifact follow-up remains pending. Next20s Leymor Fissure case reaches genuine missingCastState RED27.99s after real floor/templates/fourteen ordinary children/living-victim prerequisites; no Fissure production yet. No timing/damage/data override. Remaining phases/runes/jumps stay unported.

## 2026-10-17 — Leymor native Sprouts destination boundary

[Sprouts](investigations/follower-data.md#azure-leymor-sprouts-clock--2026-10-16): fixture-only prepared SummonLevels reload restores actual374360 one-child prerequisite; genuine374364 parent0/14 RED49.34s. Nonexclusive once-effect/full-mask implementation yields one ordinary native parent/fourteen owned children GREEN at8db5f60f on active70448/a0ccc53b. First28-child fixture manually stimulated a second parent; ordinary SpellGo witness corrects this without production changes. Affected17/0failed/1ignored TelashRED, scoped servercheck/fmt and bounded independent source/artifact/readability inspection pass at8ca93220. Lifecycle/full native70448 acceptance remain open.

## 2026-10-16 — Eligible victim race53

[Native reward](systems/dungeon-achievements.md#eligible-victim-race53): genuine both-faction missing race credit despite native35 and account max-vs-sum RED; batched35/53 implementation. Native/account GREEN3/3 proves both full ten-race grants and group/reopened date. Census5529/1841 adds246/1005; post-master filter224/0/1existingignored+site1/1, workspace/all-targetcheck/fmt/diff pass and independent artifact/readability complete.
## 2026-10-16 — Refuge Critter shields223

[Bounded1344](../pet-battle-effect-coverage.md#refuge223--implementation2026-10-16): independent Block759 for user/other friendly Critters, explicit wearer/original caster and shared self26 duration policy. Actual unsupported223 RED→GREEN9;1764/2829 rows (62.35%)/1010 graphs,2232/2. Both-skin private5961 independent caller/Critter blocks keep892/892, non-Critter978 takes28/26; eight fullHD originals/ExtractedOnly/tripwire0/canonical4-asset closure pass. Final CPU gates pending; no generic selector/full Retail timing claim.
## 2026-10-16 — Gargolmar source charge impact

[Boss scripts](systems/boss-scripts.md#ramparts-gargolmar-charge-impact--2026-10-16): corrected target-owner/delayed-impact observer exposes actual19yd no-motion RED. Generic two-point effect96 now arrives at source1.5yd reach with real damage, actual42yd/s bound and exact travel time; explicit ignored acceptance1/1 passes64.13s at91df4428, ignore removed. Obstacle paths/transports/native animation remain open. Final72fd14b3 ordinary4-thread filter214/0failed/12ignored571.47s and production check147.45s pass without warnings; fmt/diff/manual readability pass.

## 2026-10-16 — Wild summon lifetime and ordinary encounter gate

[Boss scripts](systems/boss-scripts.md#ramparts-omor-source-preparation--2026-10-16): generic owner-death cull reproduced before minion-only cleanup; wild expiry/Grimoire Imp3/3 and explicit ignored Omor1/1 pass at9be44701. Omor ignore removed, full native/Orbital acceptance remains open. Earlier pushedb8dc0d86 ordinary four-thread filter passed210/0failed/14ignored with endpoint fix147aa87b.
[Bounded1344](../pet-battle-effect-coverage.md#refuge223--implementation2026-10-16): independent Block759 for user/other friendly Critters, explicit wearer/original caster and shared self26 duration policy. Actual unsupported223 RED→GREEN9;1764/2829 rows (62.35%)/1010 graphs,2232/2. Both-skin private5961 independent caller/Critter blocks keep892/892, non-Critter978 takes28/26; eight fullHD originals/ExtractedOnly/tripwire0/canonical4-asset closure pass. Final lib104/wild-PvP24/UI12/check/fmt pass; no generic selector/full Retail timing claim.

## 2026-10-16 — Same-species pet damage370

[Dragon's Call2283](../pet-battle-effect-coverage.md#same-species-dragons-call370--implementation2026-10-16): explicit species identity from the owned/wild factory, existing direct-damage reuse and bounded real root; test-only templates rejected. Actual owned-team unsupported370 RED→GREEN6,1761/2829 rows (62.25%)/1009 graphs. Both-skin private5961 actual2283 deals85/77 with two matching reserves, HP/cooldown3 and strict-mode/tripwire0; four fullHD originals inspected. Adopted70448 canonical model closure/readback pass; final96/23/12/check/fmt and focused post-gate matrix1 pass, with no redundant broad rerun or blanket parity claim.
## 2026-10-16 — Hasty Human queue and native authority

[Guild perks](systems/guild-perks.md): per-consumption flat-adjusted bases/deadlines preserve the actual Human's mixed25/10/25-minute charge queue.17 charge controls pass, including unchanged rate-restoration expectations. Identical-binary actual item6948/8690 proof passes in both skins: four casts, four invitation/removal cycles, current1→0/max2 and exact114 non-guild IDs retained. Twelve inspected full-HD originals show bag/item/cast context; numerical recovery is native owner state. Extra haste/script controls and final checks remain pending; no whole-perks or pending-clock restart-persistence claim.

## 2026-10-16 — Guild mounted rates and Hasty charge boundary

[Guild perks](systems/guild-perks.md): supported Mount Up172/211/map suppression landed; living flat-ground native input and real guild removal confirm seven coherent samples in each skin, with ten selected mounted full-HD originals. Controlled-NPC210 remains blocked. Hasty8690/83944 already gives30/15/30-minute new-charge recovery; actual RED exposed joining halving a consumed clock. Preserving that453 consumption boundary passes3/3, including removal→recovery→next30-minute use; native actual-item proof is pending.

## 2026-10-16 — Bench pet resurrection112

[Sourced Raise Dead/Mass Resurrection](../pet-battle-effect-coverage.md#bench-resurrection112--implementation2026-10-16): four actual112 rows, inactive-slot routing and zero-HP owned-load state. RED3+persisted-redb1→GREEN4;1756→1760/2829 rows (62.07→62.21%),1006→1008 graphs. Both-skin private5961 actual2347 shows both78/1563 reserves, pointer swap and real147-damage win; eight fullHD originals inspected. Canonical78-asset closure/strict loader readback and final lib91/wild-PvP22/UI12/check/fmt pass; stale sibling `/hug` compilation drift resolved by lead-authorized fast-forward, no gameplay patch. Generic resurrection semantics remain gated.
- 2026-10-16: [Leymor Sprouts clock](investigations/follower-data.md#azure-leymor-sprouts-clock--2026-10-16): genuine3s cast RED→native self-cast/SpellGo GREEN1 at3b856731; unchanged-production Infused control PASS. Repeat/lifecycle runtime, fourteen child summons and fullAzure70448 remain separate; controls/check pending.
- 2026-10-16: [Leymor Infused Strike](investigations/follower-data.md#azure-leymor-infused-strike--2026-10-16): actual10s missing-cast RED after intro/living-victim checks; sourced finite schedule implemented, native GREEN pending. Intro scoped check passes.
## 2026-10-16 — Leymor intro and non-following formation depletion

[Source boundary](investigations/follower-data.md#azure-leymor-intro--2026-10-16): genuine startup/NPC RED4, then startup1PASS/actual-depletion3RED, then source groupAI0 membership/per-copy key4/one-second wake375749 and NPC mask pass. At6ae44dc0, five intro cases include removed leader and copy/reinitialize isolation; three Brand and six Frost controls pass14/14,0ignored. Independent source/artifact/readability and both format gates pass; scopedcheck/protocol unit control pending. Historical70391 proof, not70448 native acceptance or full boss phases.

## 2026-10-16 — Leymor Brand expiry hook

[Source boundary](investigations/follower-data.md#azure-leymor-brand-expiry--2026-10-16): native Brand374567 applied/expired but omitted374570 in genuine8049cebc RED; two controls passed. Finite source expiry hook5f1418c2 passes native3/3, fmt and independent source/artifact gate; six shared Frost controls and zero-warning server check pass. Complete Leymor/native acceptance pending.

## 2026-10-16 — Azure finite Frost Bomb chain

[Source and proof boundary](investigations/follower-data.md#azure-frost-bomb--2026-10-16): source-timed native Telash186737/205 genuine2RED (initial0.1s run invalid), then exact misplaced-ground RED;386781→386881 expiry-only387149+386910 and target-derived destination now pass6 native+3 adjacent controls, including the arena control with its existing fixture env. Scoped check/fmt and independent source/artifact gates pass; native pixels remain separate. No boss phases or unbound ground actions inferred; private225-actor import preserves canonical/Stockade proof snapshots.
## 2026-10-16 — Authoritative target LOS

[Cast receipt boundary](systems/spell-casting.md#authoritative-target-los--2026-10-16): moving-target ordinary loopback RED reproduces stale-client-endpoint refusal with clear authoritative sight. Validation now searches from authoritative caster/target positions; client geometry and chosen endpoint tolerances are removed from cast admission/queue. Wall controls and fresh release-server acceptance remain verification gates.
## 2026-10-16 — World honorable-kill35

[Native boundary](systems/dungeon-achievements.md#native-world-honorable-kills35): real lethal47XP/no35 RED and account max-vs-sum RED; minimal event/eligibility split implemented. Actual item/aura ten-kill GREEN3/3, reopened quantity/date and summed contributions pass;5527/1841 structural census. Required filter221/0/1existing ignored+site1/1, workspace/all-target check and fmt/diffcheck pass; independent artifact verification complete. Local install70448, imported data70391.

## 2026-10-16 — Two-way pet-wall150 charges

[Seven source-qualified pairs](../pet-battle-effect-coverage.md#two-way-walls150--bounded-proof2026-10-16): actual unsupported150 RED6→GREEN88, canonical journal3/wild12/PvP9 and check/fmt;1749→1756/2829 rows (61.82→62.07%),993→1006 graphs. Both-skin private5961 Ice Barrier479 physically blocks incoming then outgoing attacks, expires, and exposes subsequent45/65critical damage; six fullHD originals inspected. Conditional123/126 is bounded to six auras; Onyx, multi-wall order and exact Retail/VFX feedback remain gaps. Subject model/texture receipts match the current local root; owned-process cleanup passes.

## 2026-10-16 — Call Pet missing-storage refusal

[Pet lifecycle](systems/combat-pet-lifecycle.md#missing-storage-call-pet--2026-10-16): absent PetStable admitted883 success then effect returned without a pet. Actual native-target fresh-storage RED→GREEN; four controls and copied real character47 storage probe pass. No saved pet exists. Pinned creation-cast data has no Human hunter pet row; named source gap, no granted kit. Authenticated real-profile follower melee death already reaches its owner exactly once in a separate baseline test; terminal native delivery ordering remains under investigation rather than adding duplicate events.
## 2026-10-16 — Sourced Fade107 continuation

[Current70391 census](../pet-battle-effect-coverage.md#current70391-continuation--2026-10-16):105 referenced properties, exact rejected effect/turn IDs and123-state aura gate; Fade1084 alone adds3 admitted rows1746→1749/2829 (61.72→61.82%). Actual unsupported107 RED3→GREEN81, lib check/fmt and both-skin private5961 fullHD pointer/log/model proof pass. No canonical DB/index change; remaining unknown semantics and lead-owned asset publication explicit.
## 2026-10-16 — Guild perk membership knowledge

[Perks](systems/guild-perks.md): actual persisted join RED reproduced no perk grants. Membership-derived known IDs now remain separate from class/profession learning and survive ordinary spec rebuilds; owner deltas follow Update membership changes. Focused GREEN and real UDP/native/effect proof remain open.

## 2026-10-15 — Read-only follower group reward receipts

- 2026-10-16: [Arena residual](investigations/slow-native-test-fixtures.md#arena-residual-bounded-independent-fixtures-2026-10-16): LEAD-approved cap4 workers; all sixteen native cases pass52.70s with unchanged assertions/frames/50msstep; sampled CPU113.70s versus52.90s wall, full-suite separate.
## 2026-10-16 — Stampwhistle source assistant and mailbox

[Katy156833](systems/toys.md#stampwhistle-source-assistant-and-mailbox-2026-10-16): genuine importer/template, journal Assistant35 and NPC mailbox REDs; preserve actual default TempSummon flags/faction instead of owned-pet sanitization. Ordinary mail service sends real persisted letters and keeps distance/death guards; full600s summoned lifecycle and service pass2/2 after per-frame transient-message fixture correction. Whole-Toy143/143, affected mail/NPC/summon49/49, importer controls3/3, check/fmt pass. Canonical370 still refuses missing132969; prepared392 is classification, not completed toys. No canonical/native-rendering claim.

## 2026-10-16 — Sightless detection consumer

[Sightless153293](systems/toys.md#sightless-detection-consumer-2026-10-16): journal RED rejects152 despite an existing native target-detected-range consumer. Real hostile12yd/7yd controls pass before minimal registry admission; actual learning/UseToy plus aggro GREEN2/2 retain source group5454/600s aura/300s recovery and ordinary expiry. Whole-Toy142/142, creature-aggro30/30, binary/tests check and fmt pass; canonical classification369→370 adds only153293. Native delivery/rendering remains separate; remote-view/possession roots unimplemented.

## 2026-10-16 — Waywatcher source lifecycle

[Waywatcher130249](systems/toys.md#waywatcher-source-lifecycle-2026-10-16): actual Val'sharah terrain/learning/instant use/source model choices/15s expiry/900s recovery pass1/1 using existing published99572, superseding the old missing-template snapshot. Eighteen same70391 assets and four aliases authenticate/publish privately; canonical ownership absent before publication. Actual private Rust loader, whole-Toy141/141, server binary/tests check and fmt pass; native rendering remains separate.

## 2026-10-16 — Grizzlesnout source lifecycle

[Grizzlesnout129965](systems/toys.md#grizzlesnout-source-lifecycle-2026-10-16): actual learning, terrain-backed Northrend admission,1500ms cast, source9287/3.5/241/15s expiry and600s recovery pass1/1. Test-only existing-handler acceptance; twelve canonical body receipts/two aliases authenticate without new publication. Actual canonical Rust loader, whole-Toy140/140, server binary/tests check and fmt pass; native rendering remains separate. Next130249 now has published99572, superseding the earlier missing-template snapshot.

## 2026-10-16 — Forced-forward flag boundary

[Aura241 flag](systems/toys.md#forced-forward-flag-boundary-2026-10-16): source148626/194807 overlap/removal RED→GREEN1/1 preserves unrelated bits. Generic native input35/35 and whole-Toy139/139 pass at the recorded62 revisions; actual104329 mount/body/lifecycle remains deferred for missing73689. No complete-toy claim.

## 2026-10-16 — Ysera source admission and private body closure

[Ysera source boundary](systems/toys.md#ysera-source-summon-boundary-2026-10-16): genuine importer/use RED, authentic99518 admission, six importer controls and actual private-owner/front-offset/static120s simulation lifecycle pass. Whole-Toy138/138 and check/fmt pass after module-order-only formatting repair. Seventeen70391 body dependencies plus four aliases authenticate/publish privately; procedural8605/type22 remains unrendered. Prepared classification adds only130170 to385/1166, not complete-toy parity; earlier interaction and guardian HP gaps remain.

## 2026-10-16 — Stone Whistle source admission

[Enchanted Stone Whistle](systems/toys.md#enchanted-stone-whistle-source-summon-2026-10-16): actual importer and learned-use RED reproduce missing98300. Add only authentic Petrified Sand Piper through the maintained importer; existing destination/lifetime handlers are unchanged. Five importer controls, actual learned-use lifecycle, whole-Toy137/137 and server check/fmt pass; prepared classification adds only129279 to384/1166. Follow-up60 authenticates five70391 model/texture/skin assets and privately publishes them with two runtime aliases through the maintained importer; rebuilt graph is complete. Canonical publication, native rendering and guardian HP remain separate.

## 2026-10-16 — Shrine server per-target conversion

[Source stat table](systems/toys.md#shrine-server-per-target-stat-conversion-2026-10-16): real learned-use actor has client viewer20 but server level30/armor155 RED. Retain raw difficulty DamageModifier and build the existing per-level ExpectedStat table for final summon tuning; nineteen focused controls pass concrete level/armor/dealt/loss outputs. Final exact seven master regressions plus whole-Toy143/143, server all-target check, fmt and maintained build pass. Actual combat hits, client HP and guardian stats remain separate; previous wire evidence is not relabeled.

## 2026-10-15 — Shrine viewer-level projection

[Source boundary](investigations/follower-data.md#group-reward-receipts--2026-10-15): opt-in corpse-generation/roll-start traces expose actual eligible state and item IDs, without changing loot. Native Stockade Moloch/Overheat credit2/3 is separate from unclaimed rewards; Overheat's configured row is quest-only. Owned-evade9b lifecycle3+ordinaryNPC1/check/build/fmt pass; native retention/route/loot acceptance remain open.
## 2026-10-16 — Additional optional resource cost

[Native63](systems/auras.md#additional-optional-resource-cost-2026-10-16): successful native cap6 Envenom leaves1CP RED; signed caster/power/family optional-cost consumer passes5/5, including6/7 spend, availability, unlearning, foreign and mismatched native-mask controls. Anticipation is only a native cap fixture, not a legal build. Merged32 focused resource controls pass; fmt/manual Rust readability pass. Workspace/all-target check held by the bot's live window, not yet claimed. Optional percentage costs, flat-modifier exclusion, full talent/legal and fresh UDP acceptance remain open.

## 2026-10-16 — Autoattack-only critical chance

[Native334](systems/auras.md#autoattack-only-critical-chance-2026-10-16): TC Unit2387 traces Avenging Wrath's separate334/290 effects. Ordinary native100/-999 endpoint casts reproduce missing positive/negative swing modifiers; shared attacker-only consumer passes4/4, including cancellation, victim ownership and spell isolation. Adjacent attack/critical-damage controls36/36; workspace/all-target check and fmt pass. No native stat/RNG/clock edits; full Wings/talents/legal builds and fresh bot UDP remain open.

- 2026-10-16: Added [Razorlash source binding](systems/boss-scripts.md#maraudon-razorlash-source-binding--2026-10-16); binder13/13 and real copy-only generation, runtime proof pending.
## 2026-10-16 — Thermaplugg source identity and linked bombs

[Thermaplugg](systems/boss-scripts.md#gnomeregan-thermaplugg-source-binding-and-bombs--2026-10-16): Journal70391CSV422/992/display36563 plus pinnedTC/TDB authenticate7800→382/bit5; producerRED→GREEN12/12 and actualSQL generation. Ordinary spell damage/isolated controlled credit and linked30s bomb pair GREEN2/2c0e41c8a; fixture retains real7915 template and observes exact birth positions before unmodified chase. Strengthened spell-repeat and second-pair checks pass2/2 at43dd64c6 (41.16s), check/fmt/diffPASS; strict TC timer<diff requires repeat capture by60.05s. No ordinary/native/full-instance claim.

## 2026-10-16 — Crowd Pummeler source clock and identity

[Crowd Pummeler](systems/boss-scripts.md#gnomeregan-crowd-pummeler-source-clock--2026-10-16): authenticated6229→381/bit4 primary, missing-profile RED→GREEN10/10 and actual SQL generation; real SAI first/repeat clocks and positive damage for8374/5568/10887, isolated controlled death credit and stopped post-death casts GREEN1/1 at9b0e0235. Only player survival health staged; no resource/timer/chance/flag overrides. Ordinary fight, knockback/interrupt, loot, persistence and native clear remain open.

## 2026-10-16 — Electrocutioner identity and explicit mana gap

[Electrocutioner](systems/boss-scripts.md#gnomeregan-electrocutioner-binding-and-source-gap--2026-10-16): copy-only pinned TC/Journal primary6235→380/bit3 RED→GREEN9/9, actual source generation succeeds; controlled death/copy credit GREEN1/1. Real SAI acceptance remains source-data RED at Shock NotEnoughPower(Mana), TDB class1; lead approved named ignore without mana/class/triggered overrides. Blackfathom legacy/modern identity mismatch recorded, no substituted credit.

## 2026-10-15 — Controlled Disciple gossip/start

[Source callbacks](systems/boss-scripts.md#wailing-controlled-gossipstart--2026-10-15) port Hello/Select and events1–3 without synthesizing flags/menu options/5232. Genuine missing-Hello RED→controlled2/2 GREEN56.54s, including real flag0 InteractNpc rejection and physical first-path movement. Missing NpcInteractionPlugin/faction initialization caused earlier empty/panicking ingress fixtures; no transport behavior changed to fix those fixtures. Integrated3a035ee1/protocol1707389: controlled2/2(60.82s), ordinary NPC controls9/9, first-camp regression1/1(25.18s), scoped check without warnings and fmt/diff pass. Natural positive activation and full escort/native clear remain open.
## 2026-10-16 — Touch of Death primary damage and native flags

- 2026-10-16: [Slow native fixtures](investigations/slow-native-test-fixtures.md): three measured RED→GREEN profiles; master6403a988 integration passes14/14 unchanged gameplay/isolation/imported-copy controls410.83s and warning-freecheck11.37s. Arena residual>60s and full-suite timing remain separate.

[Health damage](systems/spell-casting.md#touch-of-death-primary-damage--2026-10-16): native999-health/zero-damage RED→GREEN100%/35% maximum-health outcomes with actual armor, school modifiers and Can'tCrit. Shared source attribute consumers honor caster-percent/victim/armor exclusions; ordinary-modifier and adjacent controls62/62 pass. PvP/Stagger/mastery/follow-up talents and fresh bot acceptance remain open.

## 2026-10-16 — Touch of Death admission

[Target policy](systems/spell-casting.md#touch-of-death-target-admission--2026-10-16):70391 weaker-creature/improved15% admission; seven actual rank/health cases RED→GREEN2. Reject before CastSuccess/cooldowns. Strict description boundary retained with SimC equality discrepancy documented; damage/PvP/follow-up talent consumers remain open.

## 2026-10-16 — BM Kill Command owned pet damage

[Source bridge](systems/spell-casting.md#beast-mastery-kill-command-pet-damage--2026-10-16):70391 names83381 and pinned SimC identifies its pet caster. Actual Call Pet/owner intent reproduces missing damage before the bridge; native BM/Survival/foreign-owner controls pass3/3. Extra talent/charging/admission and fresh UDP acceptance remain open.

## 2026-10-16 — Neutral DK faction creation policy

[Source creation boundary](systems/race-class-eligibility.md#neutral-pandaren-death-knight-creation--2026-10-16):70391 neutral24/DK6 model rows are valid, but Retail explicitly requires early faction choice for DK. TDB has25/6 and26/6 Icecrown starts, no24/6. Reject neutral identity without inventing a start; faction DKs/neutral Monk/Dwarf Shaman preserved. Exact master all-pairs creation test reproduced RED before fix; scoped gates pending.

## 2026-10-16 — Authored scenario-step kill guard

[Positive82/native0](systems/dungeon-achievements.md#authored-scenario-step-kill-guard) resolves current authored IDs via OrderIndex=Asset-1, not vector positions. Compiler46/46/generated5783; genuine native component RED→GREEN2/2; structural5517/1824 adds four account roots. Explicit run binding, rejected1031 admission, missing original research content and stages7/14/17 remain named gaps.

## 2026-10-15 — Completed-quest login guard

[Positive110/Login74](systems/dungeon-achievements.md#completed-quest-login-guard) retains complete companions and authored metadata, reusing actual character reward history. Compiler45/45/generated5768, native real-selection RED→GREEN4/4, census5513/1824 adds only account17495. Integratedbd808152 required four-filter208/0/1ignored plus site-export1/1; server/engine checks and formatting pass warning-free. Existing additive login counters and original account dates preserved; quest acquisition unproven.

## 2026-10-15 — Nested encounter conjunction
## 2026-10-16 — Guild news real producers and private readback

[News](systems/guild-news.md): actual achievement, loot-slot, CREATE_ITEM24 and vendor boundaries; authenticated query/sticky and live subscriptions. Backend6/6 and final consumer/wire/check gates pass. Genuine private Lillehoff acquisitions drive250-row native both-skin retention; real permission/post-quit refusals and exact fresh-process history/sticky readback pass. No synthetic history or catalog changes; independent lead integration remains separate.

## 2026-10-15 — Readiness real-time clock

[Clock isolation](systems/spell-casting.md#readiness-clock-isolation--2026-10-15): real60-second slow-frame owner-message RED→GREEN for Overpower/Throw Glaive, sourced Fury Slam GCD lifetime and67 focused controls. SpellHistory uses real elapsed time; simulation/aura/cast/missile clocks unchanged. Fresh native/UDP and independent acceptance remain open.
## 2026-10-15 — Shrine viewer-level projection

[Viewer-level boundary](systems/toys.md#shrine-viewer-level-projection-2026-10-15): actual learned Shrine projects30 to viewer20 RED; selected regional range and same rolled delta now survive wild initialization. Nine targeted controls pass, including Podium's zero-tuning native-level behavior. Whole-Toy136/136, six current build/check/fmt gates and fresh replicated Shrine/Podium/default-Scarab viewer projections pass. Admin creation-timeout evidence retained; only unfinished scenarios resumed with new accounts. Server-combat tables and exact client HP remain separate.

## 2026-10-15 — Guild lifecycle query boundary

[Lifecycle history](systems/guild-lifecycle.md): additive history table and authenticated actor-guild query; same-transaction six-action/founder producer wiring and100-row insertion-order retention implemented. Actual six-action RED observed an empty history; targeted4/4 now passes six-action/refusal/privacy/reopen, founder join labels,100-row tied-order retention and abort. Actual UDP transport1/1 and both-skin native real-action/privacy/100-row overflow acceptance now pass; fresh server/client processes preserve exact100 raw events/timestamps with identical binary receipts. Eleven inspected full-HD PNGs and durable trial evidence linked in the system page; refreshed-master gates remain separate. No fabricated/backfilled history or bank/news reuse.

## 2026-10-15 — Merged IoC vehicle foundation gates

[Demolisher](systems/mounts.md#ioc-demolisher-seats--implementation2026-10-15): serverd414bb03/protocol5ce52e0 pass19 foundation+8bomb/AV/arena controls, runtime check and shared handshake2. Client source/scene/UDP proof linked; no driving/fire/both-skin model claim. Earlier fingerprints runner error and fixture integration compile error remain separate from gameplay RED.

## 2026-10-15 — General promoted-map spellclick publication

[Source spellclick publication](systems/mounts.md#demolisher-spellclick-publication--implementation2026-10-15): lead rejects NPC-specific filtering; authentic map628+559 RED expects nine source rows, got one. Refresh all spawned-entry rows, preserve unrelated entries and transaction rollback on absent source table. Process GREEN4/4 atd0c7a255,168.538s,0skips; no canonical write.

## 2026-10-15 — Normal Demolisher spellclick promotion

[Demolisher source row](systems/mounts.md#demolisher-spellclick-publication--implementation2026-10-15): authentic copy/map628 promotion RED proves34775→66245 omitted. Exact source-row copy and missing-source rollback added inside normal population transaction; process GREEN pending, no canonical write. [Movement metadata](systems/mounts.md#vehicle-movement-metadata--implementation2026-10-15) importer3PASS retains source Flags/TurnSpeed; current driving/native gates remain open.

## 2026-10-15 — Authored vehicle movement metadata

[Vehicle catalog](systems/mounts.md#vehicle-movement-metadata--implementation2026-10-15): actual source Flags/TurnSpeed omission RED, importer projection/seat-only schema upgrade added without synthetic defaults. Fresh/upgrade/idempotence/missing-source preflight controls pending; no canonical publication or movement/native claim.

## 2026-10-15 — Independent vehicle control-aura ownership

[Demolisher seats](systems/mounts.md#ioc-demolisher-seats--implementation2026-10-15): driver/source-pose/normal-exit/reentry/departure and vehicle/rider death release pass3/3, deploy closure4/4. A second real friendly caster then reproduces seat0 instead of2 because generic aura replacement ejects the first driver. TC vehicle-control stacking rule added; same occupancy/overfill/seat2-reuse test GREEN1 at31939668. No native movement/fire claim.

## 2026-10-15 — Demolisher boarding implementation, verification pending

[Source NPC seating](systems/mounts.md#ioc-demolisher-seats--implementation2026-10-15) adds aura236/Vehicle509 occupancy, source bind-pose placement and lifetime hooks after genuine processed spellclick RED. Faction1732 now passes before the missing-driver assertion. No boarding GREEN, deploy closure, client attachment, movement/fire or native acceptance claim yet; protocol epoch2 needs coordinated server/client gates before landing.

## 2026-10-15 — IoC authored spawn-faction regression

[Source per-spawn faction](systems/factions.md#isle-per-spawn-script-factions--implementation2026-10-15): actual Workshop/Demolisher35→expected1732 RED; existing CreatureStringIds join retained, only TC OnCreatureCreate mapping added. No content schema/import or fixture-only faction. Verification pending; processed66245/aura236 missing-boarding RED remains separate. [Both-skin bomb proof](../../../game-engine/docs/wiki/systems/battlegrounds.md#forever88-functional-bomb-chain--2026-10-15) is complete only at its four-phase/return boundary, not vehicle/siege/victory.

## 2026-10-15 — Modern native activation boundary

[Client-owned native evidence](../../../game-engine/docs/wiki/systems/battlegrounds.md#modern83-functional-bomb-chain--2026-10-15) records normal/huge typed hits, real picked-model disarm and20-player return/save on007/da6/a438/4135 components. Forever and explicit renderer/framing/timing gaps remain; no vehicle/siege/victory certification.

## 2026-10-15 — Owned IoC bomb lifecycle channel proof

[Timed Seaforium](systems/game-objects.md#source-timed-seaforium-lifecycle--active-2026-10-14): normal/huge real pickup→owned item→UseItem, ten-second25%/60% one-shot/copy isolation and acknowledged normal disarm/cancellation pass5/5. Authoritative BG team relation regression passes1/1 across racial factions; reproduced/fixed NPC-path regression passes existing AV captain1/1 without changing stats, arena lifecycle remains1/1. Native activation, siege and victory remain pending. Earlier too-early/transient-reply fixture observations are not genuine disarm RED; corrected pre-fix replay is retained separately.
## 2026-10-15 — Authored talent Starter builds

[Starter Build](systems/talents.md#starter-build): authored cumulative ranks, normal eligibility and built-in selection in existing redb encoding. Five scoped Starter proofs pass, including installed-class legality and Arms apex deferred until authored levels; check/build have no warnings. Real private both-skin level10→11→80, exact relog projections and confirmed deactivation pass. No reference-data/default-realm mutation; coupled landing remains lead-owned.
## 2026-10-15 — Nested encounter conjunction

[Native165 nested All](systems/dungeon-achievements.md#nested-encounter-conjunction) preserves complete companions and scoped prerequisites. Compiler44/44, genuine native-handler RED→GREEN3/3, exact structural5512/1824 adds only11660. Required integrated four-filter205/0/1ignored plus site-export1/1 at299b09a9; checks/fmt pass. Initial missing fixture-env setup failures retained, corrected invocation green without source changes. No boss combat or prerequisite acquisition certificate.

## 2026-10-15 — Absent-player-aura kill guard

[Bounded SingleFalse8](systems/dungeon-achievements.md#absent-player-aura-kill-guard) on KillCreature0 requires known actor aura authority and complete authored companions. Compiler43/43, genuine native RED, structural5511/1824; native/catalog/prior-positive/pinned GREEN5/5(39.10s) at2dc751cc. Native counters, genuine actor mark casts and DB reopen pass; controlled boss identities do not certify encounters or full-map grants.

## 2026-10-15 — Wild summon initial-state source RED
## 2026-10-15 — Wild summon initial-state source and wire proof

[Wild55](systems/toys.md#wild-summon-initial-level-and-stats-2026-10-15): genuine learned-use RED corrects regional level with matching HP/armor/mana, and NPC-child RED removes fabricated owner1. Authentic terrain/relative fixture geometry closes ty232/ty239; entire toys:: filter passes, followed by current server check/format/build and fresh private Podium/Shrine/default-Scarab initialized-field wire. Exact revisions, hashes and retained failures live in the linked coverage ledger. Broader scaling/combat-stat and older pixel/lifetime recertification remain open.

## 2026-10-15 — Winner's Podium source placement

[Podium54](systems/toys.md#winners-podium-source-placement-2026-10-15): real-source RED3 for missing240220, scaled player-front minimum and destination138 airborne height; minimal source fix GREEN6/6 at3c2d0bb6, with real learning/use/20min simulation expiry and retained62/Bashiok/Shrine87 controls. Importer3/3, postmerge20/20/check/format/matching builds and both-skin native source/static proofs pass. Final clamp extraction377d5139 rechecks six affected controls58.66s/check/format/build; native receipts retain ed425/fe1d/ab40 scope. Prepared383 adds only239018, not383 complete;20min native wall expiry remains unproved. No runtime clock or cooldown changes.

## 2026-10-15 — Source guardian level selection

[Guardian51](systems/toys.md#source-guardian-level-selection-2026-10-15): actual UseToy owner80 Scarabs RED80→GREEN30, owner20 retained20; template/map/destination-area tuning and difficulty delta precede guardian initialization, explicit UseCreatureLevel. Thirteen final controls pass at0e2c24e8, including retained class/count/follow/expiry/startup fixtures. Merged check/format/build gates pass; fresh private UDP bag-learning/UseToy receives29/26 own Scarabs at20/30 for owners20/80. Exact binary/source ledger linked from the system page. Complete guardian-stat/AI parity remains open.
## 2026-10-15 — Installed race/class source

[Eligibility source](systems/race-class-eligibility.md): real Dwarf3/Shaman7 admin rejection despite installed membership drives removal of the copied Retail matrix. Installed283-pair/missing-source/actual admin kit+persistence regressions pass3/3; post-master race/mask/start/Forever controls10/10 and scoped check/fmt pass. Source95/96 contracts, start-level and visage policy remain separate and unchanged; rendered Dwarf login is not claimed. No realm5000 operation or stat tuning.
## 2026-10-15 — Player-owned evade aura retention

[Native source boundary](investigations/follower-data.md#owned-evade-aura-retention--2026-10-15): Stockwait67 credits Moloch/Overheat but later loses all Garrick passives on evade (10584→7840HP,68→0SP). Shared cleanup now follows TC Unit.cpp4461 owned-unit exclusion. Real initialized-spec→evade→stat-refresh RED1 matches7840; GREEN/build/native retention pending. Route/loot gaps keep fullStockade/Azure gates open.

## 2026-10-15 — Follower passive-derived stat recomputation

[Source boundary](investigations/follower-data.md#passive-derived-stat-refresh--2026-10-15): NPC bridge passives stored modifiers but the player-only query never recomputed them. Native spec66 RED2 proves7840 versus sourced10584/8232 without stat overrides; same production query now accepts follower profile identity and ordinary modifier changes. GREEN/build/live proof remain separate; Stockade0/3 stays open.

## 2026-10-15 — Breakable CC target protection

[Source boundary](investigations/follower-data.md#breakable-cc-targeting--2026-10-15): follower hold/retarget and unfinished hostile cast/swing stop; Attr6 `0x100` implicit chain/area exclusion. Native118/profile/31935/176017 RED8 plus explicit-player positive control; GREEN/build/live proof tracked separately. Stats, CC budgets and launched missiles unchanged; Stockade0/3 remains open. Subsequent native9/9 and integrated13/13/check/builds pass. Stockccguard52 observes zero NPC interference across three real118 intervals but loses Garrick; separate diagnostic stop/cast same-Update race replay RED1→GREEN3. Next native run completes3/3 Polymorphs and14 trash with full party, then reproduces an authored ramp-lip wall-ray block; same-ramp analytical detour prepared. Next normal-ADD native run reaches that landing but loses Garrick; the global waypoint also altered the initial approach, so further route changes await a scoped lead decision. Stockade0/3 remains open.

## 2026-10-14 — Source-correct proximity distance

[Creature aggro](systems/creature-aggro.md#aggro-on-sight) replaces the pet-style detection radius with TC a352b1fa GetAttackDistance, level/expansion and detect-aura rules, authored scaled combat reaches and strict distance. Stockade30vs32 native player/companion regressions RED2→GREEN34, including positive boundary and scaled cross-cell controls. Check/build/live rerun remain separate gates; no claim that radius alone caused the prior over-pull.
## 2026-10-15 — Flute follow source adjudication

[Wild/default route](systems/toys.md#flute-follow-source-adjudication-2026-10-15): current70391 effect142917→property64 Control0/Title0/Slot0/Flags0 selects TempSummon and no follow-on-spawn in pinned TC. Preserve source non-follow; no descriptive-text AI/distance. Independent Retail observation and fresh native source-model pixels remain unproved.
## 2026-10-15 — Unearned-achievement cast guard

[Bounded SingleFalse87](systems/dungeon-achievements.md#unearned-achievement-cast-guard) on CastSpell29 uses existing scoped earned state and requires authored metadata. Final native/catalog/prior-positive87/pinned9dfe04b9 GREEN6/6(45.26s), compiler40/40/generated5737; actual twenty1217637 casts grant account41707 then stop, preserving dates/counters across DB/auth reopen and account boundaries. Census5495ordinary/1824statistics adds only41707; no general negation or gear acquisition/encounter/UI certificate.

## 2026-10-15 — Full native pet-family grants
## 2026-10-15 — Spell-created unit ownership logs

[Combat log ownership](systems/spell-casting.md#messages): actual effect28/effect56 creations emit Summon with the real spell and actor identities; temporary guardian ancestry delivers combat to its owner without changing gameplay credit. Login restoration emits no synthetic event. Bounded native gates tracked separately from DPS/balance acceptance.

## 2026-10-15 — Source Toy forward destination

[Bashiok destination](systems/toys.md#source-forward-destination): bounded18/78 forward displacement shares existing79 offset calculation; native secondary radius and caster facing, without collision or visual certification.
## 2026-10-15 — Titan's Grip hand rules

[Inventory](systems/inventory.md#titans-grip-hand-rules--2026-10-15) derives effect155 capability and weapon/armor subclass masks from learned native spells; type17 gains the off hand with Dual Wield and matching Grip. Explicit/auto-equip retain the permitted two-hand pair. Published70391 authentication:261 spell fields plus pinned TC hashes. Request/persistence RED: three missing-admission/auto-equip failures; two negative controls pass. At9593985e native five tests GREEN under four threads (73.94s); merged7ff7b5e4 native ranged plus three retained Dual Wield controls bring scoped proof to nine pass/zero fail (7.62s). Workspace/all-target check72.481s/fmt4.779s pass warning-free. Acquisition, unlearn-time eviction and whole-Fury acceptance remain open.

## 2026-10-15 — Ordinary NPC physical-spell dodge

[Shared physical hit roll](systems/player-damage.md#shared-weapon-spell-defense-roll) consumes NPC3+aura49+positive effective-level bonus, allowing rear PvE dodge. Native70188/38598 genuine damage RED→4 native controls GREEN; actual tuning and separate nonlethal health buffer preserve guard proof. Block-only fixtures isolate dodge with source45769e1/251−20 and suitable caster level; repaired23 block/type/table controls pass. Final8fe095bc defense32/32 pass (95.39s tests); workspace/all-target check17.901s/fmt2.774s pass warning-free. Follow-up97592a1e native tuning/level chance assertions prove3/3/9/7.5 after49 removal across four native level cases; seven scoped controls pass64.20s tests. Sampled attack distribution, Totems, player expertise, full loadouts and independent acceptance remain unproved.

## 2026-10-14 — Signed attacker enemy dodge

[Existing player dodge consumer](systems/player-damage.md#shared-weapon-spell-defense-roll) now adds actual caster aura251's signed total before expertise/final clamp. Native48250e2+20/110294e0−25 are authenticated;24fb3cb5 native Strike damage37 despite verified90miss/+20 reproduces genuine RED. Merged18274ca5 native11/11 GREEN (17.48s tests), check16.844s/fmt2.490s pass warning-free:256 miss/dodge draws, removal/negative-total/recipient controls plus Evasion/expertise controls. Whole251/abilities and broader hit system remain open.

## 2026-10-14 — Physical mechanic resistance

[Native physical hit table](systems/player-damage.md#shared-weapon-spell-defense-roll) requires recipient aura117 after miss and before NoActiveDefense. Initial cd651cb7 full-table RED invalid: neutralNPC6 omits baseline5, actual85+10. Corrected hostileNPC40/pre117 consumer at2c4a7ecb records genuine native1833 stun RED atdraw10; foreign and silence-selector controls pass. Implementation follows corrected RED. Corrected c31715af native3/3 GREEN:256 miss/resist draws, no stun/control, removal/ownership/mismatch controls; retain27 source-identical defense controls. Workspace/all-target check130.216s/fmt4.316s pass warning-free; independent/full117/magic/ability acceptance open.

## 2026-10-14 — Non-player expertise dodge consumer

[Native player dodge boundary](systems/player-damage.md#shared-weapon-spell-defense-roll) subtracts the non-player caster's own active240 total/4. Published70391 NPC Hit Buffs62627/Strike38598 are authenticated; the first alleged RED listened on an unrelated connection and is invalid. Receiver-corrected ae251a10 now records actual Strike `Miss(Dodge)` with caster expertise10000: genuine RED1/foreign-controlPASS1. Native caster240/4 implementation follows that RED; current merged31 controls/check/fmt pass, including native removal/foreign ownership and all four cw150 cases. Independent acceptance remains pending. Player expertise, NPC dodge/parry and global hit certification remain gaps.

## 2026-10-14 — Live player weapon-spell dodge

[Live player field](systems/player-damage.md#shared-weapon-spell-defense-roll) feeds melee dodge before block. Native Rogue Evasion/receiver stat producer gives genuine Slam301 RED then six GREEN controls for direction/stun/flags/ranged/removal/expiry. NPC dodge, attacker modifiers, full probabilities and independent acceptance remain gaps.

## 2026-10-14 — Wildfire primary variance master red

[Native variance fixture](systems/spell-casting.md#wildfire-primary-variance-fixture) re-sources coefficient/primary bonus without widening tolerance or changing runtime math. Actual292 upper draw reproduces the stale interval;256 primary draws plus owned six-tick/expiry and AP37/RAP100 controls pass within merged76 tests/check/fmt. Cone/secondary and whole-class certification remain gaps.

## 2026-10-14 — Shared weapon-spell defense roll

[Implemented physical defenses](systems/player-damage.md#shared-weapon-spell-defense-roll) share one quantized cumulative roll. Native Sling Mud/Improved Blocking produces genuine unblocked-hit RED;60 Slam and60 Aimed Shot/Veil trials now pass with retained controls (19 total). 45 adjacent controls/check/fmt also pass. Missing dodge/parry, mechanic resistance, player baseline and independent acceptance remain explicit.

## 2026-10-14 — Ordinary NPC weapon-spell blocks

[Native block consumers](systems/player-damage.md#native-player-criticalblock-hit-state) preserve authenticated NO_BLOCK16, distinct from NO_XP64, and native30%/critical60% damage. Genuine flag RED→GREEN15 controls includes Ice Tomb36980/Gorilla1511 plus retained player11;45 adjacent controls/check/fmt also pass. Independent acceptance remains pending. Source Totem/Lightwell spawning and full cumulative probability order stay explicit gaps.
## 2026-10-14 — Full native pet-family grants

[Twenty real battles](systems/dungeon-achievements.md#full-family-earned-level-proof) at33782a95 pass1/1(37.10s), proving all0–9 families at10/25, actual6609/6610 grants only after every family, original dates, real Repair/persistent injuries, consumed targets, DB reopen/auth login and account43 isolation. No HP/XP/progress/earned-ID overrides after baseline. Source state161 guard stays intact; optional unsupported Extra Plating was not used. Predicate/census unchanged; acquisition/world population/rendered UI unclaimed.

## 2026-10-14 — Pet-level type qualifiers, native proof

[BattlePetType78](systems/dungeon-achievements.md#pet-level-type-qualifiers) uses already-preserved actual earned-level species and joined PetTypeEnum0–9; no new event/schema. Valid native RED3959ea9c and initial GREEN0032d528 prove actual39Mechanical9/1426Magic5 XP24→25. Compiler39/39/generated5736; census5494ordinary/1824statistics adds only6609/6610 structurally. Integrated native/catalog/prior91/pinned6a52bb57 passes4/4(21.37s); the subsequent twenty-battle follow-up proves all ten types/10-level/25-level grants; acquisition/rendered UI remain unclaimed.

## 2026-10-14 — Post-count7 source audit

[Remaining boundaries](systems/dungeon-achievements.md#remaining-source-only-boundaries) pins44/58/64's missing progress rules,153's actual action-set dispatch with zero matching native portals/templates, and218's hard-false Party Sync companions. Follow-up208/222/229/230/234 likewise lacks executable progress/callers; existing scenario/item/spell/Mythic events cannot stand in. Source/data proof only; no runtime/census activation. Removed stale spec assertions that already-implemented BG70/objective30/rated37 had no producers; world honor and garrison gaps remain.

## 2026-10-14 — Raw StartedAtLeast statistic projection

[Operator7](systems/dungeon-achievements.md#started-child-statistic-projection) now shares the existing direct-positive-child count without ordinary target clamping. Genuine three-kill REDa2fb7631 reaches107sum3/1336max2 but1337Nonevs2. GREEN2325ef39 passes2/2(25.37s): native zero→one→two→repeat-type behavior, existing raw-statistic regression and persistence controls. Census/qualification unchanged; no rendered UI claim.

## 2026-10-14 — Native creature-type kill statistics

[CreatureType30](systems/dungeon-achievements.md#creature-type-kill-statistics) preserves actual killed NPC template type for existing eligible78 recipients. Valid native RED8e2fe0df and GREENae25b634 use normal source40/299 spawns and lethal686 casts, with typed/wrong/refused/dead/idle/account/reopen controls. Native3198504f proves107sum2/1336max1 and pinned ordinary5492; its initial1337 scalar gap is closed by the operator7 follow-up above. Compiler38/38, census5492ordinary/1824statistics adds only107/1336/1337 structurally; no grant/acquisition/rendered-UI claim.

## 2026-10-15 — Minion ownership flag composition

[Source ownership](systems/toys.md#spawn-group-guardian-count-boundary-2026-10-14): native minion/player-owner rule replaces category-only player-control flags; companion immunity augments rather than overwrites ownership. Combat-pet designation, counts and captured privacy remain separate.

## 2026-10-14 — Spawn-group guardian count

[Scarab2301 source boundary](systems/toys.md#spawn-group-guardian-count-boundary-2026-10-14): preserve flag-directed default-title guardian routing and source count/scatter without inventing attacks or a pet frame; native visuals and other count routes remain open.

## 2026-10-14 — Initial random summon destination

[CalcRadius boundary](systems/toys.md#initial-random-destination-radius-2026-10-14): native72/86 require random radius as well as direction; preserve91 and separate secondary scatter. Flute follow and broader radius modifiers remain open.

## 2026-10-13 — Startup-owned generated Toy casting data

[Runtime data](../toy-runtime-data.md): removed production area/facing JSON from server source; current canonical export build is mandatory producer input. Generator4/4, loader3/3 and four private process cases verify missing-file refusal and two builds without binary rebuild. Historical69-area and single-radius test fixtures stay test-only. DB2 apply/deploy commands coordinated through lead-inbox-db2.md; canonical data remains lead-owned.

## 2026-10-13 — Clean-master coin dispatch recheck

[Coin ordering](../toy-effects14-coverage.md#coin-dispatch-regression-recheck-2026-10-13): reproduced missing emote on c732ab79, traced targetless dispatch to3ca41770, confirmed existing709082c0 fix after master merge (90839e14,1/1PASS). No duplicate runtime fix or native/transport completion claim.

## 2026-10-13 — Prismatic Bauble source and native gaps

[Coverage](../toy-effects14-coverage.md#prismatic-bauble--source-admission-native-gaps-open-2026-10-13): exact20s/current120s admission, real-source ribbon parser failure/fix, stationary Modern state proof and explicit Forever delivery/native-rendering gaps.

## 2026-10-13 — Source Toy facing

[Coverage](../toy-effects14-coverage.md#source-toy-facing--bounded-gate-2026-10-13): explicit unit180° arc, strict exact3D target-radius exemption and completion refusal before item recovery. Bounded gate and native proof status remain claim-specific.

## 2026-10-13 — Source Toy area-group lifecycle

[Coverage](../toy-effects14-coverage.md#source-requiredareasid-lifecycle-2026-10-13): real-area guards, source ancestry, cast completion and interrupted owned-aura removal; category preservation and both native skins. Whole1166 goal remains open.

## 2026-10-13 — Current PvP faction and War Banners

[Coverage](../toy-effects14-coverage.md#current-pvp-faction-and-war-banners-2026-10-13): source team scalar, actual BG/arena entry/return and refusal, UDP-only field changes,54 visual contexts and four native physical-combat runs. Whole1166 scope remains open; no independent-review PASS.
## 2026-10-15 — Wailing targeted Say

[Source engagement speech](systems/boss-scripts.md#wailing-targeted-aggro-say--2026-10-15) adds30% group10 with Player/Npc `$n`, preserving nearby Say listeners. Corrected host-boundary RED→5/5 GREEN79.17s/check18.26s/fmt ond6aa7d3e includes source schedule-before-roll order, unchanged combat and real wake/flight exit controls. No protocol/data mutation or natural/native-clear claim; lead independent gate pending.

## 2026-10-14 — Wailing later ritual waves

[Events17–20](systems/boss-scripts.md#wailing-later-ritual-waves--2026-10-14) add same-copy Naralex emotes and timed group3/group4 summons (seven Ectoplasms, Mutanus). Source combat-paused15/25/20/30s sequence passes3/3 in69.88s with first-wave/notification regressions; check/fmt pass. Controlled deaths, runtimebase stats and prior arrival are not ordinary fights or native clears; wake-up/exit21+ remain open.

## 2026-10-14 — Wailing first awakening wave

[Source events14–16](systems/boss-scripts.md#wailing-first-awakening-wave--2026-10-14) add1s Talk7/+5s self6271/Talk8/+4s three Moccasins. Actual aura/birth coordinates/target/copy/timers pass after retaining the runtime template in the fixture (not changing summons); merged25/25 gate184.24s/check/fmt pass. Normal difficulty stats, later awakening and native clears remain open.

## 2026-10-14 — Wailing chamber continuation

[Source events12–13](systems/boss-scripts.md#wailing-chamber-approach--2026-10-14) add1s speech and +5s path367803 launch. Controlled prior arrival then actual78.6692yd movement passes floor/budget/corridor/LOS and timing/copy checks; strengthened upper-bound proof and merged24/24 gate206.06s, check/fmt pass. Awakening/natural activation/native clears remain open.

## 2026-10-14 — C++ waypoint motion ownership and physical escort

[Ownership/support proof](systems/navigation.md#current-controlled-physical-proof): source default/active preemption, fresh-goal diff0, real-position resume, callback ownership and exact moving-foot support pass23/23 in134.17s; check/fmt pass. Controlled Wailing traversal reaches both camps and stopped endpoint (477.2578yd, waves[2,3], real6270). Natural activation, balance and native clears remain open.
## 2026-10-14 — Native node-ownership expression credit

[Node67](systems/dungeon-achievements.md#node-owned-world-state-expressions) preserves exact8656/5717 bytecode and accepted native ledger authority. RED4f69590f and GREENc0a6f87f prove real capture/win14943 credit; twelve genuine matches earn5253 at ten wins, preserve award dates/reopened counters, reject two-base wins while unconditional16513 credits, and leave timed14944 disabled. Four targeted native/catalog/regression tests pass; compiler37/37 and decoder3/3. Census5492 adds only5253; generic expressions, honor and acquisition/UI remain excluded.

## 2026-10-14 — Alterac captain source diagnosis

[AV25 investigation](investigations/alterac-captain-combat.md) distinguishes current175xHP/35xdamage from the legacy25xHP row, quantifies ordinary melee before mitigation, and records the driver's unused known abilities. Missing captain AI and incomplete fatal-hit attribution remain explicit; no gameplay/stat changes or victory claim.

## 2026-10-14 — Source Seaforium deployment RED

[Timed lifecycle](systems/game-objects.md#source-timed-seaforium-lifecycle--active-2026-10-14) begins with actual owned46847 UseItem rejecting custom-script195235, while normal pickup remains passing. Commit e06cb017 reads source charges/spell, owns the ten-second one-shot GO cast and reads owner reaction without rewriting raw faction0. GREEN/disarm/new native activation remain pending; compile-width failure is separate from the genuine behavioral RED.

## 2026-10-14 — Native IoC pickup retake

[Retakes78/79](systems/game-objects.md#authored-goober-spell-dispatch--2026-10-14) prove actual normal pickup+return with both skins after private-only receipt/cache reconciliation. All eighteen new captures inspected; local textured model/portrait and source bomb/bags visible, animation/151 meshes and no prior duplicate/v5 errors. Peer particle art, timing and origin framing remain open; no deployment/victory claim. Admission77 failed before native startup and is not render evidence.

## 2026-10-14 — Native IoC pickup, bounded visual result

[Goober pickup](systems/game-objects.md#authored-goober-spell-dispatch--2026-10-14) completes normal Workshop capture,195237 use and46847 count+1 in Modern75/Forever76, then normal return/save. Inspected eighteen captures show banners/bomb/bags, but private duplicate receipts and missingv5 cache prevent player/portrait acceptance. Private-only reconciliation is prepared; retake pending. Both pressure peaks exclude timing certification; no deployment/victory claim.

## 2026-10-14 — Matched-protocol goober adjacency

[Goober dispatch](systems/game-objects.md#authored-goober-spell-dispatch--2026-10-14) passes source Workshop pickup1/1 on server22ba4903/protocol492ee28. Object fixtures now supply the native45490 summon dependencies rather than skip authored effects; adjacent quest objectives7/7 and scoped binary check pass, assertions unchanged. Historical compile/fixture failures remain separate; no deployed/native pickup or timed-siege acceptance.

## 2026-10-14 — Authored goober spell dispatch

[Goober dispatch](systems/game-objects.md#authored-goober-spell-dispatch--2026-10-14) retains Data10/23 and source caster selection after normal use validation. Persistent-owner/free-space source Workshop fixture RED6 reaches owned46847 count0 versus1; implementation34d32065 committed before corrected GREEN1/1 (26.43s), with helper-resolved leased protocol/transport provenance. Direct-Cargo API-drift compile failure is retained separately. Native render, timed traps and siege victory remain separate gates.
## 2026-10-14 — Pet-level species compiler

[Species91](systems/dungeon-achievements.md#pet-level-species-qualifiers) compilerb6aaad84 qualifies sole160/162 species with authored level1–25;5/5 compiler tests and149 generated predicates. Runtime693218ca preserves actual species/level; native/catalog90220bb0 GREEN2/2(11.08s) proves real9070 grant, negatives and reopened counters. Actual census5491/1821 adds only account9070; all243 rows/pinned membership reconcile. Initial fixture used legacy XP;34411d29 switches to imported1979/1980. Qualified162 positive is catalog-only because1974–1977 lack combat states. Required workspace four-thread filter182/0/1ignored(255.20s), site-export1/1, server/engine checks and three-repo fmt pass without warnings at9610ef81 source scope. No capture/win species, native-qualified162, acquisition/UI or independent-provider claim.

## 2026-10-14 — Honor-family and legacy challenge source audit

[Source boundary](systems/dungeon-achievements.md#honor-family-source-boundary-verified-2026-10-14) traces31/35/52/53/113 through actual pinned callers: creature-only tappers and BG XP's honor-skip flag leave player-victim count dispatch unestablished. Scoreboard/type70 and wallet are not substitutes.19/22/71 have explicit no-op evaluators;204 lacks executable progress. Follow-up38/132/207/213 source rules remain unimplemented (38 explicitly rejects);194 is one per HonorLevel transition, not level or wallet, and195 lacks a prestige caller. Pinned8800 HonorXP threshold is placeholder logic, not current-build certification. No behavior or census change; native honor/content proof remains open.

## 2026-10-14 — Disciple second escort camp

[Second camp](systems/boss-scripts.md#wailing-second-escort-camp-2026-10-14) ports events8–11 without a cast-state guard or replacement buff. RED→GREEN proves real30s6270, three source Adders appearing at18s while casting, combat pause and12s resumed speech. Exact birth positions are observed before chase; physical onward path/native clear remain open.

## 2026-10-14 — Disciple first escort camp

[First camp](systems/boss-scripts.md#wailing-first-escort-camp-2026-10-14) ports path367800-end events4–7. Controlled arrival RED→GREEN proves authored speech/two Ravagers, actual combat pause and source-facing callback; existing Sleep/Potion remains passing. Physical traversal, activation data and native clear remain open.

## 2026-10-14 — Silverlaine primary identity
## 2026-10-14 — Derived factions survive DB2 replacement

[DB2 refresh](systems/content-import.md#db2-refresh-preserves-derived-factions) restores six Zephras factions from existing authored receipts atomically; preserves124 build receipts. Private import changes only faction_template+6/db2_row_source+6, integrity OK; native reaction test and9 importer/Skyborne tests pass. Old-source audit118/120; AreaTable/OverrideSpellData explicitly unavailable. Canonical untouched.

## 2026-10-14 — Active installed achievement import

[Importer](systems/dungeon-achievements.md#active-installed-achievement-import) no longer selects69933. Actual70391 process RED→GREEN2/2 covers current row counts, source build, Stockade and unrelated SQL; corrupt actual layout rejects before mutation. Build-selection/native-golden unit3/3; work-copy fingerprint/swap and native census gate pending.

## 2026-10-14 — Player-anchored spell-hit target class

[TargetIsPlayer5/TargetClass28](systems/dungeon-achievements.md#player-anchored-spell-hit-target-class) use the actual Player hit snapshot, with a complete positive player/race anchor for class predicates. Compiler32/32; native/catalog2/2 at b2b90543 include actual rose/turkey hits, wrong identity, missing authority and idle/reopen. Census5456/1816 adds1699/3559 structurally, excluding13 hypothetical unanchored class orphans. Mandatory acceptance pending.

## 2026-10-14 — Silverlaine primary identity

[Silverlaine](systems/boss-scripts.md#silverlaine-source-primary) adds pinned header/instance plus current canonical Journal/SmartAI validation to the copy-only producer. RED→GREEN and private120→121 application preserve unrelated/model data; no other Shadowfang boss is admitted. A real actor/spell fixture separately proves two victim casts/aura isolation and controlled-death1070/bit1 copy isolation; staged corpse→inventory once and Normal redb reopen/population/fresh-owner boundaries now have separate behavioral proof. Full fights and native acceptance remain open.
## 2026-10-14 — Actual spell-hit target race

[TargetRace27](systems/dungeon-achievements.md#actual-spell-hit-target-race) carries each hit player's race only for type110. Compiler31/31 and native1/1 at cc546863 prove delayed44212 Human/Orc hits, caster isolation, NPC/missing/depth/idle/reopen; census adds only account291 structurally. Required full four-thread workspace filter171/0/1ignored plus site-export1/1 passes at2e6a8661; server/engine checks and three-repo fmt pass. No full291/content/UI certificate.

## 2026-10-13 — Battle-pet team minimum level

[BattlePetTeamLevel34](systems/dungeon-achievements.md#battle-pet-team-minimum-level) reads all three equipped owned journal slots. Compiler30/30; native level25-team win RED lacks23257. Native/catalog3/3 proves matched-team win/lower equipped slot, numeric8278, idle/reopen and missing journal/slot qualification. Statistic8278 only; required full170/0/1ignored plus site-export1/1, server/engine checks and three-repo fmt pass at4531ce2b code scope.

## 2026-10-13 — Encounter loot eligibility ordering

[Type97](systems/dungeon-achievements.md#encounter-loot-eligibility-2026-10-13) reads existing active lock bits before native DONE saves them. Actual Skorpyron Normal14 death RED commits bit1 but lacks35448; native2/2 and required full167/0/1ignored plus checks/fmt pass at fa181cea code scope. No new lock state or bag-loot substitute; current roots remain unchanged.

## 2026-10-13 — Positive known-spell marker

[Known-spell qualifier](systems/dungeon-achievements.md#known-spell-qualifier-2026-10-13) reads existing class/profession/owned-mount IDs, not global spell metadata. Compiler RED→GREEN29/29; native/catalog2/2 prove all26 learned recipe facts and three account grants, absent/wrong knowledge and reopened persistence. Fixture runs standard ClassProgression PostUpdate publication before the next Update credit. Full four-thread workspace filter165/0/1ignored and server/engine/three-repo checks pass; no trait/public-UI marker acquisition claim.

## 2026-10-13 — Ground C++ waypoint arrivals

[Ground paths](systems/boss-scripts.md#ground-c-waypoint-paths) now use sourced baked routes rather than reject all non-exact ground MovePath calls. Real AT7904/Adarogg reaches the three authored paths and both meal-condition branches; empty-bake rejection and Golden Serpent flying-path controls accompany the walking-gait RED→GREEN. Native clear and both-skin loot remain open.

## 2026-10-13 — Ragefire copy persistence and corpse rewards

[Whole-copy boundaries](systems/boss-scripts.md#ragefire-whole-copy-proof) now exercise four real primary deaths, Normal redb reopen/dead-boss population and fresh-owner isolation; actual corpse loot requests persist each boss reward once. Native staged entry rendered but encountered missing source LOS eye receipts; no complete-fight or both-skin certificate follows.

## 2026-10-13 — Slagmaw logical identity versus preview

[Pinned primary identity](systems/boss-scripts.md#slagmaw-source-primary) authenticates61463→1445/bit2 independently of Journal preview38391 versus live model42247. Producer/census RED→GREEN and actual idempotent private SQL preserve both models/unrelated bindings; publication and full-fight/native gates stay separate.
## 2026-10-13 — Native autoattack range
## 2026-10-13 — Generic item receipt counters

[Item receipts](systems/dungeon-achievements.md#generic-item-receipts-2026-10-13) route90/91 from existing committed Retail queues, one event rather than stack quantity. ItemId19 and canonical quality14/15 preserve all conditions and complete alternatives. Native paid33999 RED reproduced missing epic counter6142; five targeted native/catalog scopes pass, including real stored quantities and one-event semantics, canonical/display-quality distinction and reopened receipt counters. Full four-thread workspace filter163/0/1ignored and server/engine/three-repo checks pass. Ordinary91 completion contradictions remain excluded.

## 2026-10-13 — Native survived falling

[Fall producer](systems/dungeon-achievements.md#survived-fall-producer-2026-10-13) preserves source Alive16/Area17, rejects unknown world-state and uses TC PROGRESS_SET distance×100. Native real-ground damage and Pandaren racial survival prove progress/grant/reopen; native/catalog6/6 also prove knockback, safe/feather/fly/lethal/immunity/safe-fall rejection and retained area predicates; full four-thread workspace filter159/0/1ignored and server/engine/three-repo checks pass. Puddle8354 actual map1105 completion/grant is not certified. Existing quadratic damage and complete environmental absorption parity remain separate gaps.
## 2026-10-13 — Native autoattack range

[Attacker range153](systems/auras.md#autoattack-range) now affects real main/off-hand swings, preserving signed addition, inclusive boundaries and unchanged weapon-spell ranges. Native Cat/Astral zero fields are not converted into invented positive bindings; whole13/40 remains open.

## 2026-10-13 — Current personal arena ratings

[Personal-rating39](systems/dungeon-achievements.md#current-personal-arena-ratings) maps current authored0/1 to native2v2/3v3, uses actual durable after-results and highest counters. Native full-party/accepted/prepared lethal matches prove1550 crossing, losers, second-loss maxima, replay and reopen. Raw Statistics replies retain the rating; Criteria replies retain bounded target progress. Ten character definitions and two statistics are structurally enabled; other brackets and proprietary Retail MMR remain unclaimed.

## 2026-10-13 — Wailing native summon groups

[Authored group spawning](systems/boss-scripts.md#wailing-native-summon-groups) now serves both C++ ports and SmartAI. Source Disciple targeting and real Mutanus group4 placement/credit/corpse lifetime are covered beside Wizzle storage/expiry preservation; this does not certify escort activation or scheduling.

## 2026-10-13 — Native damage shields

[Damage shields](systems/auras.md#damage-shields) add the source melee retaliation path missing from Anacondra's real8148 aura. Native percentage/power attribution, immunity/absorb and non-refreshing leash controls accompany real melee/dispel/death-ordering fixtures. Source spell2 is not substituted; final merged gate recorded separately in encounter evidence.
## 2026-10-13 — Exact map0 criterion

[Known zero map](systems/dungeon-achievements.md#exact-zero-map) preserves actual map0 versus missing authority. Native killing-blow5512 and redb reopen add existing World Killing Blows statistic1488, not an ordinary grant, world honor or relinked tree. Saved source root592/leaf593 remains authoritative until the coordinated DB2 re-export; no extraction performed.
## 2026-10-13 — Native damage shields

[Damage shields](systems/auras.md#damage-shields) add the source melee retaliation path missing from Anacondra's real8148 aura. Native percentage/power attribution, immunity/absorb and non-refreshing leash controls accompany real melee/dispel/death-ordering fixtures. Source spell2 is not substituted; final merged gate recorded separately in encounter evidence.

## 2026-10-13 — NPC Felguard learned interrupt

[Follower data](investigations/follower-data.md#npc-felguard-utility-source--2026-10-13): native level17 learned Axe Toss89766→347008 interrupts actual Fireball without canceling the owner's Shadow Bolt; below-level/cooldown/non-interruptible/stunned controls pass5/5 at72e754d5. Authenticated catalog is installed after generic fixture startup; native missile landing and existing persistence baseline are observed, not guessed. Ten pet motion/hazard/lifetime/credit cases additionally pass af677305, including real overlapping fields and movement CC. Current prepared follower73/73 and player-pet persistence10/10 pass; current server checks/fmt pass with later unrelated master changes. Full-dungeon/authored-kit/stat-budget acceptance remains open.

## 2026-10-13 — Follower utilities and strict-nav boundaries

[Follower data](investigations/follower-data.md#advanced-bridge-utilities-and-bound-fields--2026-10-13): real interrupts, casting-time readiness/cancellation, eleven-profile defensive mitigation/healing and real Stockade hostile-field escape; native Group Finder current-source Ruby admission does not close its disconnected authored route or full-dungeon acceptance. No source-coordinate changes, canonical writes or direct movement substitute.
## 2026-10-13 — PlayerHasAura Some alternatives

[Complete aura alternatives](systems/dungeon-achievements.md#player-aura-alternatives) read authenticated current player aura membership and reject one missing spell without dropping it. Compiler25/25; native Jaguar/Mighty Loa fixture GREEN2/2 proves independent source alternatives, unrelated/removed auras, depth refusal and reopened counters. Four additional ordinary definitions are structural, not boss/form-acquisition or encounter-completion certification.

## 2026-10-13 — PlayerIsInArea Some alternatives

[Complete area alternatives](systems/dungeon-achievements.md#player-area-alternatives) reuse recursive ancestry and retain every authenticated leaf. Five controlled cached authority snapshots drive real queued death109755/character61456 and reopened persistence; no terrain-travel or holiday/NPC content claim. Focused3/3, compiler24/24; four additional ordinary definitions are structural only.

## 2026-10-13 — PlayerMap Some alternatives

[Complete map alternatives](systems/dungeon-achievements.md#player-map-alternatives) retain every sourced leaf and reject a missing map at load. Native Warsong2106 lethal damage credits5538 once to its killer and survives reopen; source489 catalog evaluation is not legacy-map playability. Type56 is independent of accepted battleground reward70. Focused5/5, compiler23/23; census5430 ordinary/1812 statistics is structural only.

## 2026-10-13 — Target-creature Some alternatives

[Complete target alternatives](systems/dungeon-achievements.md#target-creature-alternatives--verified-2026-10-13) preserve one-of native request matching. Source-backed Love alternatives and six Wave selectors add seven structural account definitions; unknown qualifiers and NPC content closure remain explicit.

## 2026-10-13 — Single-child Some criterion semantics

[Some1-of-1](systems/dungeon-achievements.md#single-child-some-trees--verified-2026-10-13) preserves the supported child check. Native per-request target negatives and slime/wave credit add structural40475/1065; NPC spawn/model closure remains unclaimed.

## 2026-10-13 — Empty All criterion semantics

[Plain empty All4](systems/dungeon-achievements.md#empty-all-modifier-trees--verified-2026-10-13) matches TC's true result. Seven native goggle casts/store outcomes grant18905 in a sourced private fixture; missing canonical item projections and production/UI closure remain explicit.

## 2026-10-13 — Exact-level criterion facts

[Level39](systems/dungeon-achievements.md#exact-reference-player-levels--verified-2026-10-13) reuses inclusive minimum/maximum facts. Native level1/2/3 deaths and reopen prove raw orphan counters without linking them to current achievements; census unchanged.

## 2026-10-13 — Instance-death criteria capacity

[Type18](systems/dungeon-achievements.md#instance-death-capacity--verified-2026-10-13) uses allocated dungeon-copy capacity. Native party5/raid10/25 death, negative capacity/copy cases and reopened persistence pass; three statistics become structural, with no delve-content claim.

## 2026-10-13 — Known-solo achievement predicates

[Solo35](systems/dungeon-achievements.md#solo-group-predicates--verified-2026-10-13) requires native roster authority, not absence of visible peers. Actual marker casts and real joined groups prove credit/exclusion and reopened persistence; originating timed quest content remains unclaimed.

## 2026-10-13 — Achievement zone predicates

[Zone41](systems/dungeon-achievements.md#player-zone-predicates--verified-2026-10-13) uses one flagged parent, not ancestry. Native Stormwind pet win and reopened storage prove city credit; six account definitions become structurally evaluable without a world-honor or all-city-content claim.

## 2026-10-13 — Unrated Shuffle and authored cooldown reset

[Native arenas](../../../game-engine/docs/wiki/systems/arenas.md#unrated-solo-shuffle--verified-2026-10-13): six source-role solo entrants, real first-kill rounds, spec lock, source gate/start/reset restoration and character wins; both-skin live proof and four-map full-channel exact origins. Item overrides survive real cast-time completion and reset only the actual category; RED/GREEN fixtures preserve unrelated600s timers, charges/GCD/school locks and prove active-pet reset. Current Retail rated/leaver policy and dampening remain unclaimed.

## 2026-10-13 — Shared PvP reconnect and near teleport

[Persistence](systems/persistence.md#pvp-reconnect-verified-2026-10-13): durable captured entry points, exact reserved-copy rejoin and missing/expired-match origin restoration; reopened-redb and real disconnect/SelectCharacter tests pass. Shared Warsong mercenary restoration and last-living arena logout preserve behavior. Full-copy near teleport RED exposed dungeon-only lookup; reserved PvP-copy lookup fixes it. Native/UDP proof and completed32-case map/skin captures are in [arenas](../../../game-engine/docs/wiki/systems/arenas.md#pvp-reconnect--verified-2026-10-13).

## 2026-10-12 — Preparation visibility distinguishes unit targets

- 2026-10-13: Wailing four-druid DONE condition/Disciple action0 notification; real two-copy NPC/chat RED/GREEN. Three insufficient, fourth notifies only its copy, repeated DONE no duplicate; escort/awakening/combat unported. [Boss scripts](systems/boss-scripts.md).
- 2026-10-13: Wailing action34 mode1 resolves TC ordinal→encounter→runtime slot; actual Serpentis action-boundary RED/GREEN isolates it from generic death credit. Mode0 and missing-binding negatives preserve other states/copies. [Boss scripts](systems/boss-scripts.md).
- 2026-10-13: Deadmines pinned source has instance wiring only; exact six primaries authenticated, Oaf47297 no longer labeled Helix. Missing AI/spawns and difficulty-specific bit binding gaps retained. [Boss scripts](systems/boss-scripts.md).
- 2026-10-13: Slagmaw source port: Lava Spit/Submerge/server-side teleports, GetDistance boundary with source CombatReach24 (shared `model_reach.rs`). Dungeon-map spawn leash exempted per TC; open-world40yd boundary retained and tested. Historical Stockade39.888yd leash proof [superseded, evidence preserved](systems/navigation.md#stationary-arch-and-leash-acceptance). [Boss scripts](systems/boss-scripts.md).
- 2026-10-13: Gordoth ordinary-player AT7899 source rotated-box entry, same-copy lookup and instance-lifetime OnlyOnce; missed-boss entries remain retryable. GM/native/cage gaps retained. [Boss scripts](systems/boss-scripts.md#gordoth-combat-slice).

- 2026-10-13: Explicit Gordoth landing home/facing independent of respawn coordinates; home-return/leash honor it and respawn resets it. Real return proof,30s boundary test. [Boss scripts](systems/boss-scripts.md).

- 2026-10-13: Gordoth source DoAction jump/landing, vial state and researcher Bloody Suicide→51744 instakill. Real-population parabola/landing/self-death proof; home/static trigger/cage/animkit/native gaps retained. [Boss scripts](systems/boss-scripts.md).

- 2026-10-13: Gordoth already-started relocation/first casts/one-shot31% enrage/30s reset; source0x2 bypass with native category/cast-time and GCD/stun/mana proofs. Intro/repeats/native/map389 navigation remain gaps. [Boss scripts](systems/boss-scripts.md).

- 2026-10-13: Ragefire Koranthal source appearance/combat/death/evade;119973 dummy exposed missing destination-only effect32 dispatch, now forwards child destination. Three fixtures GREEN; native/credit publication/whole-instance gaps remain. [Boss scripts](systems/boss-scripts.md).

- 2026-10-13: Executed371652 additive source flags1/2/3, no real death and no cleanup hook. Raw flags2/3 protocol fields added; native pose delivery remains unproved. [Boss scripts](systems/boss-scripts.md).

- 2026-10-13: Storm Infusion395029 source-oriented(9,0,4) destination hook; actual area-trigger positions tested at two facings. Area actions/native visual behavior remain gaps. [Boss scripts](systems/boss-scripts.md).

- 2026-10-13: Ruby Life Pools Ice Shield/Excavate periodic source hooks, immediate initial +1s ticks. Two hook-boundary fixtures pass; SAI prerequisites and absent boss AIs remain gaps. [Boss scripts](systems/boss-scripts.md).

- 2026-10-13: Zul +1s activates pool289347 in its own copy; authoritative GO state and collision, with timing/copy isolation proof. Native state animation remains open. [Boss scripts](systems/boss-scripts.md).

- 2026-10-13: Minion of Zul appearance, Bound by Shadow self-death and Fixate128 preference/clear; different-caster channel coexist. Three real Kings' Rest fixtures pass. [Boss scripts](systems/boss-scripts.md).

[Typed invisibility](systems/auras.md#typed-invisibility-and-spell-area-eligibility-2026-10-11): real32727/type4 preparation exposed reciprocity incorrectly hiding game objects. Pinned TC Object.cpp:1003 excludes non-units; source-channel Cage/Interest reproduces the boundary while preserving unit detection and invisible-object checks.

# Wiki Log

-2026-10-19: Impending Victory202168→202166 then own32216 removal: five native qualifications4+1 first, pushed ignored69af7bb76→fiveRED/one control14.11s; minimal existing AfterCast/FULL-trigger/remove consumerf041d6b70→fresh ignored sixGREEN13.06s/0warnings before enabling; initial eleven-case composite then masterdfdd71578 merged d1af4b8a7→fresh targeted elevenGREEN14.64s/0warnings; lead gate pending. VICTORIOUS_KILL_PRODUCER_UNIMPLEMENTED/direct-native32216 removal-only boundary preserved. See [spell casting](systems/spell-casting.md#impending-victory-consequence--2026-10-19).
-2026-10-19: Invigorating Fury184364+383468→385174 qualified native payload/8vs11s/defense/recovery/actual Warrior cast preservation3PASS; pushed ignored788ff0e68→3RED/3controls; existing AfterCast/limited helper consumer4a5aaa555→fresh ignored6GREEN25.43s/0warnings; enabled after GREEN, retained3qualification forms nine-case composite; lead integration pending. See [spell casting](systems/spell-casting.md#invigorating-fury-consequence--2026-10-19).

- 2026-10-13 — [Recovery rates](systems/auras.md#cooldown-recovery-rates): native286 start/apply/remove/refresh/expiry, owning spell attribution for category-only timers, real Rapid Fire/Sprint/Hot Hand controls and current owner updates; full ability/global-time/native-rendered parity remains open.
- 2026-10-13 — [Map instance type](systems/dungeon-achievements.md#map-instance-type-verified-2026-10-13): exact authored122 map membership, native arena/BG killing-blow/reward/queued-death RED/GREEN and cross-type/persistence controls; +4 statistics, missing raid-death18 unclaimed.

- 2026-10-13 — [Flat versatility](systems/auras.md#flat-versatility-bonuses): source471 computed percentage and real damage/heal/taken/auto RED/GREEN; pinned69933 CASC rating/curve proof and lifecycle/flag/recipient controls. Owner inheritance/native-rendered parity remains open.

- 2026-10-13 — [Category cooldown offsets](systems/auras.md#category-cooldown-offsets): source341 additive milliseconds applied after haste/spellmods, actual25s→20s Counterspell RED/GREEN, category/learning/unlearn/both-timer/zero controls; no whole-role or rendered parity claim.
- 2026-10-13 — [General arena player predicates](systems/dungeon-achievements.md#general-arena-player-predicates-verified-2026-10-13): accepted/current-copy native24/60 facts for all gated events; real-channel56/17/70 rated statistics RED/GREEN, size/unrated/replay/reopen controls and redundant result fields removed. +5 statistics; current Shuffle unrated/source239/240 gaps explicit.

- 2026-10-13 — [Battleground kill rewards](systems/dungeon-achievements.md#battleground-kill-rewards-verified-2026-10-13): native lethal controlling-player/accepted-team distance and corpse credit, untimed orphan26004 RED/GREEN and persistence controls; four structural ordinary gains. Current-root grants, world/lifetime/scoreboard honor and rendered battle unclaimed.
- 2026-10-13 — [Category cooldown offsets](systems/auras.md#category-cooldown-offsets): source341 additive milliseconds applied after haste/spellmods, actual25s→20s Counterspell RED/GREEN, category/learning/unlearn/both-timer/zero controls; no whole-role or rendered parity claim.

- 2026-10-13 — [Penetrating Shots](../class-completeness6.md#batch44-penetrating-shots-ranged-critical-damage): native25%-of-owner-crit aura updater, real Aimed Shot RED/GREEN, live crit/unlearn and heal isolation; full-class/rendered parity remains open.
- 2026-10-13 — [Full-roster criterion limit](systems/dungeon-achievements.md#full-roster-limit-verified-2026-10-13): LessThanTappers29 uses full native group membership, not physical tappers; authored Malygos solo/8/9 roster RED/GREEN, offline-member and persistence controls. +8 ordinary; complete raw inventory counts reconciled. Full boss mechanics/rendering unclaimed.

- 2026-10-13 — [Rated arena map criteria](systems/dungeon-achievements.md#rated-map-wins-and-participation-verified-2026-10-13): committed WinArena32/ParticipateInArena33, qualified native team-size24/ranked60, real-channel2v2/3v3 RED/GREEN and persistence controls; +World Wide Winner699/+40 statistics.5v5/rating39 mapping remain disabled.

- 2026-10-13 — [Per-request emote targets](systems/dungeon-achievements.md#per-request-emote-targets-verified-2026-10-13): native target-entry snapshots, text-only Love225, emote-only TargetCreatureId4; real-channel RED/GREEN and persistence controls. Structural census gains five account achievements; spawn completeness and other target modifiers unclaimed.

- 2026-10-13 — [Conditional Gladiator banners](../toy-effects14-coverage.md#conditional-gladiator-back-banners-2026-10-13): existing race/sex PlayerConditions resolve both banner models; aura4 lifecycle/faction refusal, all104 visual contexts and representative both-skin native proof. Census354→356; War Banners remain blocked by CurrentPvpFaction.
- 2026-10-12 — [Placed statues](../toy-effects14-coverage.md#placed-statues-and-the-missing-template-audit-2026-10-12): 110 missing-template summon toys were audited. Ornate Dragon Statue and Stuffed Bear are imported; prepared census354→356. Attraction Sign (invisible weighted display) and the interaction toys stay blocked.
- 2026-10-12 — [Toy root scripts](../toy-effects14-coverage.md): Unusual Compass (ForcedFacing epoch), Titanium Seal coin, Goblin Weather Machine and Wormhole Generator: Pandaria are admitted only for their exact TC/TDB source shapes; census338→348. Transporter/Teleporting/Taunt Flag are named gaps.
- 2026-10-12 — [Source summon flags/privacy](systems/toys.md#source-summon-flag-import-2026-10-12): atomic3610-record flags import, real two-peer owner-only/public-control proof, wild spawn/target visibility fixes and preserved337 census; group-private remains explicit gap.

- 2026-10-12 — [Linked cooking traps](systems/toys.md#linked-cooking-traps-2026-10-12): source31442/7353 environmental activation, snapshotted versatility retention, seven actual-use identities and census330→337; representative native both-skin proof and lead-only asset publication recorded.
- 2026-10-12 — [Achievement criteria producer inventory](../achievement-criteria-producer-inventory.md): all243 raw types separated from current-root rows and unsupported actors/conditions; earned XP196/221/253,13 sourced class predicates, traditional ranks47/87/88 and real terrain163/164. Native account Dynamic Duo, authenticated45068 settlement, five terrain destinations and reopened persistence proved; current-build both-skin/integration gate remains open.
- 2026-10-12 — [Golden spawn gate](systems/boss-scripts.md#kings-rest-source-admission-2026-10-12): manual group1256 withheld at copy fill and spawned on the copy's last TempleEvent death; group1255 comes from the Serpentine Seal → Shadow of Zul `SerpentBossEvent` chain (script-set StringId), not yet ported.
- 2026-10-12 — [Golden intro paths](systems/boss-scripts.md#kings-rest-source-admission-2026-10-12): script `MovePath` ports the used WaypointMovementGenerator/Catmull-Rom subset; NotStarted Serpent flies TDB 13532200/01/02, drops emerge at the apex and lands hostile. Spawn-group trigger and fly/hover visuals still open.
- 2026-10-12 — [Golden partial TC combat](systems/boss-scripts.md#kings-rest-source-admission-2026-10-12): explicit lead timing authorization separates exact pinned TC combat events from unavailable Retail12.1 timeline inputs; stale alerts rejected, friendly intro remains closed, InitializeAI/FAIL/30s evade lifecycle ported. Moving Gold stop, authenticated model geometry and last-definition-wins Retail boss-mod extraction have focused proof. Full current union gate/native/remaining mechanics pending; no complete boss claimed.
- 2026-10-12 — [Portals, dual wield and parry](../class-completeness6.md#dual-wield-and-parry-knowledge--batch34): all29 class trans-door spells proved (census analyzer fix), off-hand weapons gated on native Dual Wield knowledge, Parry knowledge characterized; census3105/4540/182 (+113 working). Unlearn-time off-hand removal remains a gap.

- 2026-10-12 — [Fear control and armor percentage](../class-completeness6.md#fear-control-state-and-armor-percentage--batch32): native aura7 cast admission/swings/target/avoidance (flee movement pending direction decision) and aura101 armor TOTAL_PCT for players and creatures; census2992/4653/182, nine armor memberships now working.

- 2026-10-12 — [Creature-immunity definitions](../class-completeness6.md#native-creature-immunity-definitions--batch31): authentic235-row TDB importer, aura147 definition lookup, per-effect immunity miss logging; RED→GREEN plus403 adjacent passes. Census2983/4662/182, Bladestorm Arms/Fury now working; knockback effect lists and four absent TDB definitions remain gaps.

- 2026-10-12 — [Victim melee critical chance](../class-completeness6.md#victim-melee-critical-chance--batch30): native187 RED2→ten behavior controls, positive/negative/additive fields, actual attack type and expiry/removal; nine adjacent controls. Native form process evidence remains bounded by unverified starter art.
- 2026-10-13 — [Master arena/class regressions](investigations/master-arena-fixture-regressions.md): pinned TC target/cost/cap rules; duplicate Arcane discount removed, reciprocal Moloch success/refusal controls, unchanged mastery expiry assertions isolated from capped selection. Historical bisect limitations retained explicitly.

- 2026-10-12 — [Native casting restrictions](../class-completeness6.md#native-casting-restrictions--batch29): exact263 parent-family rule and exemptions, RED3→thirteen controls; [cooking-focus/spellcaster integration](../class-completeness6.md#cooking-focus-and-spellcaster-integration) preserves both SELECT payloads and twenty merged native behavior controls. Current integration gates/census tracked separately.

- 2026-10-12 — [Native form admission](../class-completeness6.md#native-form-admission--batch28): aura275 native mask bypass and distinct form failures; RED3→GREEN10. Census2981/4664/182 retains32 required-form prerequisites and improves13 memberships. Lead gate diagnoses preserve live-target, level-scaling and placement requirements.

- 2026-10-12 — [Receiver mechanic durations](../class-completeness2.md#receiver-mechanic-durations--batch27): native232/234 stronger-negative matching, integer AddPct, applied-effect masks and finite/harmful exemptions; RED5→twelve controls and35 adjacent controls. Positive-only Oppressing Roar and invalid selector1825 remain explicit source gaps.

- 2026-10-12 — [Masked moving casts/channels](../class-completeness2.md#masked-moving-casts-and-channels--batch26): native330 admission and dynamic CastState permission, family/recipient masks and native Attr13 veto; RED4→eight controls and22 adjacent controls. Full Hover, empowerment and client/live movement remain uncertified.

- 2026-10-12 — [Zero-amount linked auras](../class-completeness2.md#zero-amount-linked-auras--batch25): native284/285 child ownership/application/removal, stack clocks and trigger depth; RED3→seven controls,37 adjacent controls. Nonzero amounts remain named source gaps; no invented sign or magnitude conversion.

- 2026-10-12 — [Final armor factors](../class-completeness2.md#final-armor-factors--batch24): native466 final player multiplier, preserved fractional stat armor, native zero class fields; six arithmetic/damage/lifecycle controls and34 adjacent stat/armor/summon controls. Pet inheritance and non-player parity remain uncertified.

- 2026-10-12 — [Minimum-speed floors](../class-completeness2.md#minimum-speed-floors--batch23): native305 player speed floor after normalization/slows; RED2→six lifecycle/recipient controls and twelve adjacent movement/mount controls. Non-player template-relative rates and whole-dash parity remain uncertified.

- 2026-10-12 — [Representative class gate](../class-completeness2.md#representative-both-skin-class-gate--2026-10-12): all13 both-skin representatives and26 inspected PNGs, authentic private datasets/assets, no shared-realm changes. Full spell/talent goal remains2889 working-boundary/4756 partial/182 broken; no legal-build/rotation certificate.

- 2026-10-12 — [AoE avoidance](../class-completeness2.md#aoe-avoidance-shared-aura229): native Feint RED1000vs600, five school/stacking/expiry/sibling/periodic controls; batch16 census2878/4767/182 recovered, fresh private DB snapshot, Stealth source-row mismatch and Lycara mastery gap preserved.

- 2026-10-11 — [Chrono Flames continuation](../class-completeness2.md#chrono-flames-partial-mechanic-follow-up): native replacement/child-impact history, ownership/window/cap/healing/missing-record controls and explicit Retail timing limits; remaining class/talent/live acceptance stays open.

- 2026-10-11 — [Class completeness continuation](../class-completeness2.md): native ground/rectangle selectors, retail ritual metadata, controlled elemental lifecycle and persisted owned-item unlocks; batch12 receipts and source/threshold limits recorded. Whole13-class/40-spec and remaining-class live acceptance stay open.

- 2026-10-11 — [Level80 class completeness](systems/spell-casting.md#level80-class-completeness):7827 native memberships/all13/all40 build-union cast/passive receipts;2841 working-boundary/4607 partial/379 broken. Source-derived537 owner/label,457 category charge haste/progress,416 masked truncated cooldowns; signed Rune/Runic generation and full/partial-school miss cost boundaries. Real-row RED/GREEN, targeted resource controls, no full suite/DB write/master merge/UDP5000. [Contract](../specs/class-completeness.md) and [proof limits](../class-completeness.md) retain legal-build, missing-data and underivable mechanic gaps.
- 2026-10-11 — [Toy input/rendering continuation](systems/toys.md#toy-effects14-authentic-input-and-rendering-boundaries-2026-10-11): source items/costumes/kits/scenery, clicked ground points, real sight witnesses and focused-search recipient retention; private both-skin proofs and explicit320 prepared-candidate versus full-effect boundary. Source-scaled scenery live proof remains pending.
- 2026-10-12 — [Kings' Rest source admission](systems/boss-scripts.md#kings-rest-source-admission-2026-10-12): Golden primary-only SQL preparation from pinned instance/header/script and local journal/TDB; no companion credit or canonical mutation. Missing emerge/intro pipeline and mutable-damage ordering remain named gaps, not a whole-boss port.

- 2026-10-12 — [Murder Row mirror callbacks](systems/boss-scripts.md#murder-row-continuation-2026-10-12): source aura23 callback, random destination offset and wild creature-spawn lifecycle implemented after the inherited mirror test reproduced missing summons. Infusion remains a named source gap; no full-encounter or native acceptance claimed.

- 2026-10-12 — [Encounter coverage](../encounter-coverage.md): complete hosted identity census and source-related work order; partial Kystia/Nibbles port. Friendly phase generic-evade boundary reproduced; scripted victim-update ownership preserves the source's skipped UpdateVictim. No full boss/instance certification.

- 2026-10-12 — [Toy scaled/state-kit proof](systems/toys.md#toy-effects14-authentic-input-and-rendering-boundaries-2026-10-11): all four actual journal/native scaled identities pass both skins; Sealamp116424 and Spire236744 authored state models/attachment scales start once and disappear with natural180-second actor expiry.21 local assets authenticated;11 scoped core tests/native builds. Prepared320 is not complete-effect certification; remaining native/per-toy boundaries explicit.

- 2026-10-11 — [Toy input/rendering continuation](systems/toys.md#toy-effects14-authentic-input-and-rendering-boundaries-2026-10-11): source items/costumes/kits/scenery, clicked ground points, real sight witnesses and focused-search recipient retention; private both-skin proofs and explicit320 prepared-candidate versus full-effect boundary. Underlight Sealamp170476/display92605 actual journal/native two-mesh scale0.35/no PetFrame passes both skins on2026-10-12; all four scaled identities render, and Sealamp/Spire state-kit/expiry proof is recorded above; broader native/per-toy coverage remains separate.

- 2026-10-12 — [Mount Journal](systems/mounts.md#mount-journal): post-merge targeted mount/merchant/auction tests and both-skin native vendor and two-account AH acceptance; [contract](../specs/mount-journal.md) closed.
- 2026-10-12 — [Mount vendor acquisition](systems/mounts.md#vendor-acquisition): authenticated mount-only ExtendedCost, exact currency precision, atomic payment/delivery and ItemSparse reputation authority; [pinned data/gaps](../mount-vendor-costs.md). Main owns native/integration acceptance.

- 2026-10-11 — [Mount Journal](systems/mounts.md#mount-journal): read-only catalog, account item/favourite persistence and spell-backed requests; both auth-entry crash boundaries reproduced and fixed through one initialized Update step. Native/final acceptance remains tracked by the contract.
- 2026-10-11 — [Adventure Guide](../specs/adventure-guide.md): copy-only catalogs, signed priorities, current-month account contributions and query/tracking. Five scoped service tests pass; both-skin persisted Track and authored display-only TrainingGrounds852 rendered. Check/fmt pass; unsupported gates/criteria/month/reward bindings remain explicit.
- 2026-10-11 — [Zone population](../zone-population-counts.md):371 ranked zones/46 hosts, private234413 NPC/59863 GO/26770 quests;305 raw tables unchanged after fresh crafting/SmartAI snapshot. Existing importer adds missing quests/vendors/taxi models, retains source/overlays, replaces legacy placements; deploy ground includes GO-only maps. Private5916 both observed skins reward48677/82747/9961 and discover/fly2037→2034. Native client player-immunity bug fixed; all57718 hosted ground files present,44 maps unbaked, actor appearance/receipt and Stormshield dock boundaries explicit.24 inspected latest-data PNGs; no canonical write/UDP5000/full suite; independent model gate unavailable.
- 2026-10-11 — [Fixed-output crafting orders](systems/crafting-orders.md):2047-row copy-only export/import,89 fixed-output catalog rows, public/guild/personal escrow/claim/cast/mail/refund lifecycle and explicit two-sender mailbox migration. Targeted proof ledger in handoff; fee/duration/content/native gaps remain explicit.
- 2026-10-11 — [Boss-mod continuation](../bossmod-data-coverage.md): target5 root fix gives real Zevrim CastState/effects; lexical/guard/locale/custom-timer regressions17PASS, timeline7PASS, changed protocol1729PASS and check/fmt pass. Rendered addon warnings and both-skin private5906 Hydrospawn/Zevrim bars/warnings proved in eight inspected PNGs. DBM/EXBoss native input clocks remain explicit gaps; no fabricated timers or shared realm changes.

- 2026-10-11 — [Boss-mod runtime boundary](systems/bossmod-data.md#current-integration-boundary): sole-source copied catalogs,191 hosted bindings,343 literal LittleWigs schedules and central server CastStart/CastSuccess reconciliation. Importer5PASS, timeline6PASS/1FAIL, check and changed-file rustfmt pass. No native/PNG proof; warning text transport, dynamic raid timeline inputs and cast-start fixture observation remain open. Not ready.

- 2026-10-11 — [Boss-mod data import](systems/bossmod-data.md): non-executing Lua AST extraction, Retail branch selection, literal timers/health/phases/warnings, pinned Spell/SpellName validation and copy-only SQLite. Four behavioral tests pass; server reconciliation and private native proof remain open.

- 2026-10-11 — [Private terrain destinations](investigations/private-terrain-destinations.md): UDP5881 reproduced cooldown-assist/Erland authority falling before client tiles parse; stale setup Z rejected by unchanged step reach. Shared build-qualified private destination manifest/helper; actual server gravity RED on original coordinates. Live/final proof retained separately, no canonical writes or collision clamps.

- 2026-10-11 — [SmartAI final integration](../smartai-census.md#proof): code65dee4d9 keeps master follower leash exclusion/data-selected boss tracking; targeted53PASS/1pre-existingignored, importer2PASS and enginephysics14PASS; check/fmt clean. Rebuilt final binaries completed both native skins on explicit authentic legacy copy;12 new inspected PNGs. Mixed-Retail scaling and unsupported vocabulary remain open; independent verifier unavailable.

- 2026-10-11 — [SmartAI private live proof/census](../smartai-census.md): both-skin full GUI863 acceptance/path/group0/credit/choice/turn-in,12 inspected PNGs. Explicit authentic legacy source fixture; mixed Retail scaling remains unproved.24 action kinds41,696 rows,19 event kinds41,014 rows,8 target kinds50,628 rows are admissions, not per-row quest certification.

- 2026-10-11 — [SmartAI summon lifecycle](systems/boss-scripts.md#smartai-escort-coverage-2026-10-11): actual child initialization54, parent17 and timed-despawn35 dispatch; two lifecycle regressions fail before implementation. Generic summon boundary owns each callback once.

- 2026-10-11 — [SmartAI ranked actions](systems/boss-scripts.md#smartai-escort-coverage-2026-10-11): orientation66 and looping emote17 (1,998 total/1,116 quest-linked rows) follow pinned TC semantics; two real-row missing-action regressions. Timed chance/max-delay proofs exercise existing behavior; live escort remains separate.

- 2026-10-11 — [SmartAI motion authority](systems/boss-scripts.md#smartai-escort-coverage-2026-10-11): authentic Wizzle acceptance plus production fixed-schedule point/wander integration reproduces a second0.125-yard stride; active escort ownership excludes idle roaming. Client stall/home-return and both-skin live acceptance still pending.

- 2026-10-11 — SmartAI escort coverage: generic private overlay, timed-list combat gates/self-nesting policy and generic death/distance/waypoint lifecycle. Targeted proofs and remaining vocabulary/live work tracked in coverage handoff; [system](systems/boss-scripts.md#smartai-escort-coverage-2026-10-11).
- 2026-10-11 — [Armory refresh](systems/site-snapshots.md): user-selected default-public character policy minus ID opt-outs; exclusive mode0600 copy-only redb crash recovery; native release exporter and installed-but-disabled agent-server 15-minute user timer. Refresh validates before staging and uses atomic directory exchange, retaining data.old. Production-copy local dry-run succeeded with zero saved characters; existing real private-proof copy produced five characters against canonical read-only world.db. First publish/enable stays lead-owned. [Operator contract](../specs/site-snapshots.md#agent-server-refresh-job).
- 2026-10-11 — [Non-visual equipment](systems/inventory.md#non-visual-equipment--verified-2026-10-11): preserve visual/character bitcode, additive jewelry/trinket/relic storage, paired first-empty/lower-level replacement, authored unique/category rejection, source-qualified snapshots/stat input. Eight focused handler tests cover pairs/swaps/rejection/migration/reopen/class/ranged/stat-modifier boundaries; native both-skin paid Mood Ring/five slots/stat/tooltip/drag and exact restart/relog receipts. No full server suite/canonical DB/UDP5000 writes; current Mainline has no relic fixture or ranged button.

- 2026-10-11 — [Static armory snapshots](systems/site-snapshots.md): versioned JSON contract, read-only public allowlist/opt-out export, real fb-owned private fixtures, local PNG conversion, future M+ attribution and raid first-kill/guild credit. Reset-lock progress stays separate from post-deploy lifetime rankings; no invented score, season or historical timestamps. Source and exporter behavioral tests linked in site-data diagnostics; final integration gates recorded in handoff.

- 2026-10-11 — [Mythic Keystone runtime](systems/mythic-plus.md): supersedes the earlier catalog-only boundary with persistent bag GUIDs, authenticated Font activation, Mythic23→Keystone8, shared clock/weighted kill/death/encounter state and idempotent terminal key/history. Real connection character-switch authority regression; default-disabled numeric policy. Private5800 Modern8→11 and Forever8→7 accelerated-clock proof; inspected both-skin PNGs, admin Damage/Revive boundaries and visual/season/combat gaps explicit. No canonical world.db/5000/master mutation.

- 2026-10-11 — [Mythic+ catalog foundation](systems/mythic-plus.md): six local-CASC challenge tables, copy-only layout/hash receipts, atomic replacement and linked/canonical refusal; authentic5/5 behavioral tests. Qualified populated Azure Vault/Font/timer/scenario/weighted forces. Runtime inventory/Font/countdown/tracker/completion/rating and both-skin live proof remain unimplemented; no numeric rule fabricated or canonical DB mutated.
- 2026-10-11 — [Follower spec bridge](investigations/follower-data.md): user-approved player-spell workaround, eleven sourced profiles/authentic templates, ordinary executor/passives/costs and aura458324, companion identity/presence/health and native LFG transport. Role/catalog7/7, party loopback1/1, profile SQL2/2 and native2/2 bounded proofs; server check/fmt pass. Canonical population is now present, but exact205-tagged maps have no authored entrance and Ruby's entrance has no205 population. Full TDB/location audit retained; no invented remap, canonical writes, private5750 combat or PNG claim.

- 2026-10-11 — [Retail follower data](investigations/follower-data.md): pinned local CASC 27 difficulty205 entries, Ruby map2521/306 reference spawns; 37 Expedition Escort candidates have no TDB combat spell/SmartAI assignment. Prepared reference SQL and read-only audit; no canonical DB application, combat AI, queue, class-kit or both-skin fighting claim.
- 2026-10-11 — [Mythic Keystone runtime](systems/mythic-plus.md): supersedes the earlier catalog-only boundary with persistent bag GUIDs, authenticated Font activation, Mythic23→Keystone8, shared clock/weighted kill/death/encounter state and idempotent terminal key/history. Real connection character-switch authority regression; default-disabled numeric policy. Private5800 Modern8→11 and Forever8→7 accelerated-clock proof; inspected both-skin PNGs, admin Damage/Revive boundaries and visual/season/combat gaps explicit. No canonical world.db/5000/master mutation.

- 2026-10-11 — [Mythic+ catalog foundation](systems/mythic-plus.md): six local-CASC challenge tables, copy-only layout/hash receipts, atomic replacement and linked/canonical refusal; authentic5/5 behavioral tests. Qualified populated Azure Vault/Font/timer/scenario/weighted forces. Runtime inventory/Font/countdown/tracker/completion/rating and both-skin live proof remain unimplemented; no numeric rule fabricated or canonical DB mutated.
- 2026-10-11 — [Follower spec bridge](investigations/follower-data.md): user-approved player-spell workaround, eleven sourced profiles/authentic templates, ordinary executor/passives/costs and aura458324, companion identity/presence/health and native LFG transport. Role/catalog7/7, party loopback1/1, profile SQL2/2 and native2/2 bounded proofs; server check/fmt pass. Canonical population is now present, but exact205-tagged maps have no authored entrance and Ruby's entrance has no205 population. Full TDB/location audit retained; no invented remap, canonical writes, private5750 combat or PNG claim.

- 2026-10-11 — [Retail follower data](investigations/follower-data.md): pinned local CASC 27 difficulty205 entries, Ruby map2521/306 reference spawns; 37 Expedition Escort candidates have no TDB combat spell/SmartAI assignment. Prepared reference SQL and read-only audit; no canonical DB application, combat AI, queue, class-kit or both-skin fighting claim.
- 2026-10-11 — [DB2 instance coverage](../instance-coverage-counts.md): union231 dungeon/raid maps with authored difficulties, excluding garrison expansion; shared-base priority retains old124-map rows and Hogger source-backed loot. Targeted importer RED→GREEN7/7; private snapshot row diff zero. Entrance/ground census separates hostability, walk-in reachability and sparse-source gaps; no canonical DB writes.
- 2026-10-11 — [Open-world population](systems/content-import.md#open-world-population-2026-10-11):589-map/762-zone census;31 scoped terrain-backed expansion maps,12,427 NPC/2,424 GO additions on private copy; Python2/2, Rust admission/ECS2/2, unchanged raw/non-target rows and42-table final equivalence. Private UDP5820 native Wixler Brass pick in Boralus in both skins; PNGs inspected. Authored same-map overlays retained; source-empty/phase/event/appearance gaps explicit. No canonical write, master merge or UDP5000.

- 2026-10-11 — [Class spell batch23](systems/spell-casting.md#twenty-third-batch-authored-combo-builders-and-rune-weapon-parry): scripted197835 points consume121471 native100-percent modifier; actual49028/27893 spawn grants native81256 owner30-percent parry, lifetime/early removal controlled. Genuine per-target combo and missing-parry RED→targeted12/12 GREEN at449bb8e.2031 exact native rows, all28 previously underived roots reviewed and30 deferred revisited. [Census/limits](../spell-dummy23-coverage.md):157/129/28→157/157/0, all157 unresolved roots and missing mechanics retained; no no-op closure, full suite, DB write, realm/server/UDP5000 or master merge.

- 2026-10-11 — [Class spell batch21](systems/spell-casting.md#twenty-first-batch-native-combo-builders): Berserk s2 additional combo point on native Feral self-grants; actual-root Shred/Rake/Thrash, cap/overflow, expiry, periodic/unrelated/rejected exclusions. Targeted8/8 at1aef6ea,338 exact CSV/SQLite rows, local empower/variable receipts and all40 baseline reproduction. [Coverage](../spell-dummy21-coverage.md) records158/110/48→157/117/40 and seven exact source-field/runtime-prerequisite deferrals. Integratedea6a007/code1aef6ea requested spell_cast ONCE435/435, fmt/check exit0, warning-free. Main-thread proof only, no independent/full-server/live claim; shared44b2b0f pinned, later docs/proof only, no master merge/deploy/UDP5000.

- 2026-10-11 — [Class spell batch20](systems/spell-casting.md#twentieth-batch-authored-totals-and-next-spell-uses): Barbed Shot authored total over native seven-tick clock, same-owner unpaid rollover/Focus bridge, Stormkeeper two next-spell uses. Native9/9,214 exact CSV/SQLite rows, all40 baseline reproduced. [Coverage](../spell-dummy20-coverage.md) records160/104/56→158/110/48 and exact six deferred-field reasons. Equal Barbed tick distribution is a stated inference; Overload/overlap/live gaps retained. Verification pins local shared-protocol44b2b0f to exclude unrelated quest-wire drift. Code57deae4 requested spell_cast ONCE427/427, fmt/check exit0, warning-free; independent artifact verifier unavailable (expired OAuth), parent retains integration acceptance. Later docs/proof only; no master merge/deploy/UDP5000.
- 2026-10-11 — [Native Leatherworking proof](systems/professions.md#native-leatherworking-proof--verified2026-10-11): real trainer/cast/reagent/skill/persistence behavioral test and linked both-skin private client acceptance; no world.db writes or invented quality formulas.

- 2026-10-11 — [Class spell batch19](systems/spell-casting.md#nineteenth-batch-counts-healing-totals-and-owned-damage): eight spell/rank entries with real-store deterministic tests. Source-bound Flurry/Mutilate, Dispersion/Intangibility/Altered Form totals, actual dispeller Horror, owned Havoc copy and shared Ardent fatal-hit reservation. [Coverage](../spell-dummy19-coverage.md) records163/103/60→160/104/56,277 exact native rows, all40 baseline reproduction and feature/masterf5d60ca single spell_cast418/418, fmt/check passing. Unknown markers, destination-defense/live and source gaps remain; later edits docs/proof only.
- 2026-10-11 — [Toy-effects12 non-summon cosmetics](systems/toys.md#toy-effects12-non-summon-cosmetics-2026-10-11): highest eligible blockers aura4/35 and effect3/31; source-bound Heartbreaker, Death's Door Charm, B.B.F. Fist and Abyss Caller Horn. Behavioral RED5/5 rejects the actual missing cosmetic admissions. Source lifetimes and animation/model chains retained; no generic dummy implementation, gameplay water or canonical DB write. [Coverage](../toy-effects12-coverage.md) records the final proof boundary.
- 2026-10-11 — [Combat pet lifecycle](systems/combat-pet-lifecycle.md): V2 migration, authenticated205-position stable, health/name/spec/current durability, Revive/Mend, Call1–5 and six authored demons. Health/unsupported-demon/rename-chrome RED→GREEN; reopened persistence, Fel Domination charge/cast-time tests and both-skin native stable/healing/death/confirmed logout proof. [Demon rendering](systems/warlock-pets.md#expansion-proof-2026-10-11), Feed provenance and absent Incubus remain explicit boundaries; no world.db or shared-realm writes.

- 2026-10-11 — [Class spell batch18](systems/spell-casting.md#eighteenth-batch-native-leech-and-reapplication): eight bounded root entries, shared native effect9/aura53 leech, same-owner Madness rollover, fractional/accumulated Ignore Pain, improved Backdraft uses and Obliterate weapon branches. Native14/14 at edfa536,331 exact CSV/SQLite rows, all40 previous ID sets reproduced. Census164/100/64→163/103/60; one Ignore Pain Dummy closure, aura142 retained. [Coverage](../spell-dummy18-coverage.md) preserves unknown markers, Singularity area, Karma/guardian/Lifebloom threshold/Eye Beam/live gaps. Feature/master merge83c5fb3 integrates origin/master632f3ae account-achievement changes; requested single spell_cast404/404 includes all14 new functions, compiler warning-free; fmt/check pass. Later edits docs/proof only; no realm/full-rotation claim.
- 2026-10-11 — [Follower first-boss proof](investigations/follower-data.md#both-skin-first-boss-proof--2026-10-11): approved Normal population/entrance reuse, actual205 copy selection/group binding, native ownership/rewards and removal of unassigned458324. Private5750 Modern/Forever queue, five roles, following, tank threat, NPC healing, spell priorities, Juggernaut/Melidrussa kills and XP; eight inspected PNGs. Authenticated loot absent; native walk blocked, admin boss approach, full-instance/all11-spec coverage open. No canonical writes/master merge/UDP5000; targeted RED/GREEN plus check/fmt only.

- 2026-10-11 — [Instance population](systems/content-import.md#instance-population-2026-10-11): TDB copy-only importer and regeneration hook, same-ID authentic templates/models, source addons/formations/equipment and namespaced paths; remove runtime NPC cache-map gate and load model-bearing instance objects. Per-map census covers all124 maps; private three-dungeon entry/combat and missing verified receipts/display coverage recorded. Full both-skin visual acceptance blocked; canonical world.db, UDP5000 and master untouched.

- 2026-10-11 — [Achievement account/events slice2](systems/dungeon-achievements.md#account-aggregation-and-additional-criteria-slice2-2026-10-11): atomic account/character contribution migration, retained dates/deleted-character credit, isolated character dependencies, and types29/7/28. Structural union631→2584→2882→3151→3388;43/46 stay disabled without local overlay mapping/compatible Retail standing. Native cast/recipient and skill-change fixtures; durable evidence/handoff in `achievements2-2026-10-11` and `/home/osso/.worktrees/handoff-achievements2.md`. Server-only, no world.db writes.

- 2026-10-11 — [Class spell batch17](systems/spell-casting.md#seventeenth-batch-named-resources-and-restorative-hooks): eight native low-count roots;13/13 at76c4591 plus completion/cancellation1/1 at5f8ee8e;429 exact native/CSV rows and all40 baseline ID sets reproduced. Census163/99/64 ->164/100/64 exposes Blood Plague's latent markers rather than inventing a closure. [Coverage](../spell-dummy17-coverage.md) retains leech/refresh/talent/guardian/Karma/Eye Beam/live gaps. Feature/master mergeec9b5cc integrates unrelated origin/master0c73965 achievement/quest changes; requested single spell_cast389/389 includes all14 batch17 functions, compiler warning-free; fmt/check pass. Subsequent edits docs/proof only.
- 2026-10-11 — [Quest sharing, resets and timers](systems/quest-share-resets.md): redb timestamp/deadline migration, existing UTC realm reset calendar, periodic exclusive alternatives, revalidated one-shot party offers and persistent expiry. Reset/exclusive RED→GREEN, real-message party/relog tests and both-skin private proof; no world.db writes/master merge.
- 2026-10-11 — [Authoritative transmog](systems/transmog-apply.md):9d415a7 host behavioral10/10; authored service/gossip, atomic exact target-cost/modifier/binding transaction and cached-visual preservation. Private55087 native paid application, self/observer rebuilds and relog in both skins; same-redb server restart preserved modifiers. Dynamic prices/conditional source policy/illusion/outfit gaps remain explicit. No world.db writes/master merge/5000. Independent verifier unavailable (expired OAuth).
- 2026-10-11 — [Typed invisibility and spell-area eligibility](systems/auras.md#typed-invisibility-and-spell-area-eligibility-2026-10-11): boolean universal invisibility replaced by per-type observer values in interest/targeting. Authentic968-row TDB input imported only into private SQLite backup; safe incremental lead rollout documented. Jaga29401 real Modern+Forever loot/credit/reward and post-reward hiding captured/inspected on5496. No canonical world.db write, aura stripping, objective/item grant or master merge. Vin/Rendan eligibility routes remain data gaps; Core Fragment's current authored difficulty1 supersedes old ledger0, Outland530 remains unhosted.

- 2026-10-11 — [Quest GO/spell objectives](systems/game-objects.md#quest-objects-2026-10-11): validated goober Type2 credit, chest quest-item loot through existing requests, authored Lock.db2 input/export (no world.db writes), and generic personal/group spell effects90/134. Real-data quests11602/11049/27955/10566 pass request/turn-in tests; held-key refusal and native live proofs pending at this milestone. Corpse-only decay cleanup no longer deletes GO loot between requests. Gate order server → engine; no protocol payload change. Full GO lock-cast/condition/event/visual lifecycle and label/learn-spell objectives remain unproven.

- 2026-10-10 — [Achievement quest/dependency slice](systems/dungeon-achievements.md#quest-and-earned-achievement-criteria-slice-1-2026-10-10): types27/8, authored faction eligibility and correct0x20000 account exclusion. Character Dreadsteed reward/toast/relog, historical Hero of Shattrath dependencies, opposing/neutral catalog rejection and account-pending presentation. Corrected character census631 union/564 Alliance/559 Horde/492 common; old correct-mask union225. Native both-skin Loch quest26145 proof and account-pending recheck retained in `/home/osso/.worktrees/handoff-achievements.md`; account aggregation/migration queued for slice2. No world.db rows, protocol payloads or statistics UI.

- 2026-10-10 — [Class spell batch16](systems/spell-casting.md#sixteenth-batch-low-count-rank-and-charge-consumers): nine native low-count rank/charge/heal entries; targeted12/12 warning-free at9721cec;229 exact native/CSV rows and all previous40-spec ID sets reproduced. Census163/98/65 unchanged, no invented Dummy marker closure. [Coverage](../spell-dummy16-coverage.md) retains actor/Karma/Eye Beam gaps and corrected fixture RED boundaries; origin/master integrated at e0fa018; single requested spell_cast372/372 warning-free and fmt/check pass. Later master c644eaa adds three unrelated toy visual IDs; fresh156cd37 targeted12/12 and fmt/check pass, no repeated broad gate. Final edits docs/proof only.
- 2026-10-10 — [Toy-effects11 summon audit](systems/toys.md#toy-effects11-summon-classification-and-scenery-2026-10-10): exact43-toy runtime refusal/source matrix;32 missing templates,11 unsupported control/title categories, zero missing model metadata joins. Authentic sweeper/trophy templates plus trophy champion-animation aura: actual-use RED→GREEN2/2, importer RED→GREEN3/3, prepared128→130/1166 with zero removals. No blanket category admission or canonical DB write;41 summons remain blocked by underived interaction/controls/targets/scales.

- 2026-10-10 — [Toy-effects10 cosmetic auras](systems/toys.md#toy-effects10-cosmetic-target-and-growth-auras-2026-10-10): Iron Belly Spirits, Darkshard Fragment and Aqir Egg Cluster source-zero aura audit; real bag/use/target/trigger/lifetime RED4→GREEN4, prepared125→128/1166 (+3,zero removed). Source-timing fixture corrections keep Spirits1000ms cast/60000ms aura, Darkshard5000ms and Aqir15000ms. Initial post-master349/47 gates pass; subsequent source-changing master3953eb3 merged before push, requiring new-revision360/47 gates, format/check pass. No canonical DB write; concurrent source preparation observed. Native presentation and scripted/control/interaction groups remain outside this proof.
- 2026-10-10 — [Unskipped 300k auction query gate](systems/auction-house.md#unskipped-300k-query-gate-2026-10-10): reported failure not reproduced at5e858c83; four standalone runs/profile, auction96 passed/4 ignored and full unskipped workspace exit0 (server1918 passed/75 ignored). Debug49.104–121.410ms, release9.522–17.385ms; full-gate slowest127.897ms. Temporary phase profiling restored exactly; existing allocation/heap corrections already merged. No new code fix, threshold exemption or grouped/network SLA claim.

- 2026-10-10 — [Toy-effects9 banners/sign](systems/toys.md#toy-effects9-stationary-banners-and-sign-2026-10-10): source-complete effect28/content group,15 real banner items and16 scenery templates; imported-copy census109→125, no removals or shared DB write. Source tuples match local archive; item/template/model preservation, CLI idempotency and missing-item/model atomic rollback tests. Native rendering, scaled banners and scripted/control/interaction groups remain outside this proof.
- 2026-10-10 — [Class spell batch15](systems/spell-casting.md#fifteenth-batch-native-health-weapon-ranks-and-delayed-hits): eight bounded health/weapon/rank/owned-heal/delayed-hit consumers, targeted11/11 warning-free at1e719bf; single post-master spell_cast360/360 and fmt/check pass at6a56656. Frozen164/93/71 becomes163/98/65; Vampiric Blood closes, Sigil's descendant marker remains.238 native rows and previous40-spec ID sets match exactly. [Coverage](../spell-dummy15-coverage.md) records corrected deterministic RED and separate integration gates; Karma redirect, guardian inheritance and Eye Beam discrepancy retained.
- 2026-10-10 — [Pet effects3 lockdown](systems/pet-battle-engine.md#lockdown-continuation--petbattle-effects3): bounded Nevermore/Spell Gulp wearer-owned event9 and Mind Games one-based slot clocks; actual snapshot lockdown replaces hardcoded0. Real-ID RED/GREEN, pass/swap/miss/bench/continuation and immutable rejection fixtures; exact Retail callback timing, persistent Spores, OverrideIndex and special-stat combination remain unproven. No DB writes, realm process or master merge.

- 2026-10-10 — [Class spell batch14](systems/spell-casting.md#fourteenth-batch-owned-hits-and-native-clocks): eight low-count local hit/pet/periodic paths;166→164 Dummy roots,93 reviewed/71 underived,118 native row equivalence and fixed40-spec table. Single spell_cast gate347/349; two missing-native-child fixture failures corrected and bounded14/14 passes. Primary-only Eye Beam/Wildfire, native11-hit versus variable10 discrepancy, guardian gaps and proof limits explicit in [coverage](../spell-dummy14-coverage.md).

- 2026-10-10 — [Class spell batch13](systems/spell-casting.md#thirteenth-batch-authored-hits-ranks-and-channel-clocks): eight local contracts implemented after behavioral RED; targeted12/12 at2476ec8, later narrowed Bestial source contract/2 revised fixtures pass at33b965c. Unbound pet buff removed rather than choosing among reverse-description IDs. Census168→166,90 reviewed/76 underived; exact all40 baseline reproduced.151 source rows match local CSV. Post-master1e27532 single spell_cast325/337: twelve stale slot-data failures. Canonical data link repaired without DB writes; bounded24/24 covers every failure and all12 new cases. Format/check pass; no repeated broad gate/fresh337/337 claim. Guardian inheritance and wolf melee retained without guessed curves. Local DB2/read-only world.db/TDB1210 only.
- 2026-10-10 — [Toy-effects8 scenery](systems/toys.md#toy-effects8-stationary-scenery-preparation-2026-10-10): ranked effect28 first; prepare authentic incense65298/totem65387 templates/models, source20000/60000ms stationary summons. RED importer missing both templates; RED actual-use missing65298 after correcting missing-properties fixture. Actual bag/use/model/ownership/map/faction/stationarity/expiry and imported-copy census pass:107→109/1166 (+2,zero removals),shared DB107 unchanged. Post-master spell_cast325/325,toy41/41,importer2/2 incl idempotency and missing-model rollback; format/check pass. Source rows exactly match actual local archive; no shared DB write or rendered claim.

- 2026-10-10 — [Class spell batch12](systems/spell-casting.md#twelfth-batch-actor-resource-and-snapshot-consumers): eight local Mainline paths prioritize authentic wolf/Gargoyle actors and successful spend/snapshot consumers, then low-count Outlaw/Subtlety/Protection/Survival paths. Native resource scaling, Rake bleed snapshot, owned UA vulnerability, Shield cone/armor and talent Butchery bleed. Ten focused functions pass; inherited Rapture fixture corrected/focused1/1. Single post-origin/master spell_cast gate325/325 ata995632, warning-free; fmt/check pass, linked coverage/ledger. Census168 retained,88 reviewed/80 underived; Rake closes E3, Shield exposes latent child aura4.134 exact native/CSV rows, canonical9586 guardian bindings supplied by parent, read-only. No invented actor attack, non-demon stat scaling, C++ body, DB write, realm process or master destination merge.
- 2026-10-10 — [Toy-effects7 visual-only audit](systems/toys.md#toy-effects7-visual-only-audit-2026-10-10): three locally authored aura4 admissions, actual bag/use/cast/lifetime/cancel/no-NPC/no-absorb behavior,104→107/1166 with no removals. RED3/GREEN3; post-master toy39/39; required spell_cast314/315 retains unchanged guardian sacrifice damage-expectation failure, no rerun. Format/check pass. Missing runtime items, summon templates/control and interaction/scripts ranked separately; no synthetic action, world.db mutation, UDP5000, rendered-client certification or merge into master.

- 2026-10-10 — [Guardian actors and damage transfer](systems/spell-casting.md#guardian-actors-and-damage-transfer): authentic local creature-spell actor scheduling and native81 splitting across spell/periodic/melee/direct damage,6940 floor, atomic9586-row archive importer tested in memory only. Authentic111859 effect28/416/3110 fixture does not bind its root Dummy. Census168 retained,87/81; all40-spec script table refreshed. Named Wild Imp/Dreadstalker/Xuen/Rune Weapon/Darkglare/Tyrant bindings and damage-copy scripts remain underived. Final guardian8/8 and importer2/2; one requested post-master spell_cast run314/315 exposed creature hit-RNG in a fixed-damage concurrency fixture, corrected to deterministic melee units and targeted8/8; no repeated broad gate or fresh315/315 claim. Format/check pass on unchanged production code. No world.db/UDP5000/master/deploy mutation.

- 2026-10-10 — [Shared resource spell mechanics](systems/spell-casting.md#shared-resource-mechanics): native optional raw spending and launch-owned duration/damage snapshots; Rupture/Rip/Envenom/Black Powder/Barrage/Surge-payment bounded paths, eight focused functions pass.Post-master spell_cast307/307 (one run), fmt/check exit0.168 Dummy roots retained;87 reviewed/81 un-derived. All40-spec table refreshed. Actor mapping/budgets, transfer/copy and Surge interpolation remain unresolved; ungrounded linear Surge damage assumption removed. No DB writes, realm run or master merge.

- 2026-10-10 — [Class spell batch11](systems/spell-casting.md#eleventh-batch-local-mainline-low-count-paths): eight locally sourced bounded paths; Sinister level curve, Void Volley, Rain of Fire, conditional Paladin splash/wave, stealth Shred, Thrash bleed and named spell-mask-specific Lashing Flames, not a universal271 mask reinterpretation. RED0/9 missing/wrong behaviors; Final GREEN13/13 plus Colossus1/1, fmt/check pass. Single requested broad298/299 exposed universal271 incompatibility; final correction is named334168 only, no fresh299/299 claim. Census168 retained: Shred closes, Hammer exposes a latent child; all40-spec table updated. No DB writes, invented IDs, realm processes or master merge.

- 2026-10-10 — [Class spell scripts batch10](systems/spell-casting.md#tenth-batch-low-count-spec-paths): eight local Mainline paths prioritize core-zero Protection/Outlaw and core-one Arcane/Enhancement/Shadow/Demonology.11 native functions pass;170→168 Dummy roots,83 reviewed/85 un-derived;40-spec table/census updated. Whole-Dummy closure limited to Consecration and Feral Lunge; Guldan imp count and exposed Voidform child markers remain counted. No invented IDs, DB writes, realm process, deployment or master merge.

- 2026-10-10 — [Class spell scripts batch9](systems/spell-casting.md#ninth-batch-local-mainline-ground-fields-and-owned-marks): nine local Mainline paths and per-spec scripted census;171→170 roots. Ground fields, owned Atonement/base plague, rank-gated Meta Fury, Void Step and shield lifetime.14 native functions; requested broad gate once274/275, obsolete Blossom no-action fixture corrected/focused1/1; separate depletion RED and absorb5/5/Tranquility2/2. Final format/check pass. No DB writes, invented IDs, realm process or master merge.
- 2026-10-10 — [Toy-effects6 follow-up](systems/toys.md#toy-effects6-follow-up-2026-10-10): Duck Lovie source-bound nap admission, RED/GREEN and103→104/1166 (+1,no removals). Post-origin/master261 spell_cast/36 toy tests pass. Client Spitzy/ordinary train coverage and both-skin bounded asset captures documented; mortar live proof and absent periodic/procedural/ModifierTree payloads remain incomplete. No world.db/shared-realm/master changes.
- 2026-10-10 — [Pet effects2](systems/pet-battle-engine.md#effect-continuation--petbattle-effects2):15 added handlers; recursive rows1568→1727, admitted ability records913→987. Ranked99 hosted-map wild candidates;83 admit.17 new real-ID cases, corrected unsupported-object regression,72 engine +7 wild/startup tests pass after feature-branch master merge. Original partial handlers and90% admission remain unfinished; unknown semantics recorded, no DB writes/live realm/deployment/master merge.

- 2026-10-10 — [Class spell scripts batch8](systems/spell-casting.md#eighth-batch-projectile-taunt-and-low-coverage-paths): Reap destination line, real Provoke threat/victim/owned-statue path and eight additional bounded spell paths. Per-spell RED/GREEN, owner-only Breath damage reduction, requested-ground Sanctify cap, post-master census172→171. No database writes, fabricated IDs, deployment, master merge or realm process.

- 2026-10-10 — [Class spell scripts batch7](systems/spell-casting.md#seventh-batch-tdb-routed-retail-derivations): TDB1210 script/proc/link mining and Mainline effect chains derive eight bounded paths;21 native functions pass after correcting Bear mitigation in Survival's fixture. Census172 retained,70 reviewed/partial/deferred. No database writes, deployment, merge or guessed projectile/taunt bridge.

- 2026-10-10 — [Class spell scripts batch6](systems/spell-casting.md#sixth-batch-local-retail-rows): Sunfire, Verdict, Ice Lance, Starfire, Keg Smash, Butchery, Wild Growth, SWDeath backlash and capped owned Starfall extensions; local source receipts, per-spell cast proof and retained limits. No database writes, deployment, master merge or guessed selector104.

- 2026-10-10 — [Disenchant stale materials](systems/disenchant.md#stale-material-compatibility--2026-10-10): private real753 disposal/empty loot/next-cast lock reproduced. Compatibility refusal checks deliverable catalog materials and the actual one-time completion roll before disposal/success; real753→727 behavioural RED reproduced and controlled-RNG authentic partial entry5 safety test added. Census72 missing rows/16 items/50 affected entries (40 wholly,10 partial). Code519a34d: Disenchant8/8, native check and workspace format PASS. No DB writes, fabricated retired materials or post-fix rendered/deployment/merge claim.

- 2026-10-10 — [Toy effects4](systems/toys.md#transmitted-toy-objects-and-viewer-local-models-2026-10-10): replace ritual-only effect50 dispatch with authentic decoration/chair/mailbox summons, destination87 and explicit content/script failures; aura233 remains viewer-local client state, not global transforms. Targeted RED reproduced unsupported87/233 and wrong object refusal; requested-destination RED exposed incorrect target-facing inheritance. Current census75→95/1166 (+23 candidates,3 false positives removed),31 toy/19 ritual/12 teleport/7 summon tests pass with overlap; format/check pass. Retail TDB templates avoid stale Wrath banner displays. No database writes, engine changes or realm access.
- 2026-10-10 — [Pet effect expansion](systems/pet-battle-engine.md#effect-expansion--2026-10-10):40 handlers reference2605/2829 rows; global weather deltas/Elemental exemptions, team Decoy charges, conditional/drain/ramping/split attacks, forced swaps and original-actor proc cleanup. [Coverage matrix](../pet-battle-effect-coverage.md) preserves unknown semantics and distinguishes admission from behavioral proof. No merge or production operations.
- 2026-10-10 — [Stationary arch and leash](systems/navigation.md#stationary-arch-and-leash-acceptance): real supported Stockade perch exposed immediate no-path evade; restored pinned Retail5s non-raid grace with reachability/victim resets. Targeted56/56 GREEN and rebuilt private5094 live proof at6c96656: sampled39.888yard leash reversal/home and5.103s no-path hold/home with stationary supported feet.12 inspected PNGs/two VFR videos; no world.db writes, shared-realm deployment or master merge.

- 2026-10-10 — [Toy effects3](systems/toys.md#periodic-dummy-no-actions-and-linked-summons-2026-10-10): data-backed226 ticks retain channel targets; conditional/scripts remain blocked.321 uses PreventionType4, not MiscValue; linked428 lifecycle and authentic24-template import unlock tent summons with DB2 target18/79. All17 bobber258 toys remain explicit fishing/object gaps. Private prepared census71→75/1166; no canonical database, client, merge or realm changes.
- 2026-10-10 — [Class spell scripts batch3](systems/spell-casting.md#third-batch-local-retail-rows): eight bounded local-Retail paths (Prayer, Garrote, Serenity, Scorch, Shred, Thunder Clap, Devastate, Moonfire); per-spell cast proof/status and unresolved variants in `handoff-spell-dummy3.md`. No database writes, deployment or merge.

- 2026-10-10 — [Disenchant](systems/disenchant.md): Retail item-GUID casts, authentic eligibility/TDB rolls, atomic unbound disposal receipts and shared personal loot; six native gameplay tests plus metadata1/loot27/known-spell1 pass. Companion protocol/client required; no merges/services/live proof.

- 2026-10-10 — [Generic toy aura effects](systems/toys.md#generic-scale-fall-effect-immunity-and-override-sets-2026-10-10): additive61, feather105, effect-ID37 and OverrideSpellData293; authentic3050-row source, temporary permission/replicated slot list and removal semantics. Census71/1166 (3 this branch,10 earlier master gains versus historical58). Private live/client proof boundary lives in toy-effects2 handoff; no merge or protected-realm mutation.
- 2026-10-10 — [Pet battle engine](systems/pet-battle-engine.md): pure server library with stats, 3v3 decisions, injected RNG, common effects/auras/cooldowns/multi-turn moves and nine family passive cases. Code2f99296 targeted23/23; format/check exit0. [104-property matrix](../pet-battle-effect-coverage.md) distinguishes12 handlers/930 statically admitted rows from full Retail proof; SQLite battle-table producer, remaining92 IDs and exact timing/rounding remain open. Independent verification blocked by expired Claude OAuth. No network, UI, deployment, merge or UDP5000 work.

- 2026-10-10 — [Appearance acquisition](investigations/appearance-acquisition.md#never-created-account-table-regression): missing account table reads return no owner; economy helper RED→GREEN and missing/empty/multi-account reopen test. Code `f654bcc`; economy11, acquisition14, appearance4, persistence106 passed/18 ignored. Disenchant/scrap handlers re-audited absent; no profession implementation.

- 2026-10-10 — [Appearance acquisition](investigations/appearance-acquisition.md): BoP binding at store, atomic account visual receipts on equip/bind/sale/destroy, all quest choices, durable owner refresh. Targeted RED/GREEN and reopen matrix in `/home/osso/.worktrees/handoff-transmog-acquire.md`; disenchant/scrap handlers and source-level/client parity remain absent. No full gate, merge or deployment.
- 2026-10-10 — [Toy effects](systems/toys.md#data-backed-toy-effects-2026-10-10): generic client-owned ScreenEffect260 aura support; strengthened concrete Time-Lost Figurine display/cancel/revert proof. Shadescale's authentic spell has mechanism proof only because its runtime item is absent.

- 2026-10-10 — [Toy effects](systems/toys.md#data-backed-toy-effects-2026-10-10): audited visual-only dummy/aura/script path; shared runtime/preflight summon data gates, wild faction and companion follow/immunity/slot semantics. Missing Retail creature templates and real toy scripts remain explicit; CPU/loopback proof ledger in `/home/osso/.worktrees/handoff-toy-effects.md`. No rendered-client, merge or deployment claim.

- 2026-10-10 — [[toys]] server/protocol foundation: local-CASC Toy1166, unresolved joins retained; atomic account learning/consumption and favourites, all-character account broadcasts, UseToy spell/item/category dispatch and explicit unsupported preflight. CPU/loopback toys8/0 and item-use regressions11/0; no UI, live realm or effect-completeness claim. [Contract](../specs/toys.md).
- 2026-10-10 — [Spell teleports and aura overrides](systems/spell-casting.md#teleports-and-aura-overrides): data-backed Blink ground/LOS endpoint, homebind persistence and cross-map handshake, matrix jump41/254 endpoints, aura332 live substitutions and aura312 animation set transport. Companion shared-protocol branch required; client consumption and actual fixed-destination source rows remain separate gaps. Proof ledger: `/home/osso/.worktrees/handoff-spell-teleport-override.md`. No merge, deployment or live client claim.

- 2026-10-09 — [Retail auction mail](systems/mail.md#auction-house): server sends structured text tags and invoice fields; native client formats local GlobalStrings, not invented letters. [Contract](../specs/auction-mail-text.md); expiry/reopen RED2 followed by auction GREEN82/0/2, full-workspace proof plus corrected real-character fixtures. Private5510 native inbox text captured; no broad mail-policy change.
- 2026-10-09 — [Spell auras](systems/auras.md#mechanic-immunity-pushback-and-normalized-movement): data-driven mechanic immunity 77, damage pushback reduction 149 and absolute run/swim/flight cap 191; concrete catalog regression tests. Targeted proof ledger: `/home/osso/.worktrees/handoff-spell-auras.md`; full suite gate belongs to lead. No client change or deployment.
- 2026-10-10 — [Spell summons and area triggers](systems/spell-casting.md#temporary-summons-and-spell-area-triggers): effects 28/179, local-CASC SummonProperties importer and ownership/expiry. Six targeted behavioural tests pass: Rune Weapon cast/lifetime, Barrier lifetime, four more available matrix area roots, real Purging Breath enter/radius/expiry, periodic timing fixture, owner map departure. Six summon roots lack creatures; all nine matrix area roots lack trigger bindings, four also require unsupported ground selector 87. No full gate, deployment or full matrix gameplay claim.

- 2026-10-09 — [Auction expiry](systems/auction-house.md#injected-expiry-clock-2026-10-09): injected ServerClock and existing clock-advance admin path; real posting/bidding and reopened mail regressions. RED a999851:2/2 deadline failures; GREEN ab4cd69:2/2 pass. Full locked workspace gate at ab4cd69:1586 passed/0 failed/60 ignored/1 required 300k timing skip; format pass, no warnings. Exact proof ledger linked in system page. No live claim.
- 2026-10-09 — [Defensive fixes](../defensives-audit.md): shared melee immunity/absorb handling, matching event outcomes, aura 49 player dodge and scripted Brew health/reduction. Five gap tests enabled; current gate evidence and remaining Blur/Turtle limits live in the audit. No merge or deployment.

- 2026-10-09 — [Class defensives audit](../defensives-audit.md): all 13 supported classes have catalog candidates; real defensive casts demonstrate eight class reductions on spell damage and melee, two spell-immunity cases and one consumed spell absorb. Five retained ignored failures expose missing melee immunity/absorption, Evasion dodge and Fortifying Brew scripting. No gameplay fixes, client or live realm proof; final suite counts in audit ledger.

- 2026-10-09 — [Spell casting](systems/spell-casting.md): single-slot 400 ms player queue; defer full validation/costs until cast effects and GCD release. Explicit stop clears pending; hostile interrupt/death retain for revalidation, inferred from pinned TrinityCore a352b1fa. [Contract and source limits](../specs/cast-presentation.md#spell-queue-window); targeted connection RED on 2b98e75: 8 missing-queue failures, 1 outside-window control pass (`/home/osso/.worktrees/logs/spellqueue-red2.log`). Initial GREEN on a5a42eb: 9/9. Follow-up exact-400ms RED on 5bc4190: 0/1, exposed f32-to-f64 boundary widening; compare admission at CastState's native precision. Final GREEN on 9726ab6: 10/10, zero ignored, no warnings, EXIT0 (`/home/osso/.worktrees/logs/spellqueue-final-green.log`); changed-file rustfmt check passed. No live Retail/client proof.

- 2026-10-08 — [Replicated player life state](systems/networking.md#player-life-state): changed death/release/resurrection projection and stationary observer visibility, governed by shared ghost_visible; pinned TrinityCore a352b1fa sources. Targeted UDP/interest RED 0/2 followed by GREEN 2/2 at 9fcd652, including no transient decoded ghost frame for a living observer. Logs: /home/osso/.worktrees/logs/ghoststate-server-{red,green}.log. No live realm or persistence-parity claim.

## [2026-10-07] systems | Merchant sale order and bag repair

Updated [merchant](systems/merchant.md): chronological wrapped buyback lists retain physical request slots; item-guid durability survives unequipping and owned bag-item repairs save damage and payment atomically. Contract: [merchant spec](../specs/merchant.md). Live and suite proof ledger: `/home/osso-test/Projects/world-of-osso/game-engine/data/diagnostics/merchantloop-2026-10-07/proof-ledger.txt`.

## [2026-10-07] systems | Death logout/login persistence

Updated [[persistence]] with reproduced restore → select → initial-snapshot ordering race, atomic full death-state checkpoint, exact graveyard restoration and additive saved-data compatibility. Regression boundary: reopened redb → new SelectCharacter session. Proof ledger: `/home/osso-test/Projects/world-of-osso/game-engine/data/diagnostics/deathpersist-2026-10-07/proof-ledger.txt`. No merge, push or deploy.
## [2026-10-07] checkpoint | RandPropPoints source applicability

Updated [[skyborne]] and index: Main reports faithful pinned Forever70205 RandPropPoints16 schema lacks Retail DamageReplaceStatF/DamageSecondaryF; rebased shared-loader requirement is the defect. Agent267 correction pending: Retail-only spell budgets, mandatory normal Forever scaling, empty Forever spell-budget map. Retail effect evaluator does not establish Forever item-spell support; no fabricated/borrowed data or fallback. Source-aware aura/group constructors committed at cbd588f, not proof. Targeted proof/rebuild/native pending; historical scopes retained, scratch stopped, character30/40XP preserved. Docs only; no specs, operations, delegation, push, tests or builds.

## [2026-10-07] checkpoint | Rebased Skyborn source and protocol boundary

Updated [[skyborne]] with Main-supplied clean90/no-skip rebase onto8b3819b0 at pushed bef3fa745, backup ref/cold data-binary target, preserved character30/40XP and conflict-resolution boundaries. Historical1448 receipt retained but changed scopes invalidated. Native auth succeeds then old-runtime48/client510 fingerprint rejection before world; no NPC/graphics or current server tests/build GREEN. Linked engine CPU/build checkpoint. Docs-only; Main owns matching desktop rebuild; no operations, tests, delegation or push.

## [2026-10-06] systems | Player resurrection and creature taps

Added [offer/tap mechanics](systems/player-resurrection-and-taps.md) and [contract](../specs/player-resurrection-and-taps.md): Retail effects 18/172, implicit-zero target validation, owner consent/expiry replies, cast-position restoration, stable character tap IDs and roster comparison. Targeted proof ledger: `/tmp/claude/rezrtap-proof.md`. No production service, merge, push or deployment.

## [2026-10-06] systems | Dungeon objectives and achievement window data

Updated [[dungeon-achievements]] and its spec with phase-2 local category import, bounded request-only catalog/criteria pages, persisted earned dates, and copy-scoped dungeon snapshots. [Client hookup](../dungeon-achievements-client.md) pins fields, bounds and Retail file:line references. Import fixture GREEN; all six new server cases and four wire tests pass across the final scoped proofs. Stockade reconnect uses a new connection/player loaded from saved state; both copy members receive updates. Real continent world-boss rows exposed a clearing bug, reproduced and fixed by actual-copy selection. Eight phase-1 regressions and 24 additional Stockade tests pass (three existing appearance tests ignored). Maximum category payload 56,397 bytes; final check/fmt pass without warnings. Exact commands/revisions in the system page ledger.
## [2026-10-06] systems | Group meter categories

Updated [[combat]] with actor/victim group fan-out, shield-caster attribution and logged/transition death de-duplication. The [damage-meter spec](../specs/damage-meter.md) owns category semantics, export bounds, Retail references and the client migration field list. Targeted local proof: 19 server meter tests and four protocol-filtered tests pass. First-absorb roster RED reproduced before the seeding fix; final server build reports no warnings.
## [2026-10-06] systems | Group roster portrait transport

Linked [roster portrait transport](../group-roster-portraits.md): canonical customization plus head-only equipment, change-triggered reliable rosters and offline cache. Added persisted inventory-request, post-transfer and latest-offline-head fixtures. At server `cb40477`, all four targeted roster tests pass; protocol `e0a6d19` round trip passes. Corrected fixture race/class and incomplete loopback receiver pairs; inventory PostUpdate requires a fourth frame before roster receipt. Production implementation from `737fef6` is unchanged. Corrected deferred client path to `godot/ui-model/src/game/group_state.rs`; existing `apply_roster` already retains whole snapshots. Client unchanged; native Retail helm pixel parity remains unproven.

## [2026-10-05] investigations | Northshire native versus effective stats

Updated [[level-scaling]] with the `c8cd38f` audit: ContentTuning 73 gives native level 30 / 4,379 health, but level-1 effective stats are 1 / 78. `npc-info` reports the native pool without a viewer. Source difficulty rows agree; Elwynn and Durotar share the pattern. Added three real-spawn `creaturelevel_*` regressions, including tuning-13 Flesh Eater's higher floor and lethal scaled health loss. Client runtime failure is not established; server native-level semantics preserved.

## [2026-10-05] systems | SmartAI combat caster movement

Updated [[boss-scripts]] and its spec: Geomancer 476's loaded UPDATE_IC/VICTIM row never ran because those event/target types were unsupported. Added combat-only initial/repeat timers, victim targeting, CMC chase hold and cast-failure retry/chase recovery, citing pinned TC SmartScript/SmartAI. Four real-data regressions reproduced absent Fireball in RED; all 15 targeted scripted-AI tests passed at `798c4c6` (four new, eleven existing), including exact 3–5 s repeat and sustained exhaustion. Impact: 9,371 combat-update and 1,411 CMC cast rows in the loader's current creature set. Other unsupported script types remain gaps.

## [2026-10-05] systems | Recuperate full-health decision

Updated [[auras]] and its spec: user-requested completion of aura 1231418 at full health, unlike TC's permanent-aura-only skip. Full-health casting remains allowed; TC's ALREADY_AT_FULL_HEALTH check applies to consumable direct heals, with no applicable Recuperate data/Retail rule established. Real-spell behavioral tests cover aura/channel completion, no later heal logs, and unchanged Renew. Targeted RED reproduced three retained-aura/channel failures while Renew passed.

## [2026-10-05] systems | Creature threat telemetry

Updated [[creature-aggro]] with effective healing assist threat, school-matching MOD_THREAT, and rate-limited engaged-player table/clear delivery. Existing damage table and 110%/130% victim selection retained. Targeted RED reproduced absent heal threat (200 instead of 250); 9 targeted threat tests passed, including real Flash Heal, loaded Righteous Fury, engaged-player loopback delivery and reset clear. Native target frame has no threat indicator; client Threat type uses the selected target.

## [2026-10-05] systems | Capital city rested XP

Updated [[experience]] and added the city-rest spec: AreaTable import/loader, MCNK position-derived areas, parent-zone capital and faction checks, existing pool/snapshot lifecycle. TC a352b1fa UpdateZone/UpdateHostileAreaState and real Trade District/Orgrimmar/Elwynn assets ground the behavior. RED reproduced Stormwind not resting (37 passed, one failed); GREEN at `39f31b2`: 41 rest tests and three AreaTable import tests pass. Full local CSV import into a private database verified 9,848 rows without touching canonical data. Canonical world.db import and live runtime are explicitly excluded from delegated authority.

## [2026-10-05] investigations | Lift Off bar and restriction evidence

Updated [[mounts]] with full pinned 12.1.0.69933 Wago exports: OverrideSpellData 2106 and SpellShapeshiftForm 47 list Skyward Ascent 372610, not 374763; MountCapability 494 points to FlightCapability 11 with SpellID 372610; SpellActionBarPref 343 prefers 372610. No 374763 override/preset/preference or aura/casting-requirement row found. SpellMisc 633542 includes NOT_AN_ACTION and IGNORE_CASTER_AND_TARGET_RESTRICTIONS. Existing SpellCastIntent already carries the ID; native 374763 action/grant and exact unmounted/ground failure remain blocked. No production, wire, data, test or service change; no build required for this documentation-only milestone.

## [2026-10-05] investigations | Summoned-pet dismissal

Updated [[warlock-pets]] with Retail popup command-vs-hunter-spell behavior, TC command-3 dismissal/deletion distinction, and DB2 rows 35949/334/980. Removed artificial warlock fixture grant and added hunter/warlock progression assertions. Required RED is blocked before compilation by canonical desktop-server Dockerfile's obsolete `refresh_sources` call; no production fix or GREEN claimed. Existing PetAction wire supports command 3; client PetFrame menu remains separate required work.

## [2026-10-05] investigations | Imp Firebolt live capture gap

Updated [[warlock-pets]]: original fixed-SP/kobold fixture replaced with computed level-3 stats/full spell catalog and neutral Northshire dummy plus real terrain/LOS. Seven targeted tests pass at `c8da7a4`, including Firebolt and Dismiss 2641. No RED or demonstrated AI defect; no production fix. Client IPC/admin melee history is separate from owner spell events, and the IPC formatter omits spell IDs. Live pet-state/cast-rejection/owner-spell trace remains required.

## [2026-10-05] systems | Skyriding effect 203 and Lift Off investigation

Updated [[mounts]] and the Skyriding spec: TC a352b1fa effect 203 is REMOVE_AURA_2 / EffectRemoveAura, removing TriggerSpell auras through the normal lifecycle. Recorded the real Surge Forward, Skyward Ascent and Whirling Surge tutorial-highlight removals. RED reproduced retained highlight 1259214 after Surge Forward. Explicitly learned 374763 already applies FORCE_CAST 404191 for two seconds; data contains DUMMY 450, not LEAP/JUMP. Native 374763 trigger and exact not-skyriding failure remain source-blocked; no invented launch rule, client/wire change or default grant.

## [2026-10-05] systems | Warlock Summon Imp

Added [[warlock-pets]] and its spec: effect-56 MiscValue creature entry, shared pet bar/Firebolt AI, data-derived power, permanent-current relog and dismiss/death removal. Documented TC function references and the explicit level-1 kit override (imported DB2 level is 3). First targeted run passed 24 shared-pet regressions and exposed two boundaries: retail Firebolt costs Energy (not assumed mana) and needs the TC owner SP contribution; relog fixture endpoints must exist before InWorld restoration. Corrected both. No wire/client/schema change; base pet stats/names and live proof remain out of scope. Targeted proof: six imp behavior tests pass at 226c8df; level-1 grant and 24 shared-pet regressions pass at production revision 9d8acac (unchanged by fixture-only 226c8df).

## [2026-10-05] investigations | Fresh-character General grants

Added [[general-spell-grants]]: read-only SQLite/CSV comparison of 41 captured entries; Attributes_13 bit 16 marks 31 content abilities plus Throw, while ordinary entries clear it. Other requested fields do not separate them. Unknown bit semantics and Throw eligibility block a verified rule; no server code/data change or behavioral proof.

## [2026-10-05] systems/spec | Reconstructed NPC coarse stat estimates

Updated [[content-import]] and [contract](../specs/zephras-content.md): approved class-1/modifier-1 estimates only in reconstructed missing-stat templates, preserved captures and labelled analogue/curve provenance. Targeted Python tests cover imported fields, source retention, classification preservation, regeneration and transactional Retail-stat collisions; development proof in `target/zephras-stat-estimate-proof/`. No Rust, data publication, runtime acceptance, original scripts or Ailee texture/profile resolution.

## [2026-10-05] systems/spec | Source-qualified bag requirements and unique limits

Updated [[inventory]] and [source contract](../specs/item-definition-source.md): explicit model source arguments, Retail profession reagents/tools, quest progress/removal and producer-selected merchant/loot unique limits. Collision fixtures use unchanged 2947/2512/2101 IDs; native targeted RED reproduced eleven expected failures before production edits. Desktop GREEN at source commit `1a1b049`: 11 passed, 0 failed/ignored, 1434 filtered; 11.44s test time, 80.104s helper wall. Main-owned zone test patch was present but not selected. Development ledger: `target/source-requirements-proof/`. No authored reward provenance inference, legacy-marker change, broad gate or operations.

## [2026-10-05] systems/spec | Frozen character equipment disk ABI

Updated [[persistence]], index and [storage contract](../specs/character-customization-storage.md): private V7 DTO, frozen V3–V6 equipment fields/slot order, explicit six-writer boundary, GUID-based cache hydration and rejected unowned Forever cache writes. Temporary redb tests cover independent nonempty V7/V6 fixtures, historical migrations, wire source serialization and actual ten-pair creation/reopen. No live database, new decoder, deployment or merge-readiness claim; main owns final integration proof.

## [2026-10-05] systems/spec | Isolated Forever scaling CSV import

Updated [[skyborne]] and contract with the nine-table source-local scaling CLI, actual catalog fixtures, exact provenance and atomic Retail-preserving refresh. At `678c9df`, capped `python3 -m unittest discover -s scripts/tests -p test_skyborne_scaling_data.py` passed 8 tests in 24.252s after RED; [proof ledger](../../target/skyborne-scaling-proof/ledger.json). No Rust edits, canonical/live publication or runtime acceptance claim.

## [2026-10-05] systems/spec | Owned source runtime boundary

Updated [[inventory]] and [source contract](../specs/item-definition-source.md): source-qualified catalogs, stacking, explicit incoming origin, owner transfers, auction minted-GUID transaction markers, source-local equipped scaling and wire records. Targeted `owned_sources` RED at743482e:3/5 pass, cross-source occupied split and vanished-GUID failures. Fix4fea2d0:5/5 GREEN, no warnings; logs `/tmp/claude/owned-source-{red,green}.out`, ledger `target/owned-source-runtime/ledger.md`. No final acceptance claim. Unowned content-source and Classic-parity gaps remain explicit.

## [2026-10-05] systems/spec | GUID-owned definition sources

Updated [[inventory]] and [source contract](../specs/item-definition-source.md): additive immutable GUID markers preserve Retail/Forever identity independently of owner transfers without altering inventory or soulbound encodings. Targeted persistence tests cover reopen, two-owner transaction exchange, legacy Retail and invalid/changed markers; creation/runtime integration remains separate.

## [2026-10-05] systems/spec | Isolated kit creation and regeneration

Updated [[skyborne]] and contract: isolated shared definitions, source-local creation/count readers, exact importer-generated ten-pair redb fixture, full DB2 regeneration preservation, additive kit refresh, and standalone process-level rollback. Fixed outer-transaction gap exposed by projection collision. Native fixture proof awaits concurrent runtime catalog/constructor closure; no complete stats/transfer/tooltips claim.

## [2026-10-05] systems/spec | Authored Skyborne kit source closure

Updated [[skyborne]], index and contract: six pinned Forever loadouts, 45 memberships, 30 readable Item/ItemSparse definitions and selected appearance/effect parents. Strict collision/rollback tests preserve Retail loadouts. Recorded shared-ID count/type/binding conflicts and user-selected isolated Forever definitions; full ten-pair authoritative creation remains pending, not empty-kit success.

## [2026-10-04 local] systems | Post-sync Forever ground and spawn recovery

Updated [[ground-assets]], [[content-import]] and index after local CASC synchronization: 256 ground dependencies/zero missing; desktop ground fixtures4/4; 72 center records22supported/50holes, Python/native72/72 agree; full72-tile LOS exits0 with zero unreadable files, eye tables unchanged. Existing normalization/import recovers exactly463 eligible observations into a new scratch DB, preserving1199 previous rows and producing1662 spawns. Grounded displayIDs174→218. Quest/content gaps and reconstructed-observation semantics unchanged; no live deployment or native NPC acceptance. Revision-scoped logs/reports: `target/forever-resync/`, [ledger](../../target/zephras-content-proof/ledger.md).

## [2026-10-04] systems/spec | Refreshed Forever capture and NPC appearances

Updated [[content-import]], [[skyborne]] and content contract after retry: all 146 normalized quests/298 NPCs/3 objects captured; historical 403/missing-display claims superseded. NPC-page appearances are primary, including Ailee 136968/Ventaari 139694; full 235-CDI list exports all captured appearances, not just 174 grounded-spawn IDs. Scratch overlay now has 278 templates/1,199 grounded observations/278 model rows/146 quests. Approved starts and race-96 inference unchanged; 20 level omissions, 27 giver links, 49 blocked completions, 109 absent rewards and native/client asset gaps remain explicit.

## [2026-10-04] systems/spec | Approved reconstructed Zephras starts and content

Updated [[skyborne]], [[content-import]], index and contracts: approved starts/facing, hosted-map gate, ten-pair real-kit reopen/login/terrain proof, supported start ground, native quest acceptance/XP and level-2/event boundary. Recorded 276 templates/878 grounded observations/136 quests, atomic regeneration/provenance, unchanged source masks, 0/295 DB2 display coverage and three captured displays. Missing pages, starting-giver models, native quest-side/reputation/reward-spell/event support and full terrain closure remain explicit gaps. Scratch-only publication; no engine checkout, live world DB or player storage writes.

## [2026-10-04] systems/spec | Forever Zephras reference capture

Updated [[content-import]], [[skyborne]] and index; added capture contract. Recorded per-page Forever capture, explicit partial metadata, CloudFront/browser 403 block, retail-calibrated UiMap inversion, real 72-center terrain comparison and conditional start proposals. No server import, mask widening, phasing or scripted events; race-96 level-1 intro remains unsupported by captured pages.

## [2026-10-04] systems/spec | Zephras ground and partial LOS

Added [[ground-assets]] and its contract; reconciled [[skyborne]] prerequisites. Recorded explicit registered-map/build-selected local extraction, 203 cached dependencies with 42 unavailable, authored-grid seam correction, required shared MPY2 collision commit, three real desktop fixtures, 21 terrain regressions, all 72 center records, and 48-tile partial LOS/runtime hit proof. No invented start, live database update or full-map coverage claim.

## [2026-10-04] systems | Skyborne server support

Added [[skyborne]] and server spec: verified Forever race bits/classes/factions, source-decoded faction/map overlay and regeneration, ten-pair real-kit storage-reopen/login proof, unchanged Northshire placement, and map-2991 ground/LOS prerequisites. No live world database writes.

## [2026-10-03] systems | Flight Style toggle

Updated [[mounts]]: Switch Flight Style 436854 / 460002 / 460003 as DUMMY spell scripts; the Flight Style aura is saved per character and restored on login. Skyriding takeoff = Skyward Ascent cast (FORCE_CAST 140 Lift Off 404191); the movement bound launches from that flap only.

## [2026-10-02] systems | Mounts and steady flight

Created [[mounts]]: Golden Gryphon 32235 mounts through MountCapability (riding + Flight Style: Steady), flight speed 207, CAN_FLY/FLYING movement without gravity, AffectingCombat/NotMounted cast checks, CancelMountAura, admin `learn-spell`. Not modelled list on the page.

## [2026-10-01] investigations/systems/spec | Combined Zaralda catalog and native blocker

Updated [[midnight-economy-content]], [[economy-catalog]], [bot spec Known gaps](../specs/bot-economy.md#known-gaps-current-cycle) and index. Recorded closure 30/30 zero-warning proof, retained additive data proof, scoped gate103 catalog/CLI/restart PASS and later established combined status. Corrected raw TDB flag 0 versus source-observed derived content flag 128; no native vendor-flag inheritance claim. Former merchant data gap resolved locally; native 0/3 and overall goal remain open. Linked [engine fixture documentation](../../../game-engine/docs/wiki/reference/test-assets.md#zaralda-native-merchant-fixture-2026-10-01). No new test/build/service operations; historical unchanged-Classic/pending-gate entries below describe earlier checkpoints.

## [2026-10-01] investigations/systems | Approved derived Zaralda publication

Updated [[midnight-economy-content]], [[content-import]], [[economy-catalog]] and index after committed terrain `830157a`, publisher `94e6d88`, integration `e43f4d3` and local publication. Recorded native template/model/difficulty identity, actual CASC ground sampling, explicit approximate placement/local facing policy, tracked required-manifest rebuild preflight and protected additive publication. Two thread/floss suppliers only; unsupported currency rows excluded. Removed obsolete identity/stock/exact-capture blockers; preserved recipe limitations, unchanged Classic catalog/service and pending independent integration gate. Bounded development GREEN is not accepted integration or full-goal completion. The source-backed Zaralda candidate catalog validates with15recipes/21markets/2gatherables; live acceptance remains pending.

## [2026-10-01] investigations | Browser-observed Silvermoon merchant

Updated [[midnight-economy-content]] from saved browser-cli Zaralda evidence: NPC243531, Leatherworking Supplies, all 34 sells rows extracted without individual item navigation, two thread/floss costs700 and raw stack/availability, plus Silvermoon uiMap2393 marker43.2,55.8. Canonical item rows confirm price700 and bundle1 for both reagents; neither appears in the 15 staged recipes. Preserved patch12.0.0 provenance and generic thread736 discrepancy. Follow-up item listings explicitly show unlimited stock and bundle1; corrected the overstated Retail-captured elevation/facing gate. Derived placement remains a user decision under the no-fabricated-spawns contract. Linked parsed/raw temporary artifacts. Classic/catalog unchanged.

## [2026-10-01] systems | Exploration quests

Updated [[quests]] and [[content-import]]: TrinityCore area trigger objectives and completion flags map to AzerothCore's exploration/event flag plus `content_areatrigger_involvedrelation`; the server credits a quest when a living player walks into one of its `AreaTrigger.db2` spheres or boxes. The Fargodeep (62) and Jasperlode (76) Mine quests no longer complete on accept.

## [2026-10-01] systems | Verified crafting reference publication

Published the verified crafting references and additive Retail item projection to local `world.db` after the ingestion gate. The independent publication report confirms existing rows and unrelated tables remain unchanged; [[midnight-economy-content]] owns the exact counts, source provenance and diagnostic limits. Rebuild preflight is tested and documented in [[crafting-reference-import]]. The active Classic catalog was not replaced, no production service was restarted, and genuine new Silvermoon merchant placement remains unavailable.

## [2026-10-01] investigations | Corrected Midnight economy source diagnosis

Added [[midnight-economy-content]] after checking existing investigations; corrected [[economy-catalog]] and bot-economy spec, linked index and marked historical effect-24 inference superseded. Saved staging evidence replaces missing-recipe claims with modern membership/resolution and published-reagent coverage; genuine city placement and Spark/quality/native-bonus limits remain. Source publication and agent75 CLI preflight remain pending. Preserved independently owned reference-schema/pipeline docs. Docs only; no source/data writes, tests, operations, delegation or push.

## [2026-10-01] reference | Bounded crafting reference import

Added [[crafting-reference-import]] and its spec: seven pinned local-CASC/WagoCSV tables, complete unknown-preserving schema, float32 conversion, validated duplicate spell relationship parent, deterministic provenance and atomic staging-only replacement. Concrete WDC5/CSV and real-source joins cover invalid-source and SQL rollback. Economy catalog/content policy and live data remain untouched; modern resolver handoff stays separate.

## [2026-10-01] systems | Bot return accounting and final acceptance

Updated [[bot-economy]] and [[auction-house]] with independent CLI/restart/native gates and real bot/player parity across 1/7/14-day auctions. Fixed delivered bot attachments being discarded during collection: returned inventory now persists and is consumed by relisting, including GUID-preserving whole stacks and split quantities; rollback/reopen regressions pass. Recorded twelve pinned loopback samples at 60.282–121.623 ms without a universal latency claim. Missing Midnight/Silvermoon content and operator cap-reduction reconciliation remain explicit limits.

## [2026-10-01] ledger | Final bounded economy/AH receipts

Updated [[bot-economy]], [[auction-house]], [[economy-catalog]] and specs from saved artifacts: pinned `eb649f85` binaries/twelve CLI samples, exact live totals, historical default failure/focused delta passes, sibling native ledger `5b27878f` and agent51 bounded PASS. Preserved cap-reconciliation and Midnight/Silvermoon blockers, provisional tuning and pending agent52/main readiness decision. Docs only; no test/build/ops reruns.


## [2026-10-01] specs/systems/reference | Exact economy capability and saved proof ledger

Updated bot-economy and auction-house specs/wiki, economy-catalog, auction-load-fixture and index from current `8193d7e` source (including `b0153e4`, importer `13241e0`, runtime `cd9b690`) and saved `/home/osso/.cache/economy-cli-20260930/a54a0ad0` JSON receipts. Ledger separates 300000/311-real-ID producer proof from live 300075-offer CLI workload; records five request-response samples, actual auction actions/refusals, normal restart/mail outcomes and three-boot supplier costs/purchase/cap accounting. Checked bullets are bounded receipts, not broad tests, guaranteed products or universal latency/concurrency policy. Restricted recipe2149/capacity3/customer0/cycle2 fixture is not production policy. Midnight mappings/new Silvermoon remain content gaps; native AH/mail source implemented, live engine proof IN PROGRESS. Canonical restoration is user-reported; server work isolated. Docs only; no engine docs, code, data, operations, delegation or push.

## [2026-10-01] systems | Direct supplier runtime

Updated [[bot-economy]] and its spec with exclusive supplier selection, exact whole-bundle payments, retained paid leftovers and pre-cycle rejection of limited/extended-cost suppliers. Uses existing central market commit and persisted cycle timestamp. Added supplier rejection/failure tests and corrected the no-fallback auction fixture to exercise an affordable, price-bounded offer. Live CLI/native/load proof remains main-owned.

## [2026-10-01] systems | Ingredient sourcing and direct suppliers

Updated [[economy-catalog]]: direct loot preserves raw Light Leather 2318 despite conversion 2881; subclass 5 alone does not gather vendor-only threads 2320/2321/4291. Config gains `vendor_supplies` with exact bundle `purchase_price`, positive `buy_count`, expanded runtime slot and stock fields. Deterministic active spawned unlimited suppliers only; limited/extended-cost/invalid/unavailable rows remain explicit, unsupported vendor-only inputs exclude recipes. Secondary crafted goods remain AH inputs. Runtime purchase and live integration remain main-owned.

## [2026-09-30] reference | Owned auction load fixture

Added [[auction-load-fixture]]: explicit ignored 300k-offer producer using real importer catalog, read-only item templates, `AuctionData` and central market commits. Documents atomic nonexisting-file ownership, reserved counters, complete reopen equality and meaningful grouped/ItemID second-page assertions. No production-policy or network-latency acceptance claim; main owns invocation and CLI proof.

## [2026-09-30] systems | Read-only economy admin snapshot

Updated [[bot-economy]] with `game-server-admin economy-status`, snapshot fields, copper/timestamp units and missing-initialization errors. Exact-value state and CLI tests added; live server diagnosis remains main-owned, with no economics tuning or data mutation.

## [2026-09-30] reference | Live GameObject admin lookup

Updated [[dev-commands]] with read-only `game-object-info <map_id> <instance_id> <entry>`: runtime entity bits and WoW coordinates for normal mailbox interaction, never inferred from spawn GUIDs. Targeted tests cover concrete mailbox output, map/copy/entry filtering, and not-found results; live server/CLI integration remains main-owned.

## [2026-09-30] systems | Merchant output price boundary

Updated [[economy-catalog]]: recipes whose output range exceeds existing u32 vendor prices are excluded with unchanged market values, protocol limit and TSM provenance. Input-only AH markets retain u64 prices. Read-only Lara 1295:0:165 import excludes one recipe (10621/output 8345), leaving 223; main owns fixture regeneration and startup proof.

## [2026-09-30] systems | Global auction browse contract

Added [auction house spec](../specs/auction-house.md) and [[auction-house]]; linked [[bot-economy]] and index. Inspected shared `88bc3fe`: distinct browse messages with per-item price/quantity and distinct total, unchanged flat shapes. Inspected server flat/session/class/usability boundaries; grouped server, CLI and native integration remain concurrent and unverified. Recorded ItemID → real AuctionID flow, actual-coordinate/listing-price evidence and CLI-first then engine <200 ms request/response goal. Reported optimized flat 300k/144 ms helper timing is not network acceptance; grouped performance pending. No acceptance checked; no code or owned CLI/engine docs changed.

## [2026-09-30] systems | Economy catalog importer (Midnight diagnosis superseded)

Correction 2026-10-01: [[midnight-economy-content]] supersedes the effect-24-only inference below; modern Skillup2915/effect288 recipes exist. Historical counts are not modern coverage.

Created [[economy-catalog]] and indexed it. Read-only world.db and timestamped public TSM snapshots supply runtime item metadata, market maps and explicit spawned merchant/profession/expansion assignments. Jillian Tanner 5565:0:165 has 224 classic Leatherworking recipes, 312 markets and 33 raw gatherables; quantity follows profession.rs raw rounding, not points + 1. Report records selected price/popularity provenance, exclusions and explicit empty-production warnings. Midnight 11 remains blocked: 115 CREATE_ITEM output spells, six Fishing mappings, 109 missing mappings, no Leatherworking mapping or claimed new Silvermoon spawn. Runtime integration and spec acceptance remain separate.

## [2026-10-01] systems | Transform templates kept by the import

The import keeps TrinityCore templates that TRANSFORM auras name though nothing spawns them; `scripts/add_transform_templates.py` adds them to an imported world.db additively (2,411 templates). With it the server logs no unknown-template transforms at startup (10 before).

## [2026-09-30] systems | Start levels, forms on players, transforms, admin socket

Updated [[class-progression]]: TrinityCore start levels for Death Knights and allied races. Players without `ModelDisplay` get their form display and lose it on restore. TRANSFORM with MiscValue 0 keeps the display; an unknown template shows the pink pig 16358. A second server no longer takes a live server's admin socket.

## [2026-09-30] specs | Creature hit table and player avoidance

Updated [[player-damage]] spec: creatures roll the hit table (5% miss, 5% crit) against a player's TC-master avoidance; crits and blocks are sent as `CriticalHit` / `Block`.

## [2026-09-30] systems | Chains, bouncy missiles, spell reflection

Updated [[spell-casting]]: chain targeting with per-jump amplitude, shared target selection for effects with identical targets, `SPELL_ATTR4_BOUNCY_CHAIN_MISSILES` timing, spell reflection and triggered spells ignoring their explicit target's relation.

## [2026-09-30] systems | Missile targets engage at impact

Updated [[combat]]. `CombatRefs` tracks per-side suppression: a spell's launch puts only the caster in combat (TC `SetInCombatWith(target, true)`), the hit engages the target. Admin `set-level` on an online character now refills health and mana (`GiveLevel` → `SetFullHealth`). The trade tests lost their process-global resolution counter, which other trade apps in the parallel suite bumped.

## [2026-09-30] systems | Bounded new-class server slice

Updated [[class-progression]] and [[spell-casting]] for Monk/DH/Evoker creation, start levels, progression and six basic spells; added `docs/specs/new-classes.md`. Local ChrClasses/ChrRaces verified max(class, race) levels (1/8/10 for ordinary Monk/DH/Dracthyr Evoker); locally extracted CharBaseInfo confirmed the supported-race gates, including Void Elf DH. ENERGIZE previously ignored point modifiers, so Tiger Palm gained zero Windwalker Chi. Existing modifiers now supply the authored +2. Additional authored Monk gates cover Kul Tiran 32, Earthen 84/85 and Haranir 86/91, using local race bits/levels and unchanged placeholder placement. Powers ownership, engine UI/live fixtures and delayed starting zones remain separate work.

## [2026-09-30] systems | Spell travel time

Updated [[spell-casting]] and [[combat]]. Spells with a missile `Speed` now land after their flight (TrinityCore `Spell::HandleDelayed`, `spell_cast/missiles.rs`): `SpellGo`, `CastSuccess` and combat at launch; damage, auras, threat, procs and the combat log at the hit. Before, Frostbolt's damage number showed about 0.4 s before the client's missile arrived. Also: periodic ticks start no combat (TC `DealDamage`), so a DoT alone no longer keeps a dummy or PvP fight going, and creatures drop victims more than 103 yd from home 10 s after their last direct damage (TC `CanCreatureAttack`).

## [2026-09-30] systems | Class-resource replication

Updated [[spell-casting]] with fractional power/rate, charged-point and indexed rune send contracts. Targeted RED exposed rates left nonzero for one tick at pool limits and stale ready-rune counts when only max changed. Runic Longevity also changes rune regen, so its full aura masked the latter failure; the count test isolates its max-power effect. End-to-end wire/client rendering remains main's integration gate.

## [2026-09-30] systems | Combat references

Updated [[combat]] (new Combat State section) and [[spell-casting]]. Combat is now TrinityCore's `CombatManager`: pairwise references, no timeout against a creature with a threat list, 5 s for PvP and threat-less creatures (training dummies), and `combat_stop` ends both sides. Before, every unit had its own 5 s timer after its last hit, refreshed every tick while it had `AutoAttack`: a caster auto-attacking a dummy from range stayed in combat 5 s past the dummy's evade (a 10 s tail on one Frostbolt), a kill left 5 s of combat, and a lull over 5 s in a real fight dropped combat. The damage meter's session duration is combat time, so its DPS followed.

## [2026-09-30] systems | Bot economy contract audit

Created [bot-economy spec](../specs/bot-economy.md) and [[bot-economy]] integration inventory. Recorded user-approved auction terms, recipe/expansion constraints, shared finite capacity, bounded consumers, recorded-cost expiry refunds, capped capital and persistence/CLI-first/engine/latency gates. All acceptance remains unchecked: inspected model helpers and candidate tests are not runtime or restart proof. Economy importer and expansion-11/new Silvermoon data coverage remain unestablished. No numerical tuning promoted from fixtures to policy.

## [2026-09-29] systems | Polymorph

Updated [[auras]] (new Crowd control section, procs) and [[creature-ai]] (confused movement). Polymorph 118 now works: TRANSFORM 56 swaps `ModelDisplay` to the Polymorphed Sheep and restores it, MOD_CONFUSE 5 stops swings/casts/target and wanders the creature, 10% of max health in damage breaks it (TrinityCore `HandleBreakableCCAuraProc`), and polymorphed creatures regenerate. Findings: spell hit procs carried no damage, and triggered spells (Frostbolt's missile) never procced; TrinityCore lets `SPELL_ATTR3_NOT_A_PROC`/`CAN_PROC_FROM_PROCS` through. Proc damage must be the level-scaled health loss, or Frostbolt could not break Polymorph on a scaled level-30 Blackrock Spy.

## [2026-09-29] systems | Auto-attack only from AttackSwing

Selecting a hostile unit no longer starts auto-attack or combat. Players attack through `AttackSwing` (TrinityCore `CMSG_ATTACK_SWING` → `Unit::Attack`) and stop through `AttackStop`. The server echoes `AttackStart`/`AttackStopped` to viewers. `AutoAttack` now holds its victim. Creatures still attack through their AI target. Casts of `SPELL_ATTR0_CANCELS_AUTO_ATTACK_COMBAT` spells stop the caster's auto-attack. Updated [[combat]] and [[factions]].

## [2026-09-28] systems | Client-reported player position

Updated [[networking]] and [[tick-model]]: `PlayerInput` carries the client's position and teleport epoch; the server adopts it within speed × movement bank (TrinityCore `HandleMovementOpcode`) instead of re-simulating from the direction, which drifted the server's player from the Godot client's and let mobs hit players from a distance.

## [2026-09-27] systems | Boss scripts

Created [[boss-scripts]]: TrinityCore CreatureAI/BossAI/InstanceScript/CreatureTextMgr port and the Stockade scripts (Hogger, Lord Overheat, Randolph Moloch, Warden Thelwater, Mortimer Moloch). Found that hiding an entity by removing `NetworkVisibility` leaks it to later clients.

## [2026-09-26] systems | Retail world content

Updated [[content-import]], and renamed the spec `retail-spawn-source` to `retail-world-content`. Maps 0/1 now resolve from TrinityCore TDB1210:
- game objects (21,877), vendors, loot, gossip and quests (6,078), on top of creature spawns;
- creature levels are ContentTuning ranges; ContentTuning is extracted from local CASC, and the neighbour-median levels are removed.

A finding: the apparent quest-starter gap was 1,993 quests that Cataclysm removed, so AzerothCore relations are not re-added to Retail NPCs. Server tests now use Retail Northshire's warrior chain.

## [2026-09-26] systems | Retail creature spawns

Updated [[content-import]] and [[world-data]], and created spec `retail-spawn-source`. Maps 0/1 now spawn TrinityCore TDB1210 base-phase spawns: 95,170 at guid + 20,000,000, replacing 60,412 AzerothCore spawns. The server reads `content_creature*`. 4,720 TrinityCore-only templates take their levels from the nearest AzerothCore-template spawns and have no loot, gossip, vendor or quest data. TrinityCore addons, paths, formations and event memberships follow the spawns.

## [2026-09-25] systems | Retail creature displays

Updated [[content-import]] and [[world-data]]. `content_policy.py` builds `content_creature_template_model`: templates with a display absent from Retail `CreatureDisplayInfo` take TrinityCore TDB1210 Retail model rows for the same entry (same name or reviewed `creature` override). Map-0 spawns: 53 templates / 681 spawns moved off 40 Wrath-only displays. `db2.py` imports `creature_display_info`. Server `load_models` reads `content_creature_template_model`.

## [2026-09-24] systems | Experience messages

Updated [[experience]]: owner-only `PlayerXpUpdate`/`LogXpGain` on `ExperienceChannel` and `QuestGiverQuestComplete.xp` (shared-protocol `xp-protocol`). The old-expansion ÷10 kill XP rule is removed because there is no ContentTuning scaling. Rested caps now use gtXp instead of the deleted Wrath `shared::xp::xp_to_level`.

## [2026-09-24] systems | Experience and levelling

Created [[experience]]; updated [[class-progression]] and the index. Kill XP follows TrinityCore master `Formulas.h` + `KillRewarder` over the Retail gtXp table (CASC FDID 1391661), and quest XP follows `Quest::XPValue` over the QuestXP DB2 (12.1.0.69933). Level-ups save `CharacterData.level`, set `Level`/`UnitLevel` (driving spell, stat and trait deltas) and refill health and mana. XP is stored in redb `character_xp`. There is no XP protocol message yet.

## [2026-09-23] systems | NPC faction and reaction

Created [[factions]]; updated [[spell-casting]] (relations from reactions) and the index. NPCs get `UnitFactionTemplate` from `creature_template.faction`, players from `ChrRaces.FactionID`. DB2 `FactionTemplate` imported at 12.1.0.69933; all 657 AzerothCore creature faction IDs exist in Retail. Reputation is not consulted. Reaction rules split into pure `faction_reaction.rs`; replicated `UnitTarget` mirrors player `CombatTarget` and NPC chase target ([[combat]]).

## [2026-09-23] systems | Inventory and equipment networking

Created [[inventory]] and `docs/specs/inventory.md`. Owner-only `InventorySnapshot`/`EquipmentSnapshot` on enter world, `InventoryDelta` after every persisted change (inventory requests, trade, auction/mail, profession). New `inventory_layouts` and `item_instances` tables; equipment stays in `character_equipment`. No item template data: equippability only for admin-granted gear, unknown stack limits, bags 1-4 size 0, no gear stats.

## [2026-09-23] systems | Aura runtime

Created [[auras]]; updated [[spell-casting]] (AuraApplier hook replaced by direct `auras::apply_aura`, effect 38, aura-gated costs, spell mods), [[spell-info]] (`labels` from new `spell_label` DB2 import), index and specs (`docs/specs/auras.md`). 49 aura types handled (36 of the 77-ID 95% cut, 87% of its instances); aura stats fold into `stats::StatModifiers`; stun/silence/pacify fail casts; procs on melee swing, spell hit, periodic, damage taken, kill/death, cast success; 23 SpellModOps; 9 interrupt flags; movement reads `AuraStatMods.movement_speed`. 17 real-DB App tests in `auras/tests.rs`.

## [2026-09-23] systems | Retail trait trees replace the Paladin talent stub

Created [[talents]]; updated [[class-progression]], [[spell-info]], [[persistence]], [[world-data]] and index. The DB2 import adds seven trait link/source tables. Each class `TraitTree` loads into `ClassInfo.traits`. `trait_config` validates configs using TrinityCore rules. Points per level come from `TraitCurrencySource`: at level 80 that is 31 class, 30 spec and 10 per hero currency. `CommitTraitConfig` persists purchases per spec in `character_traits`. `TraitConfigSnapshot` is sent on enter world, spec change and budget change. Talent spells join `KnownSpells`. `talent.rs` and its legacy message handlers are removed.

## [2026-09-23] systems | Retail character stats

Created [[character-stats]] and `docs/specs/character-stats.md`; updated [[spell-casting]] (SP/AP/crit from `CharacterStats`, `PowerCostPct` mana from base mana), [[combat]], the cast spec and the index. Base stats come from SimC `sc_extra_data.inc`; health, mana and rating conversions come from local CASC game tables (hppersta, basemp, combatratings), which match SimC 12.0.1.66709. There is no gear or weapon data. Shared `rating_data.rs` is corrupt and the shared rating conversion is 100× too small, so neither is used for players.

## [2026-09-23] systems | Spell cast pipeline, powers and effects

Created [[spell-casting]]; updated [[spell-info]] (conversion removed, charge categories), [[networking]] (cast section now points to spell casting, `Moving` marker), index and the cast spec (`docs/specs/cast-presentation.md`, now the spell casting contract). Effects 2/6/10/30/31/58/64/68/121/136 handled; 18 implicit target IDs; power pools in raw DB2 units with `power_type` regen. 11 real-DB App tests in `spell_cast/tests.rs`.

## [2026-09-23] systems | Class progression: spec, known spells, action bars

Created [[class-progression]]; updated [[spell-info]], [[persistence]] and index. Race/class validation now runs at creation and select, using a server table because world.db has no chr_races. The Initial spec (OrderIndex 4) is active below level 10. Auto-learned ClassSpells (AcquireMethod 1/2/4, level, race mask), spec spells and mastery form `KnownSpells`, which is snapshotted on world entry with deltas after level or spec changes. Action bars are validated and stored in `character_progression`. Admin set-level updates the online `Level`/`UnitLevel`.

## [2026-09-23] systems | SpellInfo store replaces spell_db

Created [[spell-info]]; updated [[world-data]], [[networking]], [[persistence]], crate-structure, stack-decisions, index and the cast-presentation spec. `SpellStore` loads every spell with all DifficultyID 0 effects, power costs and option tables; `ClassSpells` adds learnables/specs/power types. Conversion fixes interrupt 68, dispel 38, raw power IDs and percent costs. Rage/Runic Power/Soul Shard costs are stored in tenths (kept raw).

## [2026-09-23] systems | Build-pinned DB2 spell/class import

Updated [[world-data]]: `scripts/db2.py` fetches 50 wago.tools tables pinned to Retail `12.1.0.69933` and imports them into `world.db` in one transaction (DDL `scripts/db2_schema.sql`, provenance in `db2_build`). Header/schema mismatch fails with previous rows intact. `python scripts/tests/test_import_azerothcore.py` covers fixtures and real pinned rows. No server code reads the new tables yet.

## [2026-09-11] systems | Authoritative cast presentation

Documented named shared `CastState`, authenticated owner-bound receiver handling, progress/expiry/stop/movement, and engine worker snapshot handoff in [[networking]] and [contract](../specs/cast-presentation.md). Server `08c8634` and shared `d3b0736`: five focused HostClient tests pass (`/tmp/claude/cast-presentation-five-tests.log`, 0.02s execution, 12.84s compile); no spell effects, wire replication, rendered, or deployment claim. Shared source is sibling `shared-protocol`, not stale `crates/shared`.

## [2026-09-09] systems | Persist administrative physical equipment

Updated [[persistence]], [[dev-commands]], `index.md`, and [the equipment contract](../../specs/admin-equipment.md) for server commit `8742c1a`. A trusted plain JSON array grants validated supported slots atomically into the separate `character_equipment` redb table as `InventoryItemData`, while preserving bags, gold, existing gear, and `CharacterData` encoding. The operation allocates item-instance GUIDs in the same transaction, derives/publishes current live appearance, and `list-equipment` reads ownership without writes. This is not a catalog, unequip system, or combat-stat implementation; Theron has not been granted equipment and no runtime/deployment claim is made.

## [2026-09-09] systems | Propagate authoritative NPC names

Updated [[world-data]] and [[networking]] for shared-protocol `c8a06c0` and game-server `fc46567`: `creature_template.name` now becomes replicated UTF-8 `Npc.name`; absent templates fail rather than receiving defaults. SQLite-to-spawn coverage uses `Loup — Écorché`. Engine snapshot delivery and visible rendering remain separate boundaries.

## [2026-09-09] systems | Match server terrain sampling to authored MCVT topology

Updated [[networking]] and its index summary for `2cfe233`. MCNK X/Y offsets, MCVT row/column axes, and four-triangle center-fan interpolation now match the corrected client terrain path. Nineteen focused tests pass. Runtime/cart visual proof remains open.


## [2026-09-09] systems | Hide interest targets before character selection

Updated [[networking]] and `index.md` for server commit `34e7551`. `On<Add, ClientVisibility>` hides existing `NetworkVisibility` targets, and `On<Add, NetworkVisibility>` hides new targets for every replication client rather than only selected players. The real UDP regression proves auth succeeds with zero existing or new NPCs before entry, then proves nearby-only delivery and movement-driven replacement. This is server lifecycle proof only; no client startup or visual-runtime completion claim.

## [2026-09-09] investigation | Inactive Arena Tournament creatures

Updated [[world-data]] for `1963dfb`/`d9784a8`: preserve signed event membership during import and select ordinary/inactive-negative spawns without modifying source coordinates. Exact floating Northshire GUID96194/96198/96204 belong to event31. Documented file-backed loader and real isolated importer regressions, mandatory existing-database update, and absence of event activation/scheduling. Local runtime application remains separate from code proof.

## [2026-09-08] investigation | Apply queued movement once per fixed tick

Updated [[networking]], [[tick-model]], [[tick-loop-design]], and [[tick-loop-audit]] for server commits `7b7dc1a`, `e74b980`, and `4c40480`. Recorded the receiver-boundary defect: each queued client input previously applied a full 50 ms movement delta, so a roughly 60 Hz sender could multiply authoritative displacement in the 20 Hz simulation. `process_player_inputs` now drains each connection, applies the newest continuous input once, preserves any queued jump, and dismounts on any queued swimming input. Real Lightyear host delivery through `MessageReceiver` and production `FixedUpdate` was RED 5 at `7b7dc1a` and GREEN 5 at `e74b980` (`data/diagnostics/receiver-rate/green-e74b980.log`). Native execution and local-server rebuild/restart were not performed.

## [2026-08-10] migration | Document Bevy 0.19 and Lightyear 0.28.0 upgrade

Updated `AGENTS.md`, `CLAUDE.md`, `ARCHITECTURE.md`, [[dev-commands]], [[reference-repos]], [[networking]], [[tick-model]], [[tick-loop-design]], [[stack-decisions]], [[azerothcore-comparison]], [[recurring-loop-elimination]], [[semantic-noop-replication]], [[tick-loop-audit]], and `index.md` for game-server commits `0b1e2e6` and `8b400f5` plus shared-protocol commit `a78c958`. Recorded Rust 1.95, Bevy 0.19, Lightyear 0.28.0, vendored `bevy_replicon` 0.41.1 delegation, global/per-sender dirty candidates, new-sender full sync, mutation ACK/NACK retention, local `NetworkVisibility` plus Lightyear `VisibilityExt`, UDP transport, `ReplicationMetadata` 50 ms cadence, and explicit `StatesPlugin` setup. Current proof includes shared-protocol, server, admin CLI, and seven named connection/replication integration tests. Component-aware entries, exhaustive writer coverage, and live zero-render CPU verification remain partial/unclaimed. Documentation only; no source or dependency changes made by this update.

## [2026-08-10] audit | Reconcile death source split and auto-AFK scan status

Updated [[tick-loop-design]], [[tick-loop-audit]], [[recurring-loop-elimination]], [[tick-model]], [[combat]], [[networking]], and `index.md` for current game-server commits `b4e5cbd`, `2106cd8`, `9412352`, `16768b3`, and `1b7c5a4`. Recorded the player death consumer split into `death/transitions.rs` and focused proof split into `combat_tests/death.rs`; recorded auto-AFK follow-ups and clarified that the broad all-player auto-AFK scan is completed/eliminated while only the wider lower-cadence scheduler invocation remains partial; replaced imprecise due-player wording with due-deadline entity resolution. No live CPU savings claim; no source behavior changed.

## [2026-08-10] investigation | Replace auto-AFK player scans with deadlines

Updated [[recurring-loop-elimination]], [[tick-loop-audit]], [[tick-model]], [[networking]], and `index.md` for server commit `16c821c`. Recorded that `tick_auto_afk` previously visited every player and accumulated idle seconds every `Update`; `AutoAfkScheduler` now stores generation-tagged deadlines, activity and presence boundaries reschedule or invalidate them, and `process_due_auto_afk` resolves only entities referenced by due deadlines. Tests cover timing, activity invalidation, manual DND, lifecycle, stale/despawned deadlines, zero idle-player visits, and duplicate suppression. The broad all-player scan was eliminated; only the wider lower-cadence scheduler invocation remains **PARTIAL** because the deadline heap is still polled every `Update`. No live CPU savings claim; no runtime behavior changed in this documentation update.

## [2026-08-10] audit | Correct recurring-loop completion scope

Reconciled [[tick-loop-audit]], [[recurring-loop-elimination]], [[tick-loop-design]], [[tick-model]], [[networking]], [[combat]], and `index.md` against current source through `18c936b`. Corrected the recurring inventory to 103 registrations / 130 expanded systems (`Update` 85 registrations / 104 systems, `FixedUpdate` 13/21, `PostUpdate` 4/4, `Last` 1/1). Recorded the preserved Lightyear, interest, friends, and terrain requirements as **PARTIAL**, not complete; separated expensive-work reduction from scheduler invocation removal; listed ten remaining definite idle-work systems and the persistence/fixed-order correctness findings; and recorded player death polling as completed by lethal transitions. Documentation reconciliation only; no runtime behavior changed in this update. Current-revision verification is recorded after the documentation commit.

## [2026-08-10] investigation | Gate trade offer lookups on queued requests

Updated [[recurring-loop-elimination]], [[tick-loop-audit]], [[networking]], and `index.md` for server commit `969f2e0`. Recorded the strict scope: only the generic `process_trade_offer_updates` helper used by `ClearTradeItem` and `SetTradeMoney` now checks `has_messages()` before resolving `PlayerLink` → `Player`. Empty receivers skip player lookup; queued requests still drain and apply in order with one player resolution per batch, preserving persistence, trade-session, broadcast/routing, and error behavior. The other six trade receiver systems remain outside this named 7-item slice. RED: `/tmp/claude/game-server/trade-offer-drain-red.log`; initial GREEN harness-timing failure: `/tmp/claude/game-server/trade-offer-drain-green-attempt1.log`; final GREEN: `/tmp/claude/game-server/trade-offer-drain-green.log` (**6 passed**). Formatting/readability artifacts are recorded with the final proof. Audit status is now **7/7 named slices complete**. No live CPU savings claim; no source or runtime behavior changed in this documentation update.

## [2026-08-10] investigation | Drain calendar queries before building snapshot

Updated [[recurring-loop-elimination]], [[tick-loop-audit]], [[networking]], and `index.md` for server commit `18be416`. Recorded that `calendar::process_query_requests` previously built, cloned, and sorted the calendar snapshot before inspecting `QueryCalendar`; it now drains eligible authenticated + `InWorld` receivers first, returns on idle updates, and builds one shared snapshot only when requests exist. All queued empty-payload queries still drain and produce one refresh response per requesting connection. Schedule/signup mutation order and the initial `Added<InWorld>` snapshot path remain unchanged. RED: `/tmp/claude/game-server/calendar-query-drain-red.log`. GREEN: `/tmp/claude/game-server/calendar-query-drain-green.log` (**6 passed**). Formatting/readability artifacts: `/tmp/claude/game-server/calendar-query-drain-fmt.log` and `/tmp/claude/game-server/calendar-query-drain-readability.json`. Audit status is now **6/7 complete**; trade lookup remains open. No live CPU savings claim; no source, tests, or runtime behavior changed in this documentation update.

## [2026-08-10] investigation | Gate durability snapshots on changed players

Updated [[recurring-loop-elimination]], [[tick-loop-audit]], [[networking]], and `index.md` for committed server fix `c46b322`. Recorded the previous every-`Update` per-player `Ref::is_changed()` probe and the new query-level `Changed<EquippedDurability>` gate, while retaining the `is_added()` exclusion for newly inserted durability. RED: `/tmp/claude/game-server/durability-changed-filter-red.log`. GREEN: `/tmp/claude/game-server/durability-changed-filter-green.log` (**6 passed**). Formatting/readability artifacts: `/tmp/claude/game-server/durability-changed-filter-fmt.log` and `/tmp/claude/game-server/durability-changed-filter-readability.json`. Audit status is now **5/7 complete**; calendar and trade remain open. No live CPU savings claim; no source, tests, or runtime behavior changed in this documentation update.

## [2026-08-10] investigation | Drain Who requests before building roster

Updated [[tick-loop-audit]], [[recurring-loop-elimination]], [[networking]], and `index.md` for commit `a4e7aeb`: `who::process_query_requests` now drains `QueryWho` receivers first, produces zero online-roster materializations on repeated idle `Update`s, and builds one shared roster when requests exist. Recorded preserved `Authenticated` + `InWorld` eligibility, latest-request-per-connection behavior, multi-connection routing, filtering/sorting, and snapshot/message/error content. Recorded RED `/tmp/claude/game-server/who-idle-roster-red.log` and `/tmp/claude/game-server/who-request-contract-red.log`, GREEN `/tmp/claude/game-server/who-drain-first-green.log`, and formatting/readability artifacts. Server audit is 4/7 complete: gold, Who, rest, and inactive auto-attacks are complete; durability, calendar, and trade lookup remain open. No live CPU savings claim; no source or runtime behavior changed in this documentation update.

## [2026-08-10] investigation | Replace recurring gold persistence scan

Updated [[persistence]], [[networking]], and [[recurring-loop-elimination]] for commits `4e4c73d`, `c3b3c04`, `9d2067c`, `91bbedd`, and `8920feb`. Documented login-loaded persisted gold; successful barber, completed-trade, auction, and mail-claim saves recording exact values in `GoldUpdateQueue`; same-frame coalescing; `PostUpdate` application before Lightyear buffering; equal-value suppression; offline discard with reload-on-login; profession saves not changing gold; sale proceeds remaining mail until claim; and removal of `sync_player_gold_from_inventory`. Recorded RED/GREEN and readability artifacts under `/tmp/claude/game-server/`. No live CPU or Character-stage claim.

## [2026-08-10] investigation | Gate rest-state evaluation on relevant changes

Updated [[recurring-loop-elimination]], [[networking]], [[tick-loop-audit]], and `index.md` for `fffa86a`: `update_player_rest_state` now evaluates only players without `PlayerRestState` or with changed `Position`/`Level`, instead of scanning every authenticated player and all innkeepers every `Update`. Initial snapshots, enter/leave transitions, and level-based rested-XP recalculation remain intact. Recorded RED `/tmp/claude/game-server/rest-dirty-evaluation-red.log`, GREEN `/tmp/claude/game-server/rest-dirty-evaluation-green.log`, and Rust readability evidence at `/tmp/claude/game-server/rest-dirty-evaluation-readability`. No live CPU or Character-stage claim is made before measurement.

## [2026-08-10] investigation | Remove inactive auto-attack participation

Updated [[combat]], [[recurring-loop-elimination]], and [[tick-loop-audit]] for `152ccbd`: clearing `CombatTarget` now removes `AutoAttack` instead of retaining an inactive component in the fixed combat query. Active swing cadence, combat-grace behavior, damage/death, respawn, and reliable combat events remain unchanged. Recorded RED `/tmp/claude/game-server/autoattack-inactive-removal-red.log`, GREEN `/tmp/claude/game-server/autoattack-inactive-removal-green.log`, and cargo fmt/readability evidence. No live CPU or Character-stage claim is made before measurement.

Chronological record of wiki operations.

## [2026-08-10] audit | Reconcile tick-loop audit schema and persistence summary

Added required `## Content` sections to edited legacy pages [[recurring-loop-elimination]] and [[networking]]. Corrected [[persistence]] from the stale five-minute/whole-period `Changed<T>` description to the source-backed 300-fixed-tick nominal 15-second cadence and confirmed early-return bug. Added cross-links to [[tick-loop-audit]]. No runtime behavior changed.

## [2026-08-10] investigation | Audit every recurring server schedule

Created [[tick-loop-audit]] from a mechanical inventory of 103 recurring registration calls expanding to 130 systems: 105 `Update`, 21 `FixedUpdate`, 3 `PostUpdate`, and 1 `Last`. Confirmed that `save_dirty_characters` loses earlier `Changed<T>` state because its internal 300-tick early returns still advance Bevy's system `last_run`; recorded the Bevy 0.18 source semantics and focused reproduction. Classified definite recurring waste (`sync_player_gold_from_inventory`, Who roster construction, rest scans, durability probing, inactive auto-attacks), correctness-dependent polls, 74 receiver systems, already-eliminated loops, and missing fixed-order consequences. Updated [[tick-loop-design]], [[tick-model]], [[recurring-loop-elimination]], [[networking]], `index.md`, and `ARCHITECTURE.md`. No runtime behavior changed and no production CPU claim is made.

## [2026-08-10] audit | Align tick-loop documentation with source

Audited [[tick-loop-design]], [[tick-model]], [[zone-architecture]], [[open-questions]], `index.md`, and `ARCHITECTURE.md`. Added required `## Content` sections, corrected the root architecture page's stale implemented-zone wording, and confirmed wiki/source links resolve. No runtime behavior changed.

## [2026-08-10] design | Draft explicit game tick-loop contract

Created [[tick-loop-design]] as the reviewable normative design for the server runner, Bevy schedules, fixed simulation phases, Lightyear receive/replication boundaries, bounded catch-up, maintenance cadence, persistence, and admin IPC. Rewrote [[tick-model]] as observed current state: Lightyear sets `Time<Fixed>` to 50 ms, most request handlers run in `Update`, replication buffers in `PostUpdate`, and only local ordering constraints exist. Corrected [[zone-architecture]] to proposed status, updated [[open-questions]] and `index.md`, and linked `ARCHITECTURE.md` to the current/design split. No runtime behavior changed.

## [2026-08-10] investigation | Wake replication for zero-translation movement state

Updated [[recurring-loop-elimination]] and [[networking]] for `aa5dddb`. Recorded the discovered gap where turn-in-place or zero-translation inputs changed replicated `Rotation` or `MovementSpeed` without producing a Lightyear dirty candidate. Documented the fix and `networking_movement.rs` extraction: replication dirtying now covers rotation/speed changes, while `InterestDirtyQueue` remains position-only. Recorded the RED no-op regression and 43-test GREEN focused server proof. No broader CPU or Character claim.

## [2026-08-10] investigation | Replace Lightyear root traversal with dirty candidates

Updated [[recurring-loop-elimination]], [[networking]], [[tick-model]], and `index.md` for `269655a` and `30b5078`. Recorded vendored `lightyear_replication` `0.26.4` provenance, removal of the per-sender 50 ms all-root/all-component traversal, global and sender-targeted dirty candidates, initial full sync, ACK/NACK retention, visibility and component lifecycle triggers, and authoritative writer marks. Recorded test-only candidate/visit diagnostics: the RED idle case measured exactly 160 root visits; GREEN reduced idle candidates and `replicate_entity` visits to zero after initial sync. Recorded in-process integration coverage for changed roots, ACK retention, targeted visibility, component insert/remove, initial sync, and unchanged idle roots. The strict-Empty client CPU gate remains pending; no Character advancement or `<=10%` CPU claim is made.

## [2026-08-09] investigation | Replace recurring interest scan with dirty spatial updates

Updated [[recurring-loop-elimination]] and [[networking]] for commits `4523373` and `a85b035`. The former removed the every-500ms `NetworkVisibility`-entities × players scan: authoritative position and lifecycle triggers now dirty observers/targets, a 100-yard X/Z spatial index narrows candidates, and exact 3D `<= 100²` visibility transitions remain per connection. The latter split the real fixed-tick integration tests into `interest_tests.rs` without runtime changes. Recorded proof for unchanged ticks, new targets, same-bucket radius crossings, independent players, removal/re-add, reconnect, spawn/despawn, teleports, boundaries, corpses, and other preserved visibility behavior. Lightyear's 50ms per-sender traversal remains the only pending recurring-loop slice; this update makes no broader Empty-completion claim.

## [2026-08-09] investigation | Cache and dirty friend snapshots

Updated [[recurring-loop-elimination]] for commit `914d570`: cached online presence, reverse `owners_by_friend` links, and dirty-owner fanout replace every-frame all-owner snapshot rebuilding and offline database lookups. Recorded behavioral proof for unchanged offline data, affected-only presence, login/logout, zone/level changes, and friend-list reverse-index replacement. Privacy and ignore state are not friend-snapshot inputs and remain unchanged.

## [2026-08-09] investigation | Gate terrain streaming on tile transitions

Updated [[recurring-loop-elimination]], [[tick-model]], and [[networking]] for commit `3c5ceca`. Recorded the behavioral RED/GREEN boundary: same-tile movement previously dirtied `SentTiles` through repeated 3×3 duplicate insert attempts; it now preserves the component tick, while initial entry, reconnect, boundary crossing, and teleport semantics remain active.

## [2026-08-09] investigation | Gate auction expiration to one second

Created [[recurring-loop-elimination]] and updated [[tick-model]] for commit `e02ce0a`. Recorded the scheduler-boundary RED/GREEN proof: the first due auction expires immediately, repeated sub-second fixed ticks skip expiration, and the next scan occurs at the one-second boundary while existing persistence, sale, and mail behavior remains unchanged. A deadline heap was intentionally excluded as unnecessary complexity.

## [2026-08-09] investigation | Record direct NPC death transition

Updated [[combat]], [[tick-model]], [[networking]], and [[semantic-noop-replication]] for commit `c8762ed`: NPC death now occurs at the sole authoritative `try_swing` damage boundary; the 20 Hz all-living-NPC health scan was removed; nonlethal, lethal, and zero-health-without-transition behavior are covered; respawn and reliable `CombatEvent` semantics remain; future NPC damage writers must use the same death transition. Updated `index.md` and cross-links; no source or tests changed.

## [2026-08-09] investigation | Record second semantic no-op proof wave

Updated [[semantic-noop-replication]], [[networking]], and `index.md` for second-wave RED/GREEN commits `1c139e9` (equal derived `Health`/`Mana`), `e13e09e` (equal respawn `Health`/`Position`), `057dd17` (equal equipment appearance), and `41feebb` (equal guild membership). `f84ee17` adds the independently observed equipment-clear RED/GREEN regression. Recorded `945eb0f` as Gold synchronization already guarded and ruled out. Preserved first-wave commits and marked admin teleport, manual presence, mount-triggered speed, and other event-driven writers as remaining unproven. The Lightyear serialized-update boundary and live component frequency/byte measurement remain blocked or unmeasured; no CPU, bandwidth, or wire-count improvement claim is made.

## [2026-08-09] investigation | Document semantic no-op replication

Created [[semantic-noop-replication]] and updated [[networking]] for the completed investigation and fixes in commits `86d73f5`, `74edc39`, `f33c95a`, and `bd31161`. Recorded Lightyear `0.26.4` sender tick-only selection, 50 ms `SinceLastAck` cadence, client equality suppression after wire cost, seven RED change-tick failures, preserved real behavior, and the blocked direct serialized-update integration proof/live bandwidth measurement. No CPU or network improvement claim.

## [2026-08-07] investigation | Legacy inventory rows required eager encoding migration

Updated `systems/persistence.md` for commit `01eaf52`: production rows 6 and 7 used the pre-April 6, 2026 `InventoryItemData` encoding without `unique_auction`; startup now rewrites recognized legacy bytes with `unique_auction = false` and fails explicitly for unknown encodings instead of retaining a permanent load fallback.

## [2026-08-07] investigation | Missing `pvp_states` table caused character-selection restart loop

Updated `systems/persistence.md` with the root cause and fix from commit `ec5ff52`: `create_tables()` omitted `PVP_STATES`, so existing `game.redb` databases caused character selection to panic. `run-dev.sh` restarted the server, the client re-entered `InWorld`, and repeated rendering initialization eventually reached GPU OOM. Added the legacy-database regression-test reference.

## [2026-04-09] ingest | Systems wiki build from ARCHITECTURE.md, CLAUDE.md, AGENTS.md + crate source exploration

Created 8 systems pages:
- `systems/ecs-storage.md` — archetype tables, columnar layout, zero-overhead queries, Changed<T>
- `systems/networking.md` — Lightyear/QUIC, replication, PlayerInput, channels, CombatLog
- `systems/persistence.md` — redb tables, save triggers, what is/isn't persisted, SQLite distinction
- `systems/world-data.md` — all SQLite tables with row counts, movement type enum, chr_races
- `systems/tick-model.md` — 20Hz FixedUpdate pipeline steps, zone-parallel scheduling
- `systems/zone-architecture.md` — system sets, Zone component, zone transfers, AzerothCore comparison
- `systems/combat.md` — CombatPlugin systems, melee hit table, spell/stat formulas, DR curves
- `systems/creature-ai.md` — movement types, WanderState, WaypointState, chase, leash, evade

Updated index.md with systems/ table.

## [2026-04-09] ingest | Initial ingest from ARCHITECTURE.md, CLAUDE.md, AGENTS.md

Bootstrapped wiki from the three root documentation files. Created pages across systems/, design/, and reference/ categories.

## [2026-04-09] ingest | Full wiki build from ARCHITECTURE.md, CLAUDE.md, AGENTS.md, CODE_INDEX_TRIAGE.md + crate exploration

Created 7 pages and index.md:
- `design/stack-decisions.md` — Bevy/Lightyear/redb rationale, SpacetimeDB rejection
- `design/azerothcore-comparison.md` — full side-by-side comparison table
- `design/open-questions.md` — tick rate, world size, instances, hot-reloading
- `reference/reference-repos.md` — AzerothCore, Rustbolt Core, SpacetimeDB, wow-engine
- `reference/dev-commands.md` — cargo commands, admin CLI, world data import, dependency versions
- `reference/crate-structure.md` — shared/server/client/integration-tests with module listings from actual source tree
- `reference/code-health.md` — 4,838 symbols, 88 untested candidates, dead-code guidance
- `index.md` — content catalog

## [2026-09-23] ingest | AzerothCore content import + ID mapping

Created `systems/content-import.md`: 25 content tables imported via new `scripts/mysql_dump.py` (escape-correct loader, replaces awk/sed); Wrath→Retail mapping measured by `scripts/content_mapping.py`. Updated `systems/world-data.md`, `index.md`.

## [2026-09-23] update | Retail-first content policy

`systems/content-import.md`: policy section (content_* tables, rules, kept/dropped counts, open follow-ups); CSVs moved to the db2.py root. `systems/world-data.md`: new item/quest/taxi DB2 tables.

## [2026-09-24] update | Retail mana regen

[[spell-casting]]: mana regen is 1% of base mana per second scaled by aura regen multipliers (placeholder removed); numbers per spec.

## [2026-09-24] update | Equipped gear feeds character stats

`systems/character-stats.md`: Gear section (item stat/armor/weapon formulas, squish, tooltip checks), gaps. `systems/inventory.md`: `EquippedItems` sync. `systems/spell-casting.md`: weapon hit uses main-hand damage.

## [2026-09-24] create | Quest runtime

Created `systems/quests.md` (quest data, rules, runtime) and `specs/quests.md`; index entry added.

## [2026-09-24] create | NPC interaction

[[npc-interaction]]: NpcFlags replication, InteractNpc/SelectGossipOption/CloseInteraction, gossip menus, ActiveInteraction session; auction requests require an auctioneer session and use its faction house.

## [2026-09-24] update | NPC patrol paths, formations, creature addons

[[creature-ai]]: MovementType 2 walks creature_addon/creature_template_addon `path_id` (not guid), delays in ms, run/walk per move_type, point orientation; formation followers; addon mount and auras. [[world-data]]: creature_addon, creature_template_addon, creature_formations tables.

## [2026-09-24] update | Patrol death and respawn

[[creature-ai]]: dead creatures stop wandering/patrolling/following; patrollers respawn at spawn and restart their path.

## [2026-09-24] update | Every spawn respawns

[[creature-ai]]: all spawns get SpawnPoint + RespawnDelay (spawntimesecs + corpse decay by rank); escorts rejoin respawned leaders.

## [2026-09-24] update | Grid activation

[[creature-ai]]: every map 0 creature spawns; creatures more than one 533.33 yd grid from every player carry `GridDormant` and skip AI and aura updates; respawn timers keep running. [[zone-architecture]], [[tick-model]]: map 0 spawning, FixedPreUpdate grid step.

## [2026-09-24] create | Merchant

[[merchant]]: vendor list/buy/sell/buyback/repair (AzerothCore rules), per-item durability from item_template.MaxDurability with Retail DurabilityCosts/DurabilityQuality repair costs; [[content-import]] db2.py imports durability_costs/durability_quality.

## [2026-09-25] create | Banks

[[banks]]: character bank and Warband bank at bankers (Retail BankTab prices), guild bank at Guild Vaults with default ranks, per-rank tab rights, daily reset and logs; [[game-objects]]: type 34 vault spawns with GeoBox reach; [[content-import]] imports gameobject tables, GameObjectDisplayInfo and BankTab.

## [2026-09-25] create | Professions

[[professions]]: PlayerProfessions (skill lines + spells) replaces the AzerothCore-era placeholder; TDB1210 trainers (TrainerList/TrainerBuySpell), LEARN_SPELL/SKILL_STEP/CREATE_ITEM handlers, CraftRecipe queue, SpellReagents/SpellTotems checks. [[content-import]]: tdb_* tables, SpellReagents/SpellTotems/TradeSkillCategory/SkillRaceClassInfo/TotemCategory.

## [2026-09-26] create | Mail

[[mail]]: player mail at type-19 Mailboxes (postage, C.O.D., 1 h cross-account item delay, 30/3-day expiry, return, PendingMail), Auction House results delivered as real mail (legacy `mailboxes` migrated); [[game-objects]]: mailbox spawns.

## [2026-09-26] update | Trade

[[trade]]: exchange through InventoryModel in one transaction (bag space, gold cap, soulbound), 7th non-traded slot, CancelTradeAccept, AzerothCore initiation checks, Retail messages.

## [2026-09-25] create | Creature aggro

[[creature-aggro]]: AggroMode per template (passive/reactive/aggressive), level-scaled aggro radius (GetAggroRange), threat from melee/spell damage and harmful spell hits, top-threat victim with 110%/130% swap, evade with combat_stop and full health; [[creature-ai]]: chase and return at run speed, 2 yd chase reach, leash evade through enter_evade_mode; [[combat]]: NPC CombatTarget now set by aggro.

## [2026-09-26] create | Level scaling

[[level-scaling]]: tuned creatures spawn at their ContentTuning maximum with ExpectedStat health, and are fought at each unit's clamped level through TrinityCore's damage and health multipliers. XP and aggro use the scaled level, and the target frame shows it with a difficulty colour. [[world-data]]: expected_stat tables. [[experience]], [[creature-aggro]] and [[combat]] link it.

## [2026-09-26] update | Quest money per player

[[content-import]]: content_tuning_level keeps GetContentTuningData levels (MinLevelWithDelta, ScalingFactionGroup) for the server. TrinityCore quests now pay and offer Quest::MoneyValue at the player's quest level, not MaxMoneyValue: Beating Them Back! pays 55 at level 1 ([retail-world-content](../specs/retail-world-content.md)).

## [2026-09-26] create | Player damage

[[player-damage]]: players swing their weapon (AP / 6 × speed, off hand at half) every weapon speed over haste, through TrinityCore's hit table and ExpectedStat creature armor, before level scaling; spell power only from an Intellect main stat, spell variance and versatility, armor on physical spells, TRIGGER_MISSILE. [[combat]], [[character-stats]] and [[level-scaling]] link it.

## [2026-09-26] create | Meeting stones

[[meeting-stones]]: party summons at the 15 map-0 meeting stones (59782 cast and channel, Meeting Stone Summoning Portal ritual, 7720 SUMMON_PLAYER, 2 min offer, accept teleport). [[spell-casting]]: cast/channel movement now follows `SpellInterrupts` (Spell::CheckMovement) instead of `SPELL_ATTR0` 0x10000000, and channels start after their cast time. [[game-objects]] links it.

## [2026-09-26] update | Merchant cursor buys and Sell All Junk

[[merchant]]: `BuyItem.destination` stores a purchase in the bag slot the client's merchant cursor was dropped on (TrinityCore `CanStoreNewItem(bag, slot)`: same-item stacks fill, the rest spills, another item refuses `CantStack`); `SellAllJunkItems` sells every poor bag stack through the ordinary sale path ([merchant spec](../specs/merchant.md)). `InventoryErrorReason::message` gives the client Retail `ERR_*` text.

## [2026-09-26] update | Maps and dungeon instances

[[zone-architecture]]: maps 0 and 1 and dungeon copies share one World by `InMap`; `maps` module (MapStore, InstanceManager, teleport_to/NewWorld/WorldPortAck, area trigger teleports, dungeon release, login map). Spec [instances](../specs/instances.md). [[meeting-stones]]: summons cross maps. [[game-objects]]: spawned per map copy.

## [2026-09-27] update | Wander follows the ground

[[creature-ai]]: wandering creatures step on terrain/WMO floors via `GroundMap`, refuse ledges and unsupported destinations, and the wander LCG advances its state (Stormwind Rats floating over the canal / above their floor).

## [2026-09-27] update | Dungeon difficulties and instance locks

[[zone-architecture]]: copies at the entrant's dungeon difficulty (downscaled per MapDifficulty/Difficulty fallback), `InstanceLocks` (InstanceLockMgr port: temporary/permanent, instance-id bound shared state, encounter locks, Retail 15:00 UTC reset), lock persistence in game.redb, `ServerClock`. Spec [instances](../specs/instances.md) (Dungeon difficulty, Instance locks), [retail world content](../specs/retail-world-content.md) (spawnDifficulties, content_creature_template_difficulty, Grim Batol), [boss scripts](../specs/boss-scripts.md) (Grim Batol spell-less BossAI).

## [2026-09-28] update | Dungeon level conditions

[[zone-architecture]]: `MapStore.difficulty_conditions` (MapDifficultyXCondition → PlayerCondition → `content_tuning_level`); `player_cannot_enter` refuses a player below a dungeon's ContentTuning minimum with `TransferAbortReason::Condition` (Stockade: level 10). Spec [instances](../specs/instances.md) (Level conditions).

## [2026-09-28] create | Swimming and breath

[[swimming-and-breath]]: the server reads MH2O terrain liquid and adopts the client's height ([player height](../specs/player-height.md)): swimmers within `SWIM_SPEED` × the bank between the seabed and `swim_top`, land heights between the ground and a jump/fall arc ceiling, no gravity on input ticks or for swimmers. TrinityCore mirror timers: breath (180 s, refill ×10), fatigue on MH2O deep water (60 s), drowning/exhaustion 20% every second, water breathing auras 82/155, stop on death ([breath and drowning](../specs/breath-and-drowning.md)). [[networking]] and [[auras]] updated.

## [2026-09-28] update | Spec spells by level, pushed action bar spells

[[class-progression]]: spec spells wait for their `SpellLevel` like TrinityCore `LearnSpecializationSpells`, and newly learned actives fill the first empty main bar slots (Retail `SPELL_PUSHED_TO_ACTIONBAR`).

## [2026-09-29] investigation | Drowning on :5000

[[swimming-and-breath]]: no drowning damage in the live fixture was server clock lag (4 000 ticks in 326 s), not the timer, damage or replication path; a private-instance run drowned every second and replicated. Regression test through character select added.

## [2026-09-29] investigation | Server tick lag

[[server-tick-lag]]: the debug :5000 server fell behind real time because frames over 50 ms (and over the 250 ms clamp under load) lost simulation time. Frame/system instrumentation (`frame_stats`, `system-timings` feature), dormant-creature filters for power, UnitTarget and regen, cached aura queries, aggro reach prefilter, one-transaction character saves, Entity-hashed interest maps. Debug with three clients: 36–64 ms → 15–29 ms per frame; release 0.8 ms idle.

## [2026-09-29] feature | Replicated player motion

Players replicate `PlayerMotion`, Retail `MovementFlags` (FORWARD, BACKWARD, STRAFE_LEFT/RIGHT, WALKING, FALLING, SWIMMING), set from each applied `PlayerInput` and written only on change; the stop input clears the moving bits. The Godot client animates other players from it (walk/run/strafe/backpedal/swim/jump). See [[networking]], spec [player-motion](../specs/player-motion.md).

## [2026-09-29] fix | Frostbolt's Chilled slow

[[spell-casting]]: spell scripts (`on_hit`) added; Frostbolt 116 casts Chilled 205708 on its hit unit, as TrinityCore `spell_mage_frostbolt`. Before, no aura followed a Frostbolt: neither 116 nor 228597 carries MOD_DECREASE_SPEED. Tests in `auras/mage_tests.rs` also cover Polymorph's replicated debuff and Arcane Intellect's +3% Intellect.

## [2026-09-29] fix | Debuffs by effect positivity

[[auras]]: an aura's harmful flag follows TrinityCore `_InitFlags` and ported effect positivity instead of caster hostility; Polymorph on the neutral Blackrock Spy replicated as a buff before.

## [2026-09-30] feature | Class secondary resources

[[spell-casting]], [[auras]]: aura max power (35/418; Holy Power 5, Chi 5/6), druid forms switch display power and model (MOD_SHAPESHIFT 36), Death Knight runes as per-rune cooldowns with 3 recharging, SET_POWER_POINT_CHARGE 494 charged combo points. Monk, Demon Hunter and Evoker remain unplayable, so Chi and Essence have no player yet.

## [2026-10-01] fix | Player health regeneration

[[spell-casting]], [[experience]]: players regenerate health every 2 s out of combat (TrinityCore `Player::RegenerateHealth`); before, only creatures did, so a damaged player stayed damaged forever (observed 666/1110 and 1109/1110). Level-up fills `SetToMaxOnLevelUp` pools (adds Energy). A max-health rescale truncates to whole points. Live: Godot client on a private server, 666 → 1110 in 20 ticks of 22/23.

## [2026-10-01] feature | Player stand state, food and drink

[[stand-state]], [[spell-casting]], [[auras]]: players sit, sleep, kneel and take chairs (`PlayerStandState`, replicated); moving, hits, hostile spells, casting and death stand them up; food and drink sit a standing eater and end on standing; sitting regenerates ×1.5 and is crit by every melee swing that lands. Health regen auras 84/88/116/161 and gear health regen; SpellScaling effect amounts; OBS_MOD_POWER drink ticks. Item use builds on questsrv 22cf305 and adds the item category cooldown, peaceful-only refusal in combat and expendable consumption.

## [2026-10-01] feature | New-character kit

[[new-character-kit]]: new characters get the TrinityCore a352b1fa `Player::Create` kit. It places CharacterLoadout Purpose 9 items (with playercreateinfo_item overrides) through `StoreNewItemInBestSlots` and the backpack second pass, and stores the playercreateinfo_action buttons. [[class-progression]] now teaches every default skill line's spells (LearnDefaultSkills) instead of only racial lines, and the server-invented default bar is gone. `ItemTemplate::allows` now checks PlayableRaceBit. Gaps: kit bags, jewelry and second one-handers stay in the backpack (no bag containers, no jewelry storage, no dual wield), and level-10 starts lose create buttons to spec resolution and pruning.

## [2026-10-01] docs | Zaralda paired-protocol and renderer boundary (task date)

Updated [[midnight-economy-content]], [[economy-catalog]] and index: historical 0/3 retained; paired jc35z10fr8/current server shared7ba6223 reached auth/InWorld, then genuine appearance run hit model binding failure. Linked engine model/gear SSOT for appearance preservation/geoset caveat and 016b0fce/207aea4e CPU-only fix evidence. New native build/live-window gate pending main; catalog/CLI proof does not close overall goal. Evidence task date retained, no new runtime verification or docs-update tests/builds/commits.

## 2026-10-01 — Zaralda derived-friendly/native boundary (evidence date)

Recorded supplied d4708c9/6aa2e27 SQL/rebuild receipts and approved Zaralda-only derived faction policy; linked manifest/spec/publication SSOT. Native faction-0 attack supersedes earlier pick blocker; independent scope gate and native faction-35 acceptance remain pending. Evidence dated October 1, not a new runtime verification timestamp. Docs-only update; no code/data/PLAN changes, commits, builds, tests or service actions.

## 2026-10-01 — Zaralda final observed-proof reconciliation (task evidence)

Updated existing Zaralda SSOT and linked specs/reference/index: native-friendly exit0 / 529.10s now observes actual merchant pick/title/tooltip/backpack/close. Retained historical RED attempts, independent gate114 source/import proof and renderer/cache limits; gate116 artifact audit gives scoped PASS for the saved native flow and three current AH receipts. Server SSOT separates current three AH receipts from pinned old-binary performance. Prior uncommitted docs preserved; no tests, builds, commits, data/source/PLAN edits or service operations.


## [2026-10-03] docs | Native desktop/local workflow

[Workflow SSOT](../remote-builds.md) and [build-host contract](../specs/build-host.md) document selected-host normal runs, filtered checks/tests, native dependencies, lifetime and state boundaries. Reported build evidence remains separate from pending runtime acceptance; no new tests or runtime proof.

## 2026-10-05 — Rested XP administration

Updated [[experience]] for persisted accumulation and kill bonus consumption; documented capped online/offline `set-rested` and owner updates. Corrected `LogXpGain.original` versus base `amount`.

## 2026-10-06 — Dungeon achievements phase 1

Added [[dungeon-achievements]]: local CASC provenance, partial encrypted-section coverage, supported DB2 criteria/tree semantics, Stockade full-run boundary, character/lock persistence, and client phase-2 contract. Targeted proof: nine server tests PASS (eight at 702cb80, Stockade frame-drain fixture at 96cad53), Python virtual-string/decrypted-section test PASS, focused native cargo check and cargo fmt check PASS. Normal checkpoints now persist alongside scheduled locks without imposing scheduled reset restrictions.
## [2026-10-05] systems | Player-managed guild ranks

Updated [[banks]] and added the [guild-ranks contract](../specs/guild-ranks.md): GM controls, strict hierarchy, per-rank social/bank rights, atomic member remapping and layout-2 migration. Targeted RED failed for missing rank APIs. GREEN at `26b1264`: 37 guild-related server tests and one admin test pass; protocol wire test passes at `fe5ea8d`. Additional REDs reproduced cross-guild chat with stale membership components and erased masked tab settings; both now pass. No deployment or native settings UI proof.

- 2026-10-06: Investigated Algari zero-heal consumption; [[item-healing-scaling]] records item-level RandPropPoints root cause, cast/aura context fix, and consumable regressions.

## [2026-10-05] docs | Preserve Retail / isolate Forever audit

Reconciled [[skyborne]], index and source/race contracts against tracked factory, GUID-marker, scaling and character-ABI code plus retained proof ledgers. Supersedes stale pre-isolation kit failure as current state with bounded `e405232` ten-pair creation/reopen proof; preserves historical broad-suite failure and the 26-pass/exit-101 ABI invocation boundary. Immutable provenance across ownership/reload and frozen disk ABI are distinct from explicit wire fields. Transfer integration, unowned content provenance, bare-ID auction search, Classic mechanics and paired native E2E remain open. Docs-only; no new tests, builds, operations or full acceptance.

## 2026-10-05 — Bounded Skyborne source-gap audit

[[skyborne#Native entry and NPC startup source gap (2026-10-05)]] records Main-reported two-race entry, zero NPC startup, the real 1,662-spawn class0 panic and restored allowlist/explicitly ignored regression. User retains authored health/class blocker; no invented Warrior/noncombat contract. [[inventory#Bounded owned-commerce and requirement proof (2026-10-05)]] records commerce4/4 and requirements11/11 with exact scope; buyback is live-session only, not whole parity.

## [2026-10-05] systems | Record estimated-stat startup and bounded native evidence

[Content import](systems/content-import.md#forever-zephras-overlay) owns approved health/unit-class estimates, actual startup and fresh whole-workspace proof at c5cf88a; [Skyborne](systems/skyborne.md#native-entry-and-npc-startup-source-gap-2026-10-05) links bounded native evidence. Historical missing-stat panic/ignored regression preserved; no authored/full Classic or complete gameplay claim. Native artifact gate pending.

## [2026-10-07] systems | Record matching scratch native material failure

[skyborne](systems/skyborne.md) checkpoint updated from supplied evidence: server Retail-only loader3/3 and independent bounded gate; scratch prerequisite provenance retains AreaTable build UNVERIFIED and encrypted achievement gaps. Matching world entry preserves character30/40XP/log[]; Ailee12 textured batches precede Batch94/geoset3601 empty base_texture FAIL, Grove not reached. Diagnostic assertions unchanged; batch-binding investigation pending. Historical scopes retained; docs only, no runtime/test/build/push actions.

## [2026-10-07] systems | Reconcile current final checkpoint; goal OPEN

[skyborne](systems/skyborne.md) reconciles current independent proof and later traced native evidence: corrected type20 composition is separate from unresolved Ailee type7/full-native acceptance. Records separate shoulder scope, pending Grove291, import278 preservation/provenance limits and cleanup exclusions. Historical failures retained; prior CASC root cause unproved. Overall goal OPEN. Docs-only; no code/data/tests/build/delegation/ops/push.

## [2026-10-07] audit | Reported motion fixture correction

Updated [[skyborne]] with wrong-cell root cause, stronger first-tick landing assertions, pushed test-only a7616cd and retained independent bounded proof. Linked engine-owned remote handoff and current NPC RED; no new tests, operations or full-goal claim.

- 2026-10-07 — Reconcile bounded corrective742 native Ailee/Grove functional DONE, mapped/build receipts, CPU13 and pending304; retain OS-exit/pixel/parity/provenance exclusions. Record final handoff metadata hash proof separately from unproved consumption/landing. Docs-only; no runtime or test operations.

## 2026-10-08 — Character-bank item moves

Existing ItemLocation/SwapItem/SplitItem extended to character bank tabs; [bank system](systems/banks.md) records native cursor and transactional server behavior. Targeted proof tracked in `/home/osso/.worktrees/handoff-bankmoves.md`; no live run.
- 2026-10-08: auction subcategory alternatives and Bid/Quantity ordering;4 targeted RED/GREEN paging/filter/source-tie tests. Updated auction-house spec/system. No live/broad gates.

## [2026-10-09] docs | Landed-state reconciliation

Added [deployment SSOT](../deploy.md) from `0dcaa23`/`6bfd7b6`; linked development commands without duplicating unit/state/build policy. Reconciled BankChannel versus InventoryChannel closed-bank responses (`8ac442f`, merge `104ae6a`) in [[banks]] and its spec. Indexed existing auction ServerClock/escrow proof (`9927e77`, `ab4cd69`, merge `137c462`) and defensive fixes/limits (`e86c038`, gate `9f54920`, merge `c5432a4`) rather than copying their ledgers. Docs-only inspection; no build, deployment, service or live validation.

- 2026-10-10: Added [navigation](systems/navigation.md) bake/profile contract and shared collision/global-WMO collector; runtime/AI proof pending.

- 2026-10-10: Navigation query and chase/home/waypoint/repath/evade proven on real Stockade; corrected M2 obstacle support and modern LiquidObject42 parsing. Existing map bake and Stormwind acceptance remain active.

- 2026-10-10: Completed1252 navigation tiles for existing LOS maps0/2991 plus Stockade34, zero bake input gaps; recorded6,323,034,020bytes and lead-owned release/shipping boundaries. Native96-test proof, exact-floor movement assertions, and9/9 representative byte-equivalence evidence preserved.

- 2026-10-10: Restored authentic TDB1210.26091 disenchant loot excerpt and importer/schema; reference-loot full-row equality and canonical backup/import verified. [Content import](systems/content-import.md#disenchant-source-restoration-2026-10-10). Retail zero-skill metadata and absent item-target/error contract block gameplay continuation pending decisions.

- 2026-10-10: Imported established TDB1210.26091 spell_target_position; documented prefixed loader, OrderIndex0 fixed destinations and nullable Retail effect facing in [[spell-casting]].

## 2026-10-10 — class Dummy script coverage

Updated [spell casting](systems/spell-casting.md#class-dummy-scripts), [aura hooks](systems/auras.md#script-hooks) and [contract](../specs/spell-dummy-scripts.md): six real-ID class-script paths, twelve RED/GREEN cast tests and nine bounded channel regressions. Source/data citations and unsupported variants remain explicit. The 40-spec matrix retains unresolved markers; dispatch is not full gameplay parity.

- 2026-10-10 — [Class scripts](systems/spell-casting.md#class-dummy-scripts): Cobra Shot cooldown/recharge contract and real-charge cast fixture.

- 2026-10-10 — [Class spell scripts batch2](systems/spell-casting.md#class-dummy-scripts): nine classes/nine core spell paths, 24 new real-ID cast tests; original12 plus new24 pass together. Gap roots184→180; E3 admissible125/133, a4=57/65, a226=14/15. Authored low-level damage variables331/339 resolved; conditional ranks, charge/normal timer updates, target-priority jumps, death shard, Undead validation and live armor. Stats27/defensives24/channel9 regressions, format/check pass; independent verifier unavailable (expired OAuth). No realm/deployment/merge claim.

- 2026-10-10 — [Class spell batch4](systems/spell-casting.md#fourth-batch-local-retail-rows): eight ranked candidates plus supplementary Vivify/Eruption paths;17 new functions,69 surviving Dummy cast proofs. Mainline radial Light of Dawn, strict Kill Shot gate, owned Renewing Mist targeting and Ebon extension; only53351 fully closes its Dummy root. Gap roots177→176, E3=121/129, a4=57/65, a226=14/15. Local raw DB2 receipts match24 IDs; native format/check pass, no live realm or independent gate claim.

- 2026-10-10 — [Toy effects5](systems/toys.md#audited-attachment-and-sound-toys-2026-10-10): eight audited visual-only attachment/sound toys, actual learn/use/target/cast/cancel/expiry proof, Horde cloak restriction and unbound ring refusals. Census95→103/1166;193 spell_cast regressions pass; no database writes, client rendering claim or merge.
- 2026-10-10 — [Class spell batch5](systems/spell-casting.md#fifth-batch-local-retail-rows): eight ranked paths plus Lava Burst,14 new functions; requested full spell_cast filter207/207 once, format/check exit0. Dummy roots176→173, E3=118/126;56 reviewed/partial/deferred and117 un-derived. Raw34-ID local DB2 receipts, frozen census sources; Crash Lightning selector104 deferred without substitute. No realm/deployment/merge or independent gate claim.

- 2026-10-10 — [Source-authored starts](systems/new-character-kit.md#starting-location): replace rounded race-only coordinates with TDB1210 race/class map/position/facing; preserve explicit unimplemented-intro and Skyborne exceptions, reject missing rows and retain existing saved positions. Tauren oldZ53 was6.429yd below real Mulgore terrain. Pinned importer refuses live world.db and retains NPE metadata; [contract](../specs/player-height.md).

- 2026-10-10: Guardian follow-up binds native205180/103673→205231; prepared-copy template/model and native actor timing/live existing demon-SP proofs. Local inheritance/stamina/melee and other named bindings remain underivable. Census164 roots94/70, Affliction script presence3→4/root A unchanged; see [coverage](../spell-guardians2-coverage.md) and [runtime](systems/spell-casting.md#guardian-source-follow-ups).

- 2026-10-10: Guardian final integration25bc1fc retains batch15/petbattle-effects3. Current census163 roots99/64; all40 batch15 baseline exactly reproduced, only Affliction script3→4/root A unchanged. Single earlier352/352 spell_cast gate retained; bounded post-master22/22 plus fmt/check. [Coverage](../spell-guardians2-coverage.md).

- 2026-10-11: [Class spell batch22](systems/spell-casting.md#twenty-second-batch-native-rewind-non-raid-healing): Rewind authored non-raid healing scalar, Mind Flay native count certification and12 exact source reviews. Census157/117/40→157/129/28; ten targeted tests GREEN after real Rewind RED, no broad gate per lead. [Source/proof limits](../spell-dummy22-coverage.md).

- 2026-10-10: [Stranglethorn ground](investigations/stranglethorn-ground.md): legacy underground audit destination reproduced; corrected Retail coordinates; real-tile gravity/hub proof and all-map WDT root census.

## 2026-10-11 — raid-entry

Added [raid system](systems/raid-entry.md) and contract reconciliation: source-derived single-creature credit, native both-skin private entry/saves/reset/extension proof, and stale terrain-hint ownership fix. Full raid mechanics/data coverage remains incomplete.

## 2026-10-11 — Gathering branch (acceptance incomplete)

[Gathering](systems/gathering.md): local Lock/TDB skin metadata import, GO50 namespace repair, cast/loot/progression, authored pools/depletion/respawn and reopen persistence. Ten behavioral tests; both-skin native acceptance and independent gate remain open. Lead applied world import; agents did not write world.db.

## 2026-10-11 — gathering corpse loot readiness

- [Gathering](systems/gathering.md#corpse-loot-readiness): native corpse-before-loot ordering reproduced; completed empty normal rolls now remain distinguishable from unrolled corpses. Targeted regression and both-skin native recheck owned by gathering-live.
## 2026-10-11 — Cobrahn encounter

Bounded local-data Wailing Caverns SmartAI lifecycle and aggro dialogue documented in [[boss-scripts]]. Behavioral RED exposed absent encounter state. Both-skin native entry/pull/death/tracker credit inspected;20 regression passes, one explicit population-copy loot gate fails due lost authored binding. Actor4213 and physical portal traversal remain unproved; full playable acceptance is blocked, not complete.

## 2026-10-11 — Scenarios

Added local DB2/TDB importer, active-tree copy-scoped achievement-evaluator reuse and stage snapshots. Native proof/fixture boundary lives in [shared system SSOT](../../../game-engine/docs/wiki/systems/scenarios.md); [server contract](../specs/scenarios.md).
- 2026-10-11: data-selected boss lifecycle replaces per-dungeon registration; journal dungeon joins, inspectable compatibility ordinals,393-entry SmartAI census and private both-skin Skum proof documented in [[boss-scripts]].
## 2026-10-11 — integrated gathering and shared-loot readiness

- [Gathering](systems/gathering.md): requested master snapshots integrated without merging into master; scoped checks/format and six real private-realm cases pass. Native images, numeric rank gains, timer boundaries and remaining limits are linked from the engine proof ledger.

- 2026-10-11 class-completeness2: [continuation](../class-completeness2.md) records cone24/corpse119, portals/identity predicates, corrected random72/86/91, and authentic class summon-template closure. Latest probe2854/4776/197; Monk cone has private both-skin receipts. Whole goal remains open.
## 2026-10-11 — additional profession references

Recovered nine local-CASC DB2 payloads and added source-pinned staging import; [receipt](reference/profession-reference-import.md) separates concrete data from unresolved quality/concentration/recraft formulas. No canonical DB mutation or runtime acceptance claim.

## 2026-10-11 — profession specialization runtime

Added source catalog, authenticated query/commit, atomic knowledge/rank/recipe persistence and quest reward boundary; [mechanics](systems/profession-specializations.md). Core source rules already pass; runtime/network/live proofs remain open.

- 2026-10-12: [Release tick profiling](investigations/server-tick-lag.md#release-catalog-copy-regression-2026-10-12): idle objective tick copied the full gameobject catalog; restrict reads to mapped match flag homes, preserving gameplay.

- 2026-10-12: [Class cast-state continuation](../class-completeness2.md#blooming-infusion-and-consumed-missile-modifiers--batch20): current-owner summon/raw-mastery controls, native unused-ready cooldown credit and Blooming Infusion counters; consumed charged modifiers retain launched damage without altering old missiles with new buffs. Native RED/GREEN and scoped adjacency retained; Symbiotic raw/effective basis remains source-blocked, full goal open.
## 2026-10-12 — Transmog metadata and saved configuration

[Authoritative transmog](systems/transmog-apply.md): pinned local-CASC metadata, authored source enums, primary collection states and account-isolated outfit/Custom Set persistence. Current33 transmog tests pass; both-skin private5926 native paid-save, illusion selection, set membership and paged preset load proof links to the client SSOT. Paid Apply unchanged; conditional acquisition/event eligibility and equipped enchant/price policy remain explicit gaps.

- 2026-10-13 — [Rated arena win criteria](systems/dungeon-achievements.md#rated-arena-win-criteria-verified-2026-10-13): native committed online-winner Type37, rated2v2/3v3/skirmish/loser/idle/redb proof; source/data mismatch keeps39 disabled.

- 2026-10-13 — [Current-head achievement live sample](systems/dungeon-achievements.md#current-head-live-sample-verified-2026-10-13): fresh both-skin paid grants, actual2→4 mounts and isolated online comparison; revision-specific source/PNG receipt and owned cleanup.

- 2026-10-13 — [Identity PlayerCondition criteria](systems/dungeon-achievements.md#identity-playercondition-criteria-verified-2026-10-13): class/race-only native evaluator reuse, no dropped unknown fields; authentic Human/Orc cap transitions earn7380 and survive reopen.
- 2026-10-12: Golden Molten Gold gameplay sphere from local TDB12928, source Player reach/ordinary null-target casting, per-area enter/exit, copy/attachment/removal and real periodic damage. Source default-target omission reproduced/fixed;13 supported Golden/companion tests PASS. Intro/follow/native areas remain open.
- 2026-10-13: Updated auras script hooks with native Power Strike Chi proc and actual Chaos Strike ProcChance consumption; real cast proofs and remaining Combat Wisdom effect-1 gap.

- 2026-10-13: Corrected selector56 from caster-only to modeled party/raid area; native145110 group-heal priority and lifecycle tests, explicit charm/static-NPC source boundaries.

- 2026-10-13: Native Ysera145108 owner-health ticks/self-group switching and CannotCrit; six Bevy App controls, isolated per-cast override, no rendered parity claim.

- 2026-10-13: Native163 school-mask critical-damage product and KnowYourEnemy percent updater; actual damage/order/lifecycle controls, retained floating-rounding and rendered-parity limits.

## 2026-10-13 — Prepared follower Group Finder catalog

[Catalog](investigations/follower-data.md#native-group-finder-catalog--2026-10-13): authenticated sorted source entries and empty-catalog reply; actual registered loopback RED→GREEN. Server2d62a2dc targeted22 plus required-environment companion test pass, check/fmt pass. No new live/full-instance proof or canonical data writes.

- 2026-10-13: [Pendant](systems/toys.md#blazing-diamond-pendant-2026-10-13) exact source admission and both-skin natural expiry; restored coin-script precedence before generic targetless missile dispatch.

- 2026-10-13: [Fountain of Knowledge](systems/toys.md#fountain-of-knowledge-2026-10-13) exact admission, authenticated ordinary model and both-skin natural removal; census expectations now retain source-backed Pendant/Fountain while preserving control refusals.

- 2026-10-13: [Accursed Tome](systems/toys.md#accursed-tome-of-the-sargerei-2026-10-13) exact source admission and both-skin natural model removal; fresh-account learning delivers, while earlier profile ownership-update timeouts remain unexplained.
## 2026-10-13 — Native follower class level and routed follow

[Profile/navigation proof](investigations/follower-data.md#native-class-level-and-follow-navigation--2026-10-13): eleven actual labelled basic-role casts/resource publications; Cobra Shot193455 missile-resolution Level panic RED→11GREEN. Real Stockade follow crossed wall at tick19 and lost moving-state progress4.8527yd; native route/progress fix GREEN3. Strict follower missing-bake standing, ordinary behavior retained. Private Ruby12LOS+12NAV/authored zero-gap geometry package staged; advanced AI and new whole-route live proof open.
- 2026-10-13: [Category charge rates](systems/auras.md#category-charge-recovery-rates): native148 through existing category clocks, authentic Trueshot/Aimed Shot RED/GREEN, negative native Fiery Brand and bounded refresh API proof.

## 2026-10-13 — Ruby authored support/vertical search

[Navigation source repair](systems/navigation.md#ruby-authored-support-and-vertical-search--2026-10-13): actual metalplate doorway RED→GREEN, creature/player steep-face distinction, native1500-tick regroup at4yd, nominalY5 source lookup with no larger fallback. Private12-tile NAV/M2 support rebake passes; full-instance/live acceptance remains open.
- 2026-10-13: Charred Warblades native3%/1000ms delayed self-heal producer; real-damage RED at eb9fbf29 (500HP stays500 instead of504); GREEN verification pending.

- 2026-10-13: Batch49 bounded Charred controls/check/fmt pass;3 native memberships promote. [Proof ledger](../class-completeness6.md#native-charred-warblades--batch49); broader goal open.

- 2026-10-13: [Follower diagnostics](investigations/follower-data.md#stockade-diagnostic-attachment--2026-10-13) attach native companions to an existing Normal Stockade copy without Retail catalog/admission changes; full completion proof remains open.

- 2026-10-13: Stockade diagnostic initial admission approved through sourced WorldSafeLoc3599 and real transfer/ACK; no in-dungeon repositioning or stat modifiers. Northshire world-walking cliff remains outside this slice.
- 2026-10-13 — [Group-private runtime](systems/toys.md#source-group-private-runtime-2026-10-13): captured group identity, dynamic membership interest and inherited privacy; behavioral/capture gates tracked separately.
- 2026-10-13: native aura7 fleeing — pinned timing/distance, Detour collision/routes, player control ownership and14 current controls; all-source TDB model geometry producer, lead-only canonical publication pending. See [batch53](../class-completeness6.md#native-fear-navmesh-fleeing--batch53).

- 2026-10-13: five native Druid BeforeCast form transformers and Berserk AfterCast reset;14 current controls, actual census post-cast form telemetry, no generic dummy certification. See [batch52](../class-completeness6.md#native-druid-beforecast-transformers--batch52).

- 2026-10-13: native Druid/Fear master integration85 controls pass,17 actual unshifted BeforeCast receipts and current check/fmt; canonical model-info98,609 rows verified equal to source-prepared slot. Full class census/rendered acceptance remains open.

- 2026-10-13 — [Signed summon titles](systems/toys.md#signed-summon-title-source-boundary-2026-10-13): current source3001/-1 exposed unsigned loader failure; preserve TC int32 without aborting unrelated toys.
- 2026-10-13: native168 source creature-type damage and shared type admission;17 scoped controls plus genuine spell/auto REDs, canonical59-row metadata equality, no tracking44/whole-class certification. See [batch54](../class-completeness6.md#native-creature-type-damage--batch54).

- 2026-10-14: Document pinned Warrior per-effect stun dispatch and conditional Storm Bolts selector; native RED3 retained, GREEN pending. Installed70391 slot data follows published canonical snapshot.

- 2026-10-14: Warrior native stun controls12/12 and adjacent61/61 pass; installed70391 census3185/4459/183 over7827 memberships. New1319011 CSV→published-SQLite gap retained; no whole-talent/Dummy promotion.

- 2026-10-14: Add source-distinct melee/ranged AP model and166/167 percentage consumers; native Battle Shout33→33 and Aimed Shot316 versus408..428 genuine RED retained; GREEN pending.

- 2026-10-14: Native melee/RAP166/167 controls13/13 pass across two scoped receipts; all13 class AP coefficients equal active source. Frontend RAP propagation and NPC/weapon/base budgets remain explicit next layers.
- 2026-10-14 — [Quest fixture refresh](../../quest-objectives-coverage.md#fixture-refresh--2026-10-14): reproduced missing SpellTargetConditions in real Tear item cast and stale empty-loot Netherwing Egg fixture; load authored conditions and use real9548/181746/23830 chest data. Both tests GREEN with unchanged gameplay/data; original open/autoloot ordering retained.

- 2026-10-14: [Wailing Normal run/rewards](systems/boss-scripts.md#wailing-normal-run-and-corpse-rewards-2026-10-14): eight controlled deaths persist bits0–7; fresh owner isolation and eight staged corpse reward commits. Not natural/native clear.
- 2026-10-14: [Installed70391 reconciliation](systems/dungeon-achievements.md#installed70391-reconciliation): regenerate same supported predicate families,5479 ordinary/1821 statistics, reconcile243 inventory rows; coherent-source delta+23/+5, no removals versus69933. Runtime gate pending.

- 2026-10-14: Installed70391 runtime acceptance at a4de9864:173/0/1ignored plus site-export1/1, compiler/policy38/38, server/engine checks and three-repo fmt pass; sibling quest schema aligned before gate. Whole audit continues.

- 2026-10-14: Installed Item/ItemSparse catalog replaces older reference-filtered membership; item-only transactional refresh and exact Skyborne fixture regeneration. See [content import](systems/content-import.md#installed-item-projection).
- 2026-10-14: [Target health46](systems/dungeon-achievements.md#pre-effect-spell-hit-target-health): inclusive pre-effect native110 health; bandage45544 RED reaches real aura before6799 counter failure, structural+141 only; acceptance pending.

- 2026-10-14: TargetHealth46 final integrated acceptance at30a6e515:175/0/1ignored +site-export1/1; server/engine checks and three-repo fmt pass, census5480/1821. Native authentic bandage item-level context and finite-maximum rejection retain pre-effect inclusive semantics.

## 2026-10-14 — Wild summon count boundary

[Wild counts](systems/toys.md#wild-variable-count-summons-2026-10-14): Flute count RED1→source-variable count, native6122 primary scatter RED→GREEN. Retail follow description remains an explicit gap; no stationary/full-Flute certificate. Body/CASС bytes and current NPC DB2 input provenance remain separate from cached metadata.
- 2026-10-14: C++ ground waypoint generation ports pinned TC smooth-surface controls and collision normalization; real Wailing node2 RED retained, GREEN pending. Generic navigation/exact/flying unchanged.

- 2026-10-14: Smooth-source gate05062d71 passes15/15,49 Wailing nodes collision-clear/node2 RED-GREEN, actual Ragefire+Golden controls and strict rejection preserved; check/format/independent artifact verification PASS. Full escort remains RED in unchanged generic movement after path1 arrival.
- 2026-10-14: Distinct native melee/RAP percentages, owner wire and DefenseType-selected tooltip powers; bounded9 leaves5partial. See [[systems/character-stats]] and docs/class-completeness6.md.

- 2026-10-14: Native player autoattack/weapon-spell critical block and launch/impact hit-state proof; NPC/probability/638 boundaries remain open. See [[systems/player-damage]] and docs/class-completeness6.md.
## 2026-10-14 — Pre-effect spell-hit target auras

[TargetHasAura10](systems/dungeon-achievements.md#pre-effect-spell-hit-target-auras) reads actual hit-unit active auras synchronously before effects. Complete positive type110 leaves retain companions; census5481/1821 adds only3579. Compiler RED→GREEN34/34; native RED1bf78023 reaches actual65403 then food cast but11168 absent; GREEN0a349a77 1/1 proves target authority, negatives and reopen. Full required four-thread filter176/0/1ignored plus site-export1/1, server workspace/all-target and fresh engine four-package checks, three-repo fmt pass without warnings; table acquisition/holiday/UI unclaimed.

## 2026-10-14 — Maximum-health percent effect165

[Effect165](systems/player-damage.md#maximum-health-percent-effect165) supplies the actual67531 hit producer needed by BB King. Native RED9e862790 accepts authentic cast but health50000 does not become49990. Implementation preserves integer max-health percentage and common mitigation while omitting direct caster/target bonuses; native GREEN4091b81b proves actual damage10, Enrage/Barkskin exclusions and zero truncation. TargetCreatureId4 compiler RED→GREEN35/35; required integrated filter178/0/1ignored +site-export1/1, server/engine checks and three-repo fmt pass at4091b81b.

## 2026-10-14 — Earned-player prerequisite qualifiers

[Positive87](systems/dungeon-achievements.md#earned-player-prerequisite-qualifiers) requires authored prerequisite IDs. Valid native REDb5ab795c uses the existing full App: actual220737 self-hit grants10460, then accepted Warsong player kill leaves32961Nonevs1. Initial NPC fixture used wrong event and bare World lacked cast resources; those failures are not credit proof. Actor-scope REDf94bd699 reproduces another character's10460 leaking through raw account aggregates into32961. The reader now shares the snapshot's scoped earned-ID iterator; account contribution storage is unchanged. Compiler/policy38/38; native/catalog GREEN35ef2818 passes2/2(25.17s), same-account isolation and reopened counter2. Generated5693/census5490+1821/all243/pinned membership reconciled. Required workspace four-thread filter180/0/1ignored(302.66s) +site-export1/1 at77832266, server/engine checks and three-repo fmt pass; independent artifact verification found no correctness defects. Later Python assertion has separate38/38 proof; no Rust changes. No artifact acquisition/challenge/full1000-kill/UI certification.

## 2026-10-14 — Pre-effect spell-hit target creature entries

[TargetCreatureId4](systems/dungeon-achievements.md#pre-effect-spell-hit-target-creature-entries) and complete Some selectors use actual hit NPC.template_id for native110, retaining DoEmote54 and every companion. Source165 actualdamage/bonus-exclusion/zero-hit boundary passes before valid predicate RED12660Nonevs3. Generated5621/census5483+1821 adds account4436/4437 only. Native GREEN4091b81b2/2(18.30s) proves both factions’ actual leader grants, wrong NPC/Player/missing/depth/idle rejection and durable counters/grant dates after reopen/re-entry. All243 matrix/pinned5483 and readability pass; required workspace filter178/0/1ignored(269.99s) +site-export1/1, server/engine checks and three-repo fmt pass at4091b81b source scope. Independent verifier unavailable; no content/acquisition/UI certificate.
- 2026-10-15 — [Spell casting](systems/spell-casting.md): spell queue window widened from 400 ms to 750 ms, the hasted-GCD floor (user decision; Retail/TrinityCore use 400 ms). RED: `spellqueue_exact_750ms_boundary_is_inclusive` 9/1 with the 400 ms constant; GREEN 10/10.

- 2026-10-15: Wailing source21–27 wake-up/flight exit port after actual Sleep→Stand RED; existing flight policy unchanged, behavioral GREEN/native proof still pending.
- 2026-10-15: Wake/flight exit GREEN053915ae1/1 in31.22s, unchanged firstwave/fourdruid controls2PASS, all-target check14.62s/no warnings and scopedfmt/diff exit0. Corrected fixture instant-aura assumption using actual8153 five-second cast. Independent lead gate pending; natural/native clears still open.
- 2026-10-15: Wailing Normal ritual-wave copy-only projection profile after process-boundary RED; source identity/stat fields preserved, canonical publication/balance separate.
- 2026-10-15: Wailing copy-only Normal wave projection GREENb3cc6fed2/2 PASS7.421s, actual CLI/idempotency/identity-refusal and unchanged row hashes; canonical/runtime application and balance still separate.

## 2026-10-15 — Typed location alternatives

Replaced area-only Some projection with typed17/41 alternatives and strict complete-asset validation. Compiler/native genuine RED8984dbbd; generated5759; GREEN/census pending. [System](systems/dungeon-achievements.md#typed-areazone-alternatives).

Typed location acceptance: integrated995acc3c native/catalog/prior17/41/pinned8/8(59.21s), compiler41/41, census5506/1824 with all243/same7hashes. Native cached254 selfie27968/refused/outside/idle/reopen; no full grants/travel/acquisition/UI claim.

- 2026-10-15: documented read-only Stockade proximity acquisition receipts and pinned TC social/link distinctions; native cause remains under investigation.

- 2026-10-15: recorded native14.5yd Stockade proximity boundary and Moloch source-UI Transmog click/delivered63346 proof; no gameplay change/full-clear claim.

- 2026-10-16 UTC: recorded Stockaggro70 diagnostic3/3, actual earned loot/expected-empty context, complete27067-event measurement and owned-process cleanup; Greed/Hogger-voting and full-spec parity gaps retained.
- 2026-10-15: Feral Spirit owner-power RED and approved owner/weapon-only implementation; intrinsic AP remains named gap; GREEN pending.
- 2026-10-15: Added bounded Login74 negative104 contract and native spell-completeness/initialization boundary; source TC HasSpell/SingleFalse. Focused proof pending.

- 2026-10-15: Negative104 actual SelectCharacter RED724af71c exposed missing selection-to-knowledge edge; fix408f0a3b passes unchanged native handler plus8 scoped controls,43.44s. Census5509/1824, only3hidden character roots, all243/same7sourcehashes.

- 2026-10-15: Feral Spirit owner/weapon-only repair f135c4a8+28859d64, merged afc0e5e5/protocol62865c23:24/24 scoped controls, check/fmt/readability clean; intrinsic AP and full-stat acceptance remain open.

- 2026-10-16: [Razorlash](systems/boss-scripts.md#maraudon-razorlash-source-binding--2026-10-16) real180s SAI proof1/1 PASS; source chance preserved, strict timer expiry and fixture visibility corrected; production check/fmt pass, native/extra-attack parity remain open.
- 2026-10-15: Feero4484 counter63/event77 first-wave RED and queued cross-creature implementation; native acceptance pending.

- 2026-10-15: counter first-wave GREEN1; Feero immunity/ambusher-target RED2, action145/49 implementation.

- 2026-10-16: Feero authored escort gate passes after master integration; both-skin native accept/speech/movement/first-wave partial evidence and completion/data gaps recorded in escort contract.

- 2026-10-16: Corrected70391 Regrowth8936 native cast restores Feero236→484 without admin healing; missing ambusher receipt/completion boundary remains open.

- 2026-10-16: Feero976 both native skins earn credit after all three waves; authenticated missing receipt-index entries resolved without substitutions; same-coordinate grid RED→GREEN8 fixes active escort dormancy beyond spawn grid. Eighteen inspected1920x1080 earned-run PNGs; cosmetic roar, native death-failure and unrelated renderer parity remain separate. See [escort contract](../specs/escort-quests.md).
- 2026-10-16: [Caster-owned school amplification](systems/auras.md#caster-owned-school-amplification--2026-10-16): native Haunt aura270 omission reproduced through ordinary casts; shared owner/school consumer, with pet381/unknown380/full-ability proof separate.
- 2026-10-16: [Native damage prerequisite fixtures](systems/spell-casting.md#native-damage-prerequisite-fixtures--2026-10-16) trace Charred/Rampage intermittent failures to actual NPC dodge and qualify damage-only fixtures with existing native aura; evade guard unchanged.

- 2026-10-16: Rotgrip copy-only binding and real source clocks/self-damage/Puncture/isolated credit; preserved DB2SELF versus TDBvictim mismatch, no fabricated correction. See [boss scripts](systems/boss-scripts.md#maraudon-rotgrip-source-mismatch--2026-10-16).

- 2026-10-16: Optional-cost flat exclusion: native Practiced Strikes/ProtExecute RED3/2; shared totals retain percentage and charged/live matching, optional consumer excludes flat. Native Sudden Death/cost-order controls retained; GREEN38/0/0 and merged5/0/0, workspace/all-target check64.411s, fmt and independent verifier pass. See [auras](systems/auras.md#optional-cost-spell-modifiers-2026-10-16).
- 2026-10-16: Noxxion pinned source port and due-anchored TaskContext repeat; runtime first-cast RED captured, full acceptance pending. [Source](systems/boss-scripts.md#maraudon-noxxion-source-port--2026-10-16).

- 2026-10-16: Noxxion real channel-aware clock/phase/ten-add/damage/isolated-credit acceptance GREEN3/3 at8e544d86; source21708 channel deferral explains compressed due-anchored repeat intervals. Native/ordinary acceptance remains separate.

- 2026-10-16: Gizlock13601 controlled SAI clocks/channel/cone damage/Talk0 and isolated427bit5 credit; [source runtime](systems/boss-scripts.md#maraudon-gizlock-source-runtime), no native/ordinary/publication claim.

- 2026-10-16: Lord Vyletongue21390 runtime RED→EVENT_RANGE9 source port; strict3D/reach/500ms controls, nearby Flash stun and20yd Blink/isolated424bit2 controlled credit. [Details](systems/boss-scripts.md#maraudon-lord-vyletongue-range-events).
- 2026-10-16: Added [incremental achievement evaluation](systems/dungeon-achievements.md#incremental-evaluation--2026-10-16), replacing repeated catalog-wide recursive evaluation with affected-root reconciliation and disposable full-snapshot results. Targeted proof pending.

- 2026-10-16: [Incremental achievement evaluation](systems/dungeon-achievements.md#incremental-evaluation--2026-10-16) integrated proof: 214 required-filter tests plus site-export pass, workspace/all-target check and fmt pass; warm account event 26 definitions / 174 tree nodes, unchanged event zero. Stockade runtime acceptance remains lead-owned.
- 2026-10-16: [Native aura restrictions producer](systems/spell-info.md#native-aura-restrictions-producer--2026-10-16): maintained CLI drops supplied native rows RED2pass/1fail→GREEN3; merged cbeea718 producer3/0/0, native70391 headercheck and workspace/all-target check66.132s/0warnings pass. Canonical table remains absent pending lead publication; runtime health/aura-state admission is not certified.

- 2026-10-16: [Celebras source port](systems/boss-scripts.md#maraudon-celebras-source-port): real no-Wrath RED, redeemed legacy stats replaced from base0, guardian summoner lifecycle failure reproduced; runtime gate pending.
- 2026-10-16: Noxxion pinned source port and due-anchored TaskContext repeat; runtime first-cast RED captured, full acceptance pending. [Source](systems/boss-scripts.md#maraudon-noxxion-source-port--2026-10-16).

- 2026-10-16: Noxxion real channel-aware clock/phase/ten-add/damage/isolated-credit acceptance GREEN3/3 at8e544d86; source21708 channel deferral explains compressed due-anchored repeat intervals. Native/ordinary acceptance remains separate.

- 2026-10-16: Celebras controlled runtime GREEN2/2 in30.73s, including two source guardian waves, cleanup/manual redeemed actor, due clocks and isolated credit. Actual base0 projection GREEN; native/ordinary/regional manual-summon scaling remain separate.

- 2026-10-16: [Landslide source port](systems/boss-scripts.md#maraudon-landslide-source-port) follows authenticated binder19/19/SQL and real no-casts RED; casting-aware timer/threshold port implemented, runtime gate pending.

- 2026-10-16: Theradras source port after real missing-casts RED; Zaetar actual base/Normal projection process RED→GREEN. [Details](systems/boss-scripts.md#maraudon-theradras-source-port).
- 2026-10-16: Profiled full-population idle debug and eleven-peer release; recorded ranked fixed-step costs and health468 scan RED with native crossing control in [[server-tick-lag]].

- 2026-10-16: [Gargolmar](systems/boss-scripts.md#ramparts-gargolmar-runtime--2026-10-16) source clocks/reset/credit and actual Watcher HP; retained distant-charge RED and NPC log-recipient boundary.

- 2026-10-16: [Omor source preparation](systems/boss-scripts.md#ramparts-omor-source-preparation--2026-10-16) authenticated primary/orbital server-side spells/hound base0, producer proof only.
- 2026-10-16: [Native targeted Hug247](systems/dungeon-achievements.md#native-targeted-hug247); complete qualifiers and native RED→GREEN, census247/1042 only.

- 2026-10-16: Content-tuning participation268 bounded to native15; authored min/max, complete companions, generated5809 and census5519/pinned908+909. Native predicate RED genuine, GREEN pending.
- 2026-10-16: Bounded dormant detection/resize RED→GREEN and 43 behaviour controls; irrelevant GO collision RED and membership-index implementation recorded in [[server-tick-lag]].

- 2026-10-16: Content268 native/catalog GREEN3/3(13.47s), including100 native controlled flag settlements for both908/909 grants, faction/idle guards and database-reopened original dates. Integrated server9aca3c1a, final gate pending.

- 2026-10-16: Content268 integrated gate at a8d4d866 passes217/0/1existingignored +site1/1; serverworkspace/alltargetcheckEXIT0/noRustwarnings. Native100 grants908/909 and strengthened89participation guard pass; readability/independentartifactPASS.
- 2026-10-16: Final geometry proof: 50 controls pass, warning-free check, unchanged 187770-creature idle debug sustains 19.9–20.0 ticks/s; detection 1.891 ms/step, resize/collision out of top15; private published-schema refresh recorded in [[server-tick-lag]].

- 2026-10-16: Landslide deterministic seed104 runtime RED isolates authored cooldown rejection from TaskScheduler request timing; unchanged timing bounds and source mechanics retained. GREEN3/3 passed18.40s under four test workers at e8ee92a3.

- 2026-10-16: Ordinary encounter gate exposes Vyletongue sampling-window RED; deterministic blocked/unblocked controls and integer-ms inclusive samples retain all source timing bounds.
- 2026-10-16: Five residual idle systems reproduced with full-population/layout REDs before production changes; existing exclusive queries cached for three topology-discovery costs in [[server-tick-lag]].

- 2026-10-16: All five residual idle microbenchmarks GREEN (6.2–7.4 µs/five updates); merged 86/86 controls pass with b701f32 protocol, live release acceptance pending in [[server-tick-lag]].

- 2026-10-16: Residual idle release acceptance: b701f32/11 real UDP peers/full187770-creature population holds20.0–20.1Hz,weightedmean19.93ms; all5RED→GREEN+86controls/check retained, pressure-heavy first run documented in [[server-tick-lag]].
- 2026-10-16: [Corgi and Rezan source boundaries](systems/toys.md#corgi-and-rezan-source-boundaries-2026-10-16): corrected sparse-fixture learned-use proof; genuine effect3 refusal before exact272071 cosmetic admission; native catalog source resolution, not audible playback. Chili/Enthralling stay named gaps; whole-Toy144/144 and current check/fmt/readability pass. Canonical393 classification includes separate lead template publication, not393 complete toys.

## 2026-10-17 — Combat publication fanout

[Measured fanout](investigations/server-tick-lag.md#combat-publication-fanout-2026-10-17): full-catalog/real-message eleven-owner RED11554345B, real five-toast order and clock178118B. Connection-scoped latest-delta/ordered-toast presentation and Guide content/clock coalescing committed; settlement timing unchanged, no new query variant. Producer GREEN/live11-peer comparison remain pending; earlier engine eight targeted RED/GREEN controls are separate evidence.
- 2026-10-16: [Holiday hearthstone learned use](systems/toys.md#holiday-hearthstone-learned-use-2026-10-16): real persisted bind/source10s cast and shared2309 charge acceptance,146/146 Toy tests; source type17 appearance/cross-map/wall recharge remain separate.

- 2026-10-16: [Gnoll Barrel source pair](systems/toys.md#gnoll-barrel-source-pair-2026-10-16): authentic paired template import RED→GREEN, real area/cursor/source300s simulation lifecycle,147/147 Toy gate; target-dummy combat and missing native533501 body remain separate.

- 2026-10-16: [Akunda source child and visible trap](systems/toys.md#akunda-source-child-and-visible-trap-2026-10-16): separate catalog/child-position REDs, exact302746/display16675, actual cooking/CozyFire/source300s cleanup and source-drift refusals;149 Toy+29 affected gates pass, native appearance/full trigger parity separate.
- 2026-10-17: [Recipient area31](systems/dungeon-achievements.md#eligible-recipient-area31) reads real current terrain at the existing HK boundary, retains ancestry/companions and account SUM; native/account behavioral RED2/2, full50 GREEN pending. Imported70391 structural census5533/1841 adds only224/1151/12411; area metadata is70448, no reconciled70448 achievement-profile claim.

- 2026-10-17: [Active70448 achievement inputs](systems/dungeon-achievements.md#active70448-achievement-inputs) regenerated from active CSVs; named tree Flags/OrderIndex fixes prior raw offsets. Build/key receipt, active predicates and5533/1841 census reconciled (no root delta from migration). Importer behavioralRED→GREEN4/4; canonical publication lead-owned.

- 2026-10-17: area31 on canonical70448 at18856aaf passes controlled native full50, accountSUM, ancestry/timer and pinned census4/4(13.70s); no registered BG policy/credit added. Required integration/check gates pending.

- 2026-10-17: area31/current70448 final integration1c67a39c passes required workspace four-filter228/0/1existing ignored(345.74s), site-export1/1, workspace/all-target check, three-repo fmt/diff. Importer/predicate70448 source reconciliation precedes native GREEN. No new BG policy/playability certificate; lead landing/rendered/fresh UDP remain separate.

- 2026-10-17: [Lifetime HK113](systems/dungeon-achievements.md#lifetime-honorable-kills113) implements explicit LIFETIME_HK_TRACKING_STARTS_AT_ZERO and separate persisted count before absolute113. Full100/native40+60/policy RED0/3; GREEN pending. Active70448 structural5544/1843 adds11ordinary/2stats only, same seven source hashes; all thresholds retained.

- 2026-10-17: lifetime113 at785c8f1b passes6/6(35.54s): native100 roots238/513/39935, actual40+60 account sum, zero-start JSON policy, reopen/relog and5544 pin plus2 BG merge controls. Original dates survive a one-hour clock advance. Required integration/check pending.

- 2026-10-17: lifetime113 final integration772a3be6 passes required workspace four-filter231/0/1existing ignored(353.60s), site-export1/1, workspace/all-target check and three-repo fmt/diff. Active70448 census5544/1843, all243matrix/pin exact; bounded artifact/readability checks pass. LIFETIME_HK_TRACKING_STARTS_AT_ZERO remains explicit; native higher thresholds/BG/rendered/freshUDP/remoteCI/lead landing are not certified.

## 2026-10-17 — Resurrection guild perk excluded by user

User clarification: World of Osso has no resurrection guild perk. Removed Quick/Dead from pending effect coverage; historical installed-data/native receipts are not project requirements. Mobile Banking remains active with the separately specified2h policy.
## 2026-10-17 — Pending-only netdelta producer controls

[Combat publication](investigations/server-tick-lag.md#combat-publication-fanout-2026-10-17): dedicated marked-peer/quiet-frame proof, real level-stream/reconnect equivalence and Stockade earned-delta/toast isolation now pass alongside repaired transport-free criteria and explicit HostClient fixtures. Detailed source scopes/counts live on the investigation page. Merged engine checks and genuine UDP byte/latency/performance remeasurement remain pending.

- 2026-10-16: [Forlorn Toll destination-only flight](systems/toys.md#forlorn-toll-destination-only-flight-2026-10-16): exact ten-model source identity, retained unitless destination flight after genuine early-arrival RED;150 Toy+49 affected+3 importer controls pass, native appearances/publication separate.

## 2026-10-17 — Mobile Banking user policy implemented

User decision specifies MOBILE_BANKING_COOLDOWN_2H7200000ms over installed3600000ms. Installed faction children/type34/lifetime and Friendly1168 chest-user gate; no Friendly cast gate. Real server5/5 and tooltip1/1 GREEN; midcast loss checked before consumption. Native actual5min/2h both-skin proof pending. Resurrection perk excluded by user, not a gap.

## Mobile Banking native acceptance boundary reconciled

[Native Mobile proof](systems/guild-perks.md#native-mobile-banking-proof): code gates landed; both-skin body controls and Modern real2h recast are proven. Final Forever accepted recast remains unobserved after transport loss, not an effect failure. No completed-proof claim.
- 2026-10-17: Omor30707 summon speech: distinguish invalid full-mana fixture failure from genuine128-cast missingTalk0 RED123cf9ad; source30% branch15c8e222, strict ignoredGREEN pending. See [boss scripts](systems/boss-scripts.md#omor-summon-cast-speech--2026-10-17).

- 2026-10-17: Omor30707 strict ignoredGREEN1/1/68.03s at15c8e222; enabled ordinary acceptance only after receipt.

- 2026-10-17: Enabled Omor summon/shield/source-clock controls GREEN3/3/146.31s at2a2c0f28; actual successful/rejected/triggered shield, scheduled hounds/credit retained. Full native encounter and sound10277 remain open.

- 2026-10-17: [Current-victim chase stop](systems/creature-ai.md#current-victim-stop-contract--2026-10-17): real Stockade cached-corner production RED and pinned TC contract; live clear remains open.
- 2026-10-17: Omor slay source and explicit lethal-cause hook; ordinary30686 playerRED plus actualNPCkillRED→strictignoredGREEN2/2/75.22s at3715cdd4. Document corpse-state/beforeJustDied boundary and approved later-loot difference; environmental/owner/revival/order controls pending. [Source and proof](systems/boss-scripts.md#omor-slay-hook--2026-10-17).

- 2026-10-17: Omor slay environmental/owned-hound/revival/callback-order controls GREEN4/4/99.30s at3715cdd4. Removed both acceptance ignores after strict2/2 receipt; remaining verification uses unchanged death/loot/credit/owner tests. Totem title4 remains unsupported by SummonProperties::is_supported, not a new attribution branch.

- 2026-10-17: Slay unchanged death13/loot32/native-imp owner achievement1 plus enabled30686 player1/source-clock-hound-credit1 GREEN48/48 at487f3072; fmt/diff checks0, owned e9 protocol clean. Existing tests/assertions unchanged; generated loot remains after corpse hook by explicit approval.

- 2026-10-17: [Run23 diagnostic frames](systems/creature-ai.md#run23-diagnostic-frames): optional per-tick tank/victim/cache records and fixed-clock proximity acquisition stamps; gameplay unchanged.
- 2026-10-17: Gargolmar heal/slay real2RED→strictGREEN2/2/84.46s;64ordinarynativeSurge+scheduledextras reach genuine missingTalk2RED48.79s, source50% added9c8259bd pendingstrictGREEN. Reconciled historical charge/speech gap descriptions and recorded why a global100ms caster throttle is not TC behavior.

- 2026-10-17: Gargolmar Surge50% strictignoredGREEN1/1/50.75s at9c8259bd after real64castRED; removed allthree speech ignores after their strict receipts. Enabled complete Gargolmar regression group pending.

- 2026-10-17: Enabled Gargolmar8 GREEN8/8/294.43s atce47484e, including charge/reset/Watcher heal/credit and allspeechcases. Readability audit splits two new damage predicates into named ownership/kind terms; only those two cases need scoped final proof.

- 2026-10-17: Gargolmar predicate-only final2/2/63.96s atf7126b3b; other six cases retaince47484e8/8 proof, no broad rerun. Also independently read LEAD kill-hook gate449:766/0/28ignored1785.58s/all sixSlay/EXIT0/FMT0, head5032 reportedLANDED.

- 2026-10-17: Verified active70448 Herald DungeonEncounter/Journal/actor join and named ENCOUNTER_CREDIT_FROM_RETAIL_DUNGEONENCOUNTER policy. Audited TC ignoredout-of-rangebossrow, sameactorNazan morph,30717/property64Vazruden,ordinarygroup357/3spawns and sourceCT0→maptuning. Added unverified [Herald contract](../../docs/specs/ramparts-herald.md); no runtime or canonical data changes.

- 2026-10-17: Herald copy-only server-spell profile: realCLI invalid-choiceREDfd9ad1db→GREEN6/6/0.201s2e0a3835. All7+7actualTDBrecords/everyfield/idempotence/unrelateddata/atomicmissing30698effect refusal; existing3profiles/canonical/symlinkguards retained. No actor/binder/runtime/credit claim.

- 2026-10-17: Herald70448 binding CLI REDac42b003→25/25aefee7b0; strictretailCSV/build/namedgap/sourcehash/primaryonly/populationpreservation. Fixed actual SQLite fixture lifetime warning using closing+transaction contexts, not suppression; affected binding3/spellproducer6 GREEN9/9/0.896s with ResourceWarning elevated at9874a576. Unchanged22bindingproofretained. Structuraldata only, runtimepairedcreditstillOPEN.

## Native Mobile Banking acceptance complete

[Native proof](systems/guild-perks.md#native-mobile-banking-proof): both-skin cast/bank/nonmember refusal/despawn and actual post2h accepted casts pass on identical artifacts. Forever7242700ms complements the retained Modern-origin7311223ms interval. Fifteen inspected unchanged full-HD originals published; owned runtimes stopped and owner-checked window cleared. Earlier interrupted overall results remain intact; countdown labels are not timing oracles.

- 2026-10-18: Gate547 Polymorph HP-oracle failure qualified with native50240 forced misses: RED7d14e1113 logs three actual zero-damage melee misses while oldHPassertion fails. Replace both recovery HP checks with directed ordinary swing events, retain real native forced-miss control;264consumer unchanged. See [auras](systems/auras.md#polymorph-recovery-swing-oracle--2026-10-18).
## 2026-10-18 — Follower dummy-root periodic maintenance

[Investigation](investigations/follower-data.md#dummy-root-periodic-maintenance--2026-10-18): genuine repeated8921/starved93402 RED→native two-child-aura/Wrath GREEN; three-row COPY only, source-description references and native CSV fields verified, ROOT_CHILD_LINK_DESCRIPTION_ONLY per-row receipt. Producer3 integration+4 validation passes across explicit runs; scoped check/fmt pass. Lead gate/publication pending; full-kit/live/joint interrupt gates remain open.
- 2026-10-18: Native143 SpellLabel cooldown recovery reuses286 start/remaining lifecycle. Normal Autumn leaf RED2/3→native7+retained10GREEN; label2960-only native row absent,143refresh/source-only, Seasons/charge/time-rate/fullability excluded. See [auras](systems/auras.md#cooldown-recovery-rates).
- 2026-10-18: AURA_POSITIVITY_702 actual native creature curse→real player Remove Curse RED0/3→GREEN3; generic342 signed-stat counterpart, no per-spell or follower override. Prior ignored702 enabled; targeted49PASS128.05s. PinnedTC342 omission and source/native qualification explicit in [auras](systems/auras.md#native-signed-haste-positivity--2026-10-18); full positivity/follower702 picker/legal/freshUDP remain unclaimed.

## 2026-10-18 — Shared death-prevention commit

Recorded Guardian native rescue and shared Ardent actual-absorption/same-frame mitigation boundaries in [auras](systems/auras.md#shared-death-prevention-commit--2026-10-18); native rows/clocks/RNG unchanged, final combined gate pending.

## 2026-10-18 — Prayer of Mending source/RED

Recorded real33076 ward RED, approved per-cast stack/original-caster API and native delayed jump in [auras](systems/auras.md#prayer-of-mending-baseline--2026-10-18); player recipient qualification and PRAYER_OF_MENDING_NPC_TREAT_AS_RAID_UNIT_STATIC_FLAG_ABSENT explicit; Native8/8 GREEN37.08s including actual expiry/exhaustion/foreign/no-recipient and no-override Agony controls; native4000ms Fireball readiness honored, independent lead gate pending.
- 2026-10-18 — Replace LOS/nav bake and vehicle duplicate binary readers with pinned wow-formats geometry metadata; preserve selection/posing policy and capture old-reader Stockade reference. [Shared formats](systems/wow-formats.md).

- 2026-10-18: Herald Call Nazan source ingress, native integer damage qualification and actual firstWest Fireball interruption. Two-case RED→GREEN; exact one-tick/self-target refinement plus all7 Herald/four grounding11/11 PASS176.19s atd43cd6b04; LEAD independent gate/check/fmt pending. See [boss scripts](systems/boss-scripts.md#herald-call-nazan-descent-ingress--2026-10-18).
- 2026-10-18 — Pin projected runtime-ground metadata APIs in wow-formats; preserve selected-field/FDID/error policy and frozen metadata/bake receipt boundaries. [Shared readers](systems/wow-formats.md).

- 2026-10-18: Runtime height projection; frozen old-reader receipts, shared decoder pin and local policy boundary. See [shared formats](systems/wow-formats.md).

## 2026-10-19 — Stockade path-map renderer

Manual SVG renderer with shared pure pull-source pack identity and existing validated navigation artifact codec. Trace clock/capture gaps explicit; real proof pending.

## 2026-10-19 — Path-map fixture proof

Native fixture RED→GREEN2 at91fa9b74d: concrete points/time/death/title and distinct source ALIVE/DEAD/UNKNOWN. Proven tests enabled; real geometry/run and retained planner gate pending.

## 2026-10-19 — Real five-bot path map

Explicit admitted unit_transform sampling replaces incomplete UnitPick traversal. Actual52.365s failed Stockade capture has all5pose/facing tracks; real143spawns/nav34 SVG+PNG inspected/published with FAILED/no-boss title. Original63s gap/failure record preserved. No clear/retry/server-panic fix claimed; owned5981 cold/windows cleared; independent lead gate pending.
- 2026-10-18: Shared MH2O raw records; frozen independent client/server layers, errors, queries and navigation water-bound baseline. See [shared formats](systems/wow-formats.md).

## 2026-10-19 — Online guild zone producer

Existing guild snapshot paths now resolve same-guild online map/position through GroundMap→AreaTable and startup-loaded authentic labels; None for unavailable/offline. Native HostClient behavioral RED precedes implementation; GREEN/coordinated gate/native pending.

## 2026-10-19 — Online guild zone targeted GREEN

Actual protocol1, both-skin UI1 and native HostClient server1 pass after their observed REDs. Tests enabled; production area authority/label semantics unchanged by fixture repairs. Coordinated integration gate and native proof next.
- 2026-10-18: Share MFBO minimum signed-grid decoding, preserving consumer selection/errors and runtime plane policy. See [shared formats](systems/wow-formats.md).

- [Selected native follower metadata](investigations/slow-native-test-fixtures.md#selected-native-follower-metadata-2026-10-19) — existing canonical template/class views, two-live-app isolation, unchanged1353-corner/144-origin/36-kit coverage and matched wall/CPU/RSS observations.
## 2026-10-19 — Priority-preserving follower interrupt provider

[Provider integration](investigations/follower-data.md#priority-preserving-interrupt-provider--2026-10-19): shared live source predicate with actual caster, once at first available kick-capable entry; real Garrick32 ordinary-update RED→96231 GREEN, native26 heal633-before31935 preserved. Four requested families50PASS118.50s/zero warnings, non-testcheck2m37s/fmtPASS; rewrittenf383e28c1 pushed, leadgate pending. No planner/navigation/ai.rs redesign or whole-kit/live acceptance.

## 2026-10-19 — Follower kick candidate-scope regression repair

[Exact root/proof](investigations/follower-data.md#priority-preserving-interrupt-provider--2026-10-19): fk702 two genuine Overheat regressions despite prior50GREEN; bounded diagnostic2RED identifies spec63 off-selectedkick from widened all-engaged handoff. fb4edd4b7 restores selectedenemy-only runtime envelope, providermultiAPIunchanged; fourfamilies+bothunchangedOverheat52PASS96.87s/ZEROwarnings, check8.43s/fmtPASS. Pushed; successor gatepending. No assertion/planner/ai.rs edits, no weakened collection/budget checks.

## 2026-10-19 — Swiftmend native prerequisite qualification

[Qualification](investigations/follower-data.md#swiftmend-prerequisite-qualification--2026-10-19): test-onlydecf6a597, native18562/noHoT wronglyacceptedRED; real774/actualhealPASS; lowernoHoTversushigherHoTfollowerpatientselectionRED. Explicitignoredrun1PASS/2FAIL22.71s/zero warnings, all3remainignored. Backendboundaryfailsfirst; no follower-onlyworkaround or productionchangeauthorized.

## 2026-10-19 — Swiftmend computed-state backend and generic patient eligibility

[Source/capabilities/proof](investigations/follower-data.md#computed-target-state-and-eligible-patient-ranking--2026-10-19): onlystate15computed, ANYcasterHoT, exactownerrelation/namedfollowerpolicy; originalownernegativecorrectedtoself. SourceRED3/2→backend4/1→separatelyapprovedsharedtargetpredicatebeforehealthrank7GREEN; final278PASS/8existingignored535.73s, privateAzurefrontalexplicitLOCALskip, check43.07s/fmtPASS. Uncomputedstatesnotfalse; no18562override orpriority/timer/movement/pet/planner change. Pushed7938b558d; leadgatepending.
## 2026-10-19 — Leadership transfer native proof

gz709 landed protocol724ec71/serverb15526d95/engine79b33ab93; two authenticated actors prove typed `/gleader`, immediate rank/permission refresh and same-private-DB fresh-server/client master settings in both skins. Six inspected original PNGs published; historical original unchanged, owned actors/slice stopped and window cleared. See the voluntary leadership transfer system section for exact evidence and proof exclusions.

- [Prepared encounter fixture inputs](investigations/slow-native-test-fixtures.md#fully-prepared-encounter-inputs-2026-10-19) — complete immutable producer/composite profiles, independent resource copies, cross-profile/difficulty isolation;28 unchanged cases and matched wall/CPU/RSS, including cold Ragefire regression and retained-memory cost.
## 2026-10-19 — Actor-owned periodic-damage maintenance

[Source and native proof](investigations/follower-data.md#actor-owned-periodic-damage-maintenance--2026-10-19): SimC76f8118actor-specificMoonfire semantics confirmed; ordinarydual164812caster/tick+ownrecastcontrolsPASS/foreignsuppressionRED. PeriodicdamageAttack-onlycasterfilteref0603bb1 gives4nativePASS20.11s includingforeignHoTunchanged. Enabled3provedqualifications; integratedkit/stackingcontrols andleadgatepending. No globalhas_aura/timer/priority/movement/pet/planner edits.
## 2026-10-19 — Last Online authority implementation

After pushed ignored tests and actual disconnect/save/despawn→closed DB/new-app query RED, added disconnect-only character logout persistence and optional raw guild timestamp. Rest/admin timestamps are not reused or backfilled. Focused GREEN/coordinated landing/native pending; see guild lifecycle Last Online section.

## 2026-10-19 — Actor-owned DoT integrated proof

[Exact scope/proof](investigations/follower-data.md#actor-owned-periodic-damage-maintenance--2026-10-19):6a82516ac integratedkit_+nativeperiodic/coexistence60PASS/0FAIL/1explicitfile-writingexportignored45.05s/compile5m20s, check1m55s/fmt/diffPASS, zero warnings. Allfournewnativecasesenabled, sourceSimC76f8118receiptretained; foreignHoT/globalhas_auraunchanged. Pushed; independentmo730queued(allfollowers/auras/pets+AzureENV/check/fmt). No redundantlocalrun.
## 2026-10-19 — Last Online native acceptance

gl728 landed protocol50517c9/server1a2f13fd4/engine51f1af4f8. Two authenticated actors prove observed disconnect/recency, genuine reconnect and same-private-DB fresh-server/client offline readback in both skins; six inspected original PNGs published. Historical original unchanged; owned actors/slice stopped, UDP5891 unbound and window cleared. [Exact native evidence and automated-only boundaries](systems/guild-lifecycle.md#last-online-native-acceptance-2026-10-19); no backfilled epochs or audio/clean-exit repair claim.

## 2026-10-19 — Native Germination shorter-remaining routing

[Source/native boundary](investigations/follower-data.md#germination-shorter-remaining-routing--2026-10-19): SimC76f8118/native155675learned33not32. Pushedignored3first; native1PASS/secondHoTmissingRED/thirdprerequisiteblocked19.16s→pre-applyordinary155777redirect3GREEN30.79s/zero warnings. EnabledafterGREENc252954fc, dualperiodichealreceipts/otherdeadlineunchanged; no cost/timer/globalstacking/Swiftmendinteraction changes. Pushed; finalgatepending.
## 2026-10-19 — Swiftmend own-HoT hit consumption

[Native/source scope](investigations/follower-data.md#swiftmend-own-hot-consumption--2026-10-19): pushed3ignoredtests064663f5f BEFOREproduction; actual17consumption/order2RED+32nativeVerdantPASS→f389onhit3GREEN17.62s. Enable3andaddforeign774instancepreservationcontrol; nobonusheal/cost/timer/priority/globalstacking changes. Integratedproof/leadgatepending.

- [Canonical toy spell reuse](investigations/slow-native-test-fixtures.md#canonical-toy-spell-reuse) — existing full spell-store copies at two fixture sites;213/213 unchanged cases before/after, five affected tests131s faster, whole-family contention regression disclosed.
## 2026-10-19 — Resto Soul of the Forest proc and persistent numeric healing

[Exact source and numeric proof](investigations/follower-data.md#restoration-soul-of-the-forest-next-heal-snapshot--2026-10-19): learned21missing114108RED/untalented20controlPASS→native3GREEN23.33s/zero warnings. Actual Regrowth normalized355→565 with distinctnative20/21baselines; Rejuvperiodic61→100, nativeeffect60% snapshot/next-hitconsumption. No injectedbuff/stats or392302targets. Enabled3afterGREEN93488e9b1; broader independentgatependingaftergm748.

- [Promoted battleground world reuse](investigations/slow-native-test-fixtures.md#promoted-battleground-world-reuse) — exact30/628 decoded promotion profiles cloned per app; two-live-app/cache isolation and unchanged9Alterac cases pass, wall292.732→35.023s with +97088KiB peak RSS.
## 2026-10-19 — Native Abundance five-own-HoT threshold

[Exact authority/native proof](investigations/follower-data.md#abundance-native-five-hot-threshold--2026-10-19): current70448threshold5/single-stack overrides olderTCstacking/SimCNYI. Native68controlPASS/69fifthgrantRED/expiryprereqblocked→apply/remove reconciliation3GREEN25.08s/zero warnings aftermasterad852. Enabled099e949e0; no Germination counting/cost-critnumericcertification/globaltimer change. Independentab759queued(druidfamilies+Abundance/check/AZEXIT/fmt).

## 2026-10-19 — Loa canonical fixture spell reuse

[Loa follow-up](investigations/slow-native-test-fixtures.md#loa-canonical-spell-store-follow-up): two canonical reloads use existing complete store copies, no new cache. Same seven cases pass before/after, wall132.283→99.566s/CPU127.577→98.679s/RSS+36500KiB. All1800s/3600s ordinary frames and persistence/assertions preserved. Pushed208f091df precedes after measurement; independent lead integration pending.
## 2026-10-19 — Nested achievement fixture catalog reuse

[Nested-all fixture](investigations/slow-native-test-fixtures.md#nested-achievement-canonical-catalog-reuse) clones the existing canonical catalog, no new cache or map producer change. Same native test/four difficulty-prerequisite variants PASS before/after27.029→17.224s wall; CPU26.189→17.176s and peakRSS+52508KiB. Fresh apps/redb, persisted-reopen/foreign-character/refusal assertions unchanged. Code6122e8f0c pushed before after; independent lead integration pending.

- 2026-10-20: Aura world/XP slice: sourced TC a352b1fa matrix, native ignored RED tests and server-state declarations; [coverage/gaps](systems/auras.md#aura-worldxp-native70448--2026-10-20).

- 2026-10-20: Wired generic world/XP consumers after13 genuine native RED failures; corrected safe-fall clock remains pending its own RED. Client and TC-NULL gaps remain explicit.

- 2026-10-20: Corrected native SafeFall1860 reaches numeric RED beside14GREEN; implemented144 summed landing-height reduction without changing base curve. Final targeted GREEN pending.

- 2026-10-20: Aura world/XP native15/15GREEN29.01s atc9b7c3e59, no warnings; enabled unchanged tests. Source/per-type matrix and TCNULL/client/persistence/base-curve gaps retained; lead integration gates documented.
## 2026-10-20 — Crafting reagents from banks

[Professions](systems/professions.md) count and take recipe reagents from the bags, character bank, then Warband bank (policy CRAFTING_REAGENT_CONSUMPTION_ORDER_RETAIL_ACCESSIBLE_BAGS); [banks](systems/banks.md) now send both `BankContents` on world entry and changed banks after such a craft (`ChangedBanks`). Crafting-order crafter reagents and tools remain bag-only.

- 2026-10-20: Nethekurse native combat and authentic StringId peon callback proof ingested into [boss scripts](systems/boss-scripts.md#shattered-halls-nethekurse-2026-10-20); source gaps explicit.

- 2026-10-20: Nethekurse source-backed remainder/native wild+patrol+Reset proof added; COMPOSITE17GREEN, canonical fissure publication and absent native/create-properties gaps retained.
