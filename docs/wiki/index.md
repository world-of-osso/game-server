# Wiki Index

- [Disenchant](systems/disenchant.md) — Retail bag-item cast/loot pipeline and bounded native proof.

## October 9 reconciliation

- [Deployment](../deploy.md) — sakuin NixOS unit/state SSOT; layout `0dcaa23`, bookworm compatibility `6bfd7b6`. Desktop/local workflows are private development.
- [Banks](systems/banks.md#closed-bank-protocol-rejection-2026-10-09) — closed-bank rejection/persistence `8ac442f`, landed `104ae6a`; [spec](../specs/banks.md).
- [Auction expiry](systems/auction-house.md#injected-expiry-clock-2026-10-09) — ServerClock `9927e77`, escrow proof `ab4cd69`, landed `137c462`; listing/mail wall timestamps remain separate.
- [Defensives](../defensives-audit.md) — final scoped tests `e86c038`, recorded gate `9f54920`, landed `c5432a4`; Blur/Turtle limitations remain, not complete ability parity.


[Current bounded Skyborne/commerce audit](systems/skyborne.md#native-entry-and-npc-startup-source-gap-2026-10-05): approved health/unit-class estimates supersede the historical hold; [fresh workspace/startup proof](systems/content-import.md#forever-zephras-overlay) and linked bounded native acceptance retain visual/gameplay gaps and pending native artifact gate. [Owned commerce and requirement receipts](systems/inventory.md#bounded-owned-commerce-and-requirement-proof-2026-10-05) are scoped4/4 and11/11, not whole parity.

[Native desktop/local development](../remote-builds.md) — normal build/run/check/test workflow, host-local runtime state, and pending runtime proof; [contract](../specs/build-host.md).


Content catalog for the game-server wiki.

[Appearance acquisition](investigations/appearance-acquisition.md) — Retail Warbands disposal/quest rules, atomic collection receipts, real-item acquisition/reopen proofs and missing-path limits.

[Group roster portrait transport](../group-roster-portraits.md) — cached head appearance, cross-zone/offline lifecycle, behavioral fixtures and deferred native-client hookup.

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
| [[crafting-reference-import]] | Seven pinned Midnight crafting tables, exact schema/provenance, atomic staging import and modern-resolver handoff |
| [[reference-repos]] | AzerothCore (architecture + data), Rustbolt Core (Rust patterns), SpacetimeDB (rejected), wow-engine (sibling renderer) |
| [[auction-load-fixture]] | Executed owned 300k-offer/311-real-ID producer; reopen/counter/second-page proof, linked separate CLI latency ledger |
| [[dev-commands]] | Native workflow reference, local standalone tests, admin CLI via SSH, live GameObject lookup, dependency versions |
| [[crate-structure]] | Three crates (shared, server, client) with purpose and module breakdown |
| [[code-health]] | 4,838 symbols, 88 untested production candidates, dead-code situation, top hotspots |

## systems/

- [Class spell batch6](systems/spell-casting.md#sixth-batch-local-retail-rows) — eight ranked bounded paths plus Ice Lance;17 new cast functions, spell_cast224/224, Dummy roots173→172; retained variants explicit.

- [Class spell batch5](systems/spell-casting.md#fifth-batch-local-retail-rows) — eight ranked paths plus Lava Burst;14 new native functions, full spell_cast207/207; Dummy roots176→173.

- [Class spell batch4](systems/spell-casting.md#fourth-batch-local-retail-rows) — ten bounded paths/eight ranked candidates;17 new native proofs, Dummy roots177→176.

- [Generic toy aura effects](systems/toys.md#generic-scale-fall-effect-immunity-and-override-sets-2026-10-10) — additive61, feather105, effect37 and OverrideSpellData293; census and authentic-data preparation.

- [Creature navigation](systems/navigation.md) — shared LOS/ground collision collector, tiled Recast bake and creature paths; [contract](../specs/navigation.md).

- [Class defensives audit](../defensives-audit.md) — 13 catalog candidates; melee immunity/absorb, dodge and Brew fixes, proof ledger and remaining Blur/Turtle limits.

Server gameplay systems.

- [Player resurrection and creature taps](systems/player-resurrection-and-taps.md) — spell-effect consent offers, exact restoration, stable tap IDs and bandwidth; [contract](../specs/player-resurrection-and-taps.md).

| Page | Summary |
|------|---------|
| [[player-damage]] | Retail player damage: weapon swings (AP / 6 × speed), off hand, haste timers, TrinityCore hit table, ExpectedStat creature armor, spell power from an Intellect main stat, spell variance/versatility, TRIGGER_MISSILE |
| [[level-scaling]] | Retail ContentTuning creature levels: per-viewer level, ExpectedStat health/damage, shared health pool through damage and health multipliers, target-frame level colour; Northshire native-vs-effective pool audit and starter regressions |
| [[banks]] | Character, Warband and guild bank storage (redb), one-transaction moves, player-managed guild ranks, daily reset, logs |
| [[game-objects]] | Guild Vault and Mailbox spawns from content_gameobject, GeoBox reach check, UseGameObject |
| [[warlock-pets]] | Level-3 Imp lifecycle/AI; Retail command-dismiss investigation removes artificial 2641 fixture grant, RED blocked by build-helper API drift |
| [[mounts]] | Mount spells (SPELL_AURA_MOUNTED 78): Mount DB2 capability, flight styles, movement, casting; effect-203 highlight removal; pinned override/form/capability/restriction rows distinguish Skyward Ascent from source-blocked Lift Off 374763 |
| [[stand-state]] | Player sit/sleep/kneel/chair state: requests, stand-up rules, seated food and drink, sitting regen and auto-crit, chairs, item use |
| [[meeting-stones]] | Meeting stone summons: GameObject::Use checks, 59782 cast and channel, TRANS_DOOR ritual portal, SUMMON_PLAYER offer, accept teleport |
| [[new-character-kit]] | TrinityCore Player::Create kit: CharacterLoadout items equipped/stored, playercreateinfo_action bar, server inventory gaps (bags, jewelry, dual wield) |
| [[trade]] | Trade sessions, AzerothCore initiation checks, one-transaction exchange through InventoryModel, refusal keeps the window open |
| [[mail]] | Player and Auction House mail: per-character redb mailboxes, one-transaction send/take/return, delivery delay, expiry sweep, PendingMail |
| [[professions]] | Retail skill lines (parent + expansion child), TDB1210 trainers, crafting through the spell pipeline (reagents, tools, CREATE_ITEM, TrinityCore skill-up chances) |
| [[ecs-storage]] | Bevy ECS as columnar in-memory database: archetype tables, zero-overhead queries, built-in change detection |
| [[swimming-and-breath]] | MH2O terrain liquid (client-matched sampling), swim height bounds, no gravity for swimmers, TrinityCore breath/fatigue mirror timers and drowning damage |
| [[networking]] | Lightyear UDP, pre-entry visibility, matching MCVT server terrain sampling, dirty-candidate replication, spell cast receivers and `Moving` input marker, 50 ms cadence, persistence, channels, and interest management; [replicated player life state](systems/networking.md#player-life-state) |
| [[persistence]] | Frozen pre-source character/equipment ABI, GUID-hydrated cache sources, redb for mutable player data; atomic death-state/HP saves, exact corpse/graveyard relog, legacy-record compatibility and initial-snapshot ordering; physical equipment ownership, login-loaded gold, mutation-boundary GoldUpdateQueue replication, and SQLite for static world data |
| [[content-import]] | AzerothCore quest/vendor/trainer/loot/gossip/item tables in world.db; measured Wrath→Retail ID mapping; Retail-first `content_*` tables from content_policy.py, override CSV, drop report; TrinityCore Retail spawns for maps 0/1; labelled Forever Zephras overlay, source masks/XP, captured NPC displays, labelled missing-class/health estimates, client asset needs and completion gaps |
| [[world-data]] | SQLite world.db, AzerothCore/Retail sources, signed creature-event membership and inactive-event selection, WorldDataPlugin |
| [[class-progression]] | Race/class validation including Monk/DH/Evoker, new-character levels and temporary starts, Initial spec below 10, known spells, action bars and progression persistence |
| [[talents]] | DB2 trait trees per class, TrinityCore-style config validation (visibility, gates, prerequisites, choice/hero nodes, level point budgets), `character_traits`, commit/snapshot messages, talent spells in KnownSpells |
| [[spell-info]] | SpellStore (every DB2 spell, all effects/costs/options at DifficultyID 0, charge categories), ClassSpells, raw power-cost units, load cost |
| [[character-stats]] | Retail 12.x player stats: SimC class/race base stats, CASC hppersta/basemp/combatratings, spec main stat and mastery, `StatModifiers` aura fold-in, base-mana costs, gaps (no gear/weapon data) |
| [[spell-casting]] | [Third local-Retail class script batch](systems/spell-casting.md#third-batch-local-retail-rows), TDB1210-authored fixed destinations and Retail nullable facing, spell teleports/leaps, aura overrides, tested class spell scripts (Dummy bridges, damage/crit modifiers, target plans, removal rewards and live armor), cast pipeline, single-slot 400 ms spell queue with start-time costs, cast bars/channels, GCD/cooldowns/charges, targets/effects, temporary summons and data-bound spell area triggers, combat log, fractional power/rune replication and known-spell check |
| [[auras]] | Mechanic immunity 77, reduced damage pushback 149, absolute movement cap 191; aura runtime: stacking/refresh, periodic ticks, Recuperate full-health completion, procs, interrupt flags, dispel, crowd control, spell modifiers (class mask + SpellLabel), `StatModifiers` fold-in, UnitAuras replication, handled aura IDs |
| [[tick-model]] | Observed server loop: 50ms runner/fixed cadence, schedule ownership, explicit ordering, completed player-death and broad auto-AFK scan reductions, maintenance gates, and IPC boundary |
| [[zone-architecture]] | Maps 0 and 1 plus dungeon copies in one World by `InMap`; map transfers (NewWorld/WorldPortAck), InstanceManager, area trigger teleports; per-zone sets still proposed |
| [[skyborne]] | Forever95/96; [current bounded checkpoint](systems/skyborne.md#rebased-source-and-runtime-checkpoint-2026-10-07): corrective742 native Ailee/Grove functional DONE, CPU13;304 final gate pending, no OS-exit/independent pixel/full parity claim. Import278 preservation/provenance gaps retained; character30/40XP/log[] preserved, physical redb changed; motion a761 fixture-only proof retained; remote consumption/landing open |
| [[ground-assets]] | Registered-map local CASC closure, Forever grid/MPY2 corrections, post-sync 72-tile ground/LOS proof and authored-hole/eye-table limits |
| [[factions]] | `UnitFactionTemplate` sources (creature_template, ChrRaces), FactionTemplate reaction rules, attack/assist checks, Wrath vs Retail ID coverage |
| [[combat]] | Auto-attack system, NPC/player lethal transitions, split death proofs, melee hit table (miss/dodge/parry/glancing/block/crit/hit), spell formulas, stat calculations, DR curves |
| [[creature-ai]] | Three movement types (idle/wander/waypoint by addon path_id), formations, addon mounts/auras, grid activation (GridDormant), SpawnPoint, chase system, 40-yard leash/evade, return-to-spawn |
| [[dungeon-achievements]] | Local CASC categories/criteria, bounded achievement pages/dates, ordered copy objective snapshots and Stockade grants; client hookup |
| [[boss-scripts]] | ScriptName creature AI (CreatureAI hooks, EventMap, BossAI, InstanceScript per copy, creature_text Talk), Stockade boss ports, SmartAI combat timers/victim/CMC chase control, creature casts/ExpectedStat spell damage, dynamic objects |
| [[creature-aggro]] | AggroMode, damage/heal school-modified threat, victim selection, evade and engaged-player threat update/clear delivery |
| [[toys]] | Local Toy.db2 catalog, atomic account learning/favourites, account broadcasts and authoritative UseToy cooldowns; explicit effect/data gaps |
| [[inventory]] | Owner-only snapshots/deltas, GUID-owned Retail/Forever definitions, source-qualified requirements/unique limits, transfers/stacking/equipment scaling, placements, instance tables and parity gaps |
| [[quests]] | Quest content load, AzerothCore accept/prerequisite/marker rules, per-player giver markers, kill and bag-item progress, turn-in rewards, quest log persistence and deltas |
| [[bot-economy]] | Direct suppliers and bounded real merchant CLI/cost/cap proof; auction restart/mail/latency ledger in [[auction-house]], bounded native and final source audits PASS; content gaps in [[economy-catalog]] |
| [[auction-house]] | Global ItemID browse and real AuctionID actions; injected expiry clock/admin advance with reopened settlement proof; exact saved CLI/restart/mail, twelve pinned loopback samples and bounded native and final source audits PASS |
| [[economy-catalog]] | Read-only world.db/TSM importer, gathered/market inputs, supplier contracts and price bounds; combined Classic+Midnight default and blocked native acceptance linked separately |
| [[merchant]] | Vendor lists, limited stock, chronological buyback with physical slots, and equipped/bag item repair with persisted item-guid damage |
| [[npc-interaction]] | Replicated `NpcFlags` (AzerothCore→Retail bits), `InteractNpc` validation (TrinityCore GetNPCIfCanInteractWith, 8.5 yd), gossip menus from world.db, role routing, per-frame session close, auction house gated on an auctioneer session and its faction house |
| [[experience]] | Retail kill XP (gtXp, grey/zero-difference, elite, expansion /10, group split), QuestXP DB2 with low-level reduction, GiveXP level-ups, `character_xp`, health/mana refill, persisted rested pool, faction-aware capital rest and `set-rested`, protocol gaps |

## investigations/

- [[item-healing-scaling]] — Algari zero-heal root cause, item-level RandPropPoints scaling, per-cast/aura context, consumable regressions.

Debug logs and root cause analyses.

- [[general-spell-grants]] — General grant investigation: Attributes_13 bit 16 marks 31 content abilities but also Throw; no verified prerequisite rule, server unchanged.

- [[midnight-economy-content]] — effect-288/Skillup2915 correction, additive Zaralda publication and preserved Classic+Midnight default; scoped catalog/CLI/restart artifact PASS; historical 0/3 superseded by native torso pick then faction-0 AutoAttack; approved Zaralda-only derived template 35 published, source/import gate114 scoped PASS; native-friendly exit0 observed, independent artifact gate116 scoped PASS.

| Page | Summary |
|------|---------|
| [[semantic-noop-replication]] | First- and second-wave server equal-assignment proofs/fixes, Bevy 0.19 change ticks, Lightyear 0.28.0/bevy_replicon sending, direct NPC death transition, Gold ruled out, and blocked wire-level proof |
| [[recurring-loop-elimination]] | Tests-first reductions and eliminations across auction, terrain, friends, auto-AFK, interest, Lightyear, Who roster, gold, durability, calendar, trade-offer, inactive auto-attack, rest-state, and player-death scans; broad death/auto-AFK scans are eliminated while wider scheduler/dirty-boundary requirements remain partial |
| [[server-tick-lag]] | :5000 simulation at 55–70% speed: frame/system instrumentation, dormant-creature scans, per-character save syncs, SipHash interest maps; debug/release/opt-level measurements with 1–3 clients |
| [[tick-loop-audit]] | Exhaustive recurring-schedule audit: 103 registrations / 130 expanded systems; seven narrow removal slices plus player-death and broad auto-AFK scan reductions complete, while preserved Lightyear, interest, friends, and terrain requirements remain partial alongside persistence/order risks and receiver-fan-out measurement |

- [Zaralda observed native success](investigations/midnight-economy-content.md#observed-native-success-task-evidence-2026-10-01) — saved exit0 interaction; source/import and renderer gates distinct; independent artifact gate116 scoped PASS, no whole-world PASS.

- [Reported motion fixture correction](systems/skyborne.md#reported-motion-fixture-correction-2026-10-07) — adjacent-cell fixture fix a7616cd, retained exact GREEN and independent scoped gate; unchanged production/protocol, remote consumer gates open.

- [Auction subcategory/sort proof](systems/auction-house.md#subcategory--current-bid--available--2026-10-08) — class/subclass/inventory alternatives and stable Bid/Quantity paging.
