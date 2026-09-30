# Wiki Log

## [2026-09-30] systems | Merchant output price boundary

Updated [[economy-catalog]]: recipes whose output range exceeds existing u32 vendor prices are excluded with unchanged market values, protocol limit and TSM provenance. Input-only AH markets retain u64 prices. Read-only Lara 1295:0:165 import excludes one recipe (10621/output 8345), leaving 223; main owns fixture regeneration and startup proof.

## [2026-09-30] systems | Global auction browse contract

Added [auction house spec](../specs/auction-house.md) and [[auction-house]]; linked [[bot-economy]] and index. Inspected shared `88bc3fe`: distinct browse messages with per-item price/quantity and distinct total, unchanged flat shapes. Inspected server flat/session/class/usability boundaries; grouped server, CLI and native integration remain concurrent and unverified. Recorded ItemID → real AuctionID flow, actual-coordinate/listing-price evidence and CLI-first then engine <200 ms request/response goal. Reported optimized flat 300k/144 ms helper timing is not network acceptance; grouped performance pending. No acceptance checked; no code or owned CLI/engine docs changed.

## [2026-09-30] systems | Economy catalog importer

Created [[economy-catalog]] and indexed it. Read-only world.db and timestamped public TSM snapshots supply runtime item metadata, market maps and explicit spawned merchant/profession/expansion assignments. Jillian Tanner 5565:0:165 has 224 classic Leatherworking recipes, 312 markets and 33 raw gatherables; quantity follows profession.rs raw rounding, not points + 1. Report records selected price/popularity provenance, exclusions and explicit empty-production warnings. Midnight 11 remains blocked: 115 CREATE_ITEM output spells, six Fishing mappings, 109 missing mappings, no Leatherworking mapping or claimed new Silvermoon spawn. Runtime integration and spec acceptance remain separate.

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
