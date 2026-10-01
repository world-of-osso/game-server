# Wiki Index

Content catalog for the game-server wiki.

## design/

Architecture decisions and design specifications.

| Page | Summary |
|------|---------|
| [[stack-decisions]] | Why Bevy + Lightyear + redb; SpacetimeDB rejection rationale; bitcode and SQLite choices |
| [[azerothcore-comparison]] | Side-by-side: language, ECS vs OOP, tick rate, threading, networking, persistence, prediction, scripting |
| [[open-questions]] | Unresolved: tick rate (single vs multi-rate), world size (big_space vs origin-shifting), instance management, hot-reloading scripting |
| [[tick-loop-design]] | Draft normative server-loop design: 20Hz fixed phases, Update/PostUpdate boundaries, completed player-death and broad auto-AFK scan reductions, catch-up, replication, persistence, and review decisions |

## reference/

External resources, tooling, data sources, development workflow.

| Page | Summary |
|------|---------|
| [[reference-repos]] | AzerothCore (architecture + data), Rustbolt Core (Rust patterns), SpacetimeDB (rejected), wow-engine (sibling renderer) |
| [[auction-load-fixture]] | Opt-in owned 300k-offer redb producer from real catalog/templates; reopen/counter/pagination assertions, not CLI latency proof |
| [[dev-commands]] | cargo run, cargo test, run-tests.sh, admin CLI via SSH, live GameObject lookup, dependency versions |
| [[crate-structure]] | Three crates (shared, server, client) with purpose and module breakdown |
| [[code-health]] | 4,838 symbols, 88 untested production candidates, dead-code situation, top hotspots |

## systems/

Server gameplay systems.

| Page | Summary |
|------|---------|
| [[player-damage]] | Retail player damage: weapon swings (AP / 6 × speed), off hand, haste timers, TrinityCore hit table, ExpectedStat creature armor, spell power from an Intellect main stat, spell variance/versatility, TRIGGER_MISSILE |
| [[level-scaling]] | Retail ContentTuning creature levels: per-viewer level, ExpectedStat health/damage, shared health pool through damage and health multipliers, target-frame level colour |
| [[banks]] | Character, Warband and guild bank storage (redb), one-transaction moves, guild ranks, daily reset, logs |
| [[game-objects]] | Guild Vault and Mailbox spawns from content_gameobject, GeoBox reach check, UseGameObject |
| [[meeting-stones]] | Meeting stone summons: GameObject::Use checks, 59782 cast and channel, TRANS_DOOR ritual portal, SUMMON_PLAYER offer, accept teleport |
| [[trade]] | Trade sessions, AzerothCore initiation checks, one-transaction exchange through InventoryModel, refusal keeps the window open |
| [[mail]] | Player and Auction House mail: per-character redb mailboxes, one-transaction send/take/return, delivery delay, expiry sweep, PendingMail |
| [[professions]] | Retail skill lines (parent + expansion child), TDB1210 trainers, crafting through the spell pipeline (reagents, tools, CREATE_ITEM, TrinityCore skill-up chances) |
| [[ecs-storage]] | Bevy ECS as columnar in-memory database: archetype tables, zero-overhead queries, built-in change detection |
| [[swimming-and-breath]] | MH2O terrain liquid (client-matched sampling), swim height bounds, no gravity for swimmers, TrinityCore breath/fatigue mirror timers and drowning damage |
| [[networking]] | Lightyear UDP, pre-entry visibility, matching MCVT server terrain sampling, dirty-candidate replication, spell cast receivers and `Moving` input marker, 50 ms cadence, persistence, channels, and interest management |
| [[persistence]] | Dual-layer storage: redb for mutable player data, physical equipment ownership, login-loaded gold, mutation-boundary GoldUpdateQueue replication, and SQLite for static world data |
| [[content-import]] | AzerothCore quest/vendor/trainer/loot/gossip/item tables in world.db; measured Wrath→Retail ID mapping; Retail-first `content_*` tables from content_policy.py, override CSV, drop report; TrinityCore Retail spawns for maps 0/1 |
| [[world-data]] | SQLite world.db, AzerothCore/Retail sources, signed creature-event membership and inactive-event selection, WorldDataPlugin |
| [[class-progression]] | Race/class validation, Initial spec below 10, known spells from ClassSpells, action bars, `character_progression` table, admin set-level live update |
| [[talents]] | DB2 trait trees per class, TrinityCore-style config validation (visibility, gates, prerequisites, choice/hero nodes, level point budgets), `character_traits`, commit/snapshot messages, talent spells in KnownSpells |
| [[spell-info]] | SpellStore (every DB2 spell, all effects/costs/options at DifficultyID 0, charge categories), ClassSpells, raw power-cost units, load cost |
| [[character-stats]] | Retail 12.x player stats: SimC class/race base stats, CASC hppersta/basemp/combatratings, spec main stat and mastery, `StatModifiers` aura fold-in, base-mana costs, gaps (no gear/weapon data) |
| [[spell-casting]] | Cast pipeline (validation, cast bars/channels, costs, GCD/cooldowns/charges), implicit targets, effect handlers, combat log scope, fractional power/rate and rune replication, known-spell check |
| [[auras]] | Aura runtime: stacking/refresh, periodic ticks, procs, interrupt flags, dispel, crowd control, spell modifiers (class mask + SpellLabel), `StatModifiers` fold-in, UnitAuras replication, handled aura IDs |
| [[tick-model]] | Observed server loop: 50ms runner/fixed cadence, schedule ownership, explicit ordering, completed player-death and broad auto-AFK scan reductions, maintenance gates, and IPC boundary |
| [[zone-architecture]] | Maps 0 and 1 plus dungeon copies in one World by `InMap`; map transfers (NewWorld/WorldPortAck), InstanceManager, area trigger teleports; per-zone sets still proposed |
| [[factions]] | `UnitFactionTemplate` sources (creature_template, ChrRaces), FactionTemplate reaction rules, attack/assist checks, Wrath vs Retail ID coverage |
| [[combat]] | Auto-attack system, NPC/player lethal transitions, split death proofs, melee hit table (miss/dodge/parry/glancing/block/crit/hit), spell formulas, stat calculations, DR curves |
| [[creature-ai]] | Three movement types (idle/wander/waypoint by addon path_id), formations, addon mounts/auras, grid activation (GridDormant), SpawnPoint, chase system, 40-yard leash/evade, return-to-spawn |
| [[boss-scripts]] | ScriptName creature AI (CreatureAI hooks, EventMap, BossAI, InstanceScript per copy, creature_text Talk), Stockade boss ports, creature casts/ExpectedStat spell damage, dynamic objects |
| [[creature-aggro]] | AggroMode per template, level-scaled aggro radius, threat from damage/harmful spells, top-threat victim, evade (AzerothCore CreatureAI) |
| [[inventory]] | Owner-only bag/equipment snapshots and deltas, bag placements and item-instance tables, Retail swap/split/destroy/equip validation, data gaps |
| [[quests]] | Quest content load, AzerothCore accept/prerequisite/marker rules, per-player giver markers, kill and bag-item progress, turn-in rewards, quest log persistence and deltas |
| [[bot-economy]] | Approved economy spec, integration boundaries and read-only admin economy-status snapshot; live runtime/restart/client/latency proof outstanding; importer in [[economy-catalog]] |
| [[auction-house]] | Distinct global ItemID browse API, preserved flat AuctionID selection, source audit and pending CLI-first/native/request-response proof |
| [[economy-catalog]] | Read-only world.db/TSM importer, evidence-based gathered/market inputs, exact unlimited supplier contracts, provenance, price bounds and Midnight mapping gaps |
| [[merchant]] | Vendor lists from content_npc_vendor (references, limited stock restock), buy/sell/buyback (12 session slots), repair cost from AzerothCore MaxDurability + Retail DurabilityCosts/Quality |
| [[npc-interaction]] | Replicated `NpcFlags` (AzerothCore→Retail bits), `InteractNpc` validation (TrinityCore GetNPCIfCanInteractWith, 8.5 yd), gossip menus from world.db, role routing, per-frame session close, auction house gated on an auctioneer session and its faction house |
| [[experience]] | Retail kill XP (gtXp, grey/zero-difference, elite, expansion /10, group split), QuestXP DB2 with low-level reduction, GiveXP level-ups, `character_xp`, health/mana refill, protocol gaps |

## investigations/

Debug logs and root cause analyses.

| Page | Summary |
|------|---------|
| [[semantic-noop-replication]] | First- and second-wave server equal-assignment proofs/fixes, Bevy 0.19 change ticks, Lightyear 0.28.0/bevy_replicon sending, direct NPC death transition, Gold ruled out, and blocked wire-level proof |
| [[recurring-loop-elimination]] | Tests-first reductions and eliminations across auction, terrain, friends, auto-AFK, interest, Lightyear, Who roster, gold, durability, calendar, trade-offer, inactive auto-attack, rest-state, and player-death scans; broad death/auto-AFK scans are eliminated while wider scheduler/dirty-boundary requirements remain partial |
| [[server-tick-lag]] | :5000 simulation at 55–70% speed: frame/system instrumentation, dormant-creature scans, per-character save syncs, SipHash interest maps; debug/release/opt-level measurements with 1–3 clients |
| [[tick-loop-audit]] | Exhaustive recurring-schedule audit: 103 registrations / 130 expanded systems; seven narrow removal slices plus player-death and broad auto-AFK scan reductions complete, while preserved Lightyear, interest, friends, and terrain requirements remain partial alongside persistence/order risks and receiver-fan-out measurement |
