# Retail follower data and approved spec bridge

Verified: 2026-10-13. [Contract](../../specs/follower-dungeons.md). No authored NPC combat kit was found; the user explicitly approved labelled-spec player spells as a workaround. Source/behavioral implementation is present; user approval of same-map Normal population/entrance reuse resolves the previous admission-data blocker. Both-skin queue/party/first-boss combat is now proved; authored loot, the walked route and full-instance coverage remain open.

## NPC Felguard utility source — 2026-10-13

Demonology summon30146 resolves to real creature17252/Felguard. Its CreatureFamily29 exposes SkillLine761; SkillLineAbility23475 teaches89766/Axe Toss through AcquireMethod2 at native SpellLevel17. Local SpellEffect89766 index0/effect64 triggers347008, whose index0/effect68 is the actual interrupt; index1 is its stun aura. The bridge checks the pet's actual learned action bar, never assigns Spell Lock19647 (Felhunter) or invents an early-level kick. Ordinary cast readiness applies to the pet, not the companion owner, and active hostile-field escape takes priority.

At72e754d5, five actual pet-utility cases pass: level17 Axe Toss interrupts Fireball through child347008 while owner Shadow Bolt continues; level16 has no invented kick; cooldown/non-interruptible/Hammer-of-Justice controls retain ordinary readiness and sourced damage priorities. Row counts are compared to the fixture's real existing baseline, not an assumed empty database.

First RED had two real omissions: no NPC pet dispatch and a fixture catalog replaced by relative-path Startup loading in cargo's package cwd. The fixture now loads authenticated metadata after generic startup and before summon; it never fabricates action-bar entries. The sourced30yd/s axe needs the native166ms minimum5yd flight plus delivery, so a three-frame assertion was premature; the positive case observes its actual landing. Source dispatch086f1e79 uses ordinary pet readiness; no cast/event or persistence bypass. At af677305, ten NPC pet motion/lifetime/credit cases pass with the authenticated catalog: actual collision-clear/exact-ground Stockade follow4yd/chase2yd, missing-bake refusal, authenticated-player damage/kill taps, owner removal, no-bake field refusal, two distinct overlapping Consecrations escaped before either expired without re-entry, and Hammer-of-Justice no-displacement under active escape intent. New field tests run the same FixedUpdate ordering as FollowerPlugin rather than manually substituting a movement result. Prepared source acceptance: all73 follower cases pass at9fe98137 (including all eleven labelled basic-role profiles, player-spec/pet utilities, fields, actual routes, credit and lifecycle);10 player-pet persistence/reopen/logout/refusal cases pass at598e9869, including all six sourced demon summons. Subsequent master mergec962c18a leaves the tested follower/pet/spell/movement implementations and harness unchanged; its zones diff only prepares an unrelated test schema. Current server check/fmt pass atc962c18a. No redundant full test suite was run. Changed Rust was manually audited; pet escape priority is named rather than nested in the main role loop. This remains bounded prepared-runtime proof, not full rotations, authored NPC kits, inherited-stat budgets or dungeon completion.

## Source identity

Local wow build `dcfc90fffd79ba00406ae46f5f657592` is12.1.0.69933. CASC LFGDungeons1361033/layout34B02DE8 has27 difficulty205 entries; pinned Difficulty names205 `Follower`, five players. Width truncation is required when decoding pallet values: Ruby's map is2521, not raw8128985.

Follower identity spells store their class/spec in **Spell.Description_lang**, not NameSubtext_lang. The eleven verified profiles are:

| Template | Identity spell | Follower | Spec |
|---|---:|---|---|
|209057|458306|Captain Garrick|Protection Paladin|
|215517|458309|Warlord Breka Grimaxe|Protection Warrior|
|209072|458313|Crenna Earth-Daughter|Restoration Druid|
|215011|458314|Henry Garrick|Holy Priest|
|209059|458342|Meredy Huntswell|Fire Mage|
|209053|458346|Shuja Grimaxe|Elemental Shaman|
|209065|458348|Austin Huxworth|Beast Mastery Hunter|
|215240|458349|Kybel|Devastation Evoker|
|227734|458660|Bo|Windwalker Monk|
|230861|464927|Lantha Shadeborne|Demonology Warlock|
|232800|469014|Ralia Dreamchaser|Feral Druid|

These are eleven specs across ten classes. No Rogue/Death Knight/Demon Hunter follower identity exists locally. The pinned TDB has9586 creature-template spell rows, none for the37 Expedition Escort templates; these also lack SmartAI/script kits. Garrison/Delves records do not establish dungeon kits. The workaround retires when authenticated NPC kits become available; it is not Retail bot-rotation, gear or stat-budget parity.

## Runtime bridge

`followers/` loads verified profiles and populated difficulty205 destinations, fills the selected player role to1 tank/1 healer/3 damage, uses authenticated templates/models and native labelled-spec passives. Spell458324 is not assigned in the source/runtime addon rows; it must not be inferred from its name. Companion group identities occupy the high half of the runtime group-ID space; they are not persisted characters. Existing party rosters and health updates include the NPCs.

Role AI uses ordinary `creature_cast`, native cooldowns/costs/passives, threat/taunt, chase and owner-follow systems. Tanks recover an ally's attacker with their real taunt and approach melee reach; healers pick the lowest living party-health fraction before damage; damage roles use bounded cooldown/filler priorities. Missing cast-range metadata stops combat instead of substituting melee. The bridge uses native class/spec/level stats and consistent health/mana; it does not infer NPC gear. Holy Priest137031 activates the sourced Flash Heal mana-cost row, so passives must be applied rather than allowing a free heal.

The native Godot host now forwards existing LFG IPC requests on LfgChannel and retains authoritative state/refusals. Send acknowledgment does not mean server acceptance. The later utility and bound-field section owns interrupt/defensive/escape proof. Full pet/guardian rotations, general hazardous-path planning, NPC portraits, relog recovery and full-instance completion remain unestablished.

## Admission data and approved Normal-population bridge

Canonical instance population has now been promoted; it is no longer correct to describe all candidate maps as reference-only. Latest read-only audit is `game-engine/data/diagnostics/follower-ai2-2026-10-11/admission-data-audit.json`.

Ruby Life Pools2436/map2521 has306 runtime spawns, but304 are tagged1/2/8/23 and2 are8/23: **zero205 spawns**. It has authored entrance100030. Conversely, map2830 (Eco-Dome Al'dani2991) has167 actual205 spawns, map2813 has2, maps2652/2669 have1 each, but none has a `tdb_world_safe_locs` entrance. The full pinned TDB world-safe-location table likewise contains no entrance row for those four maps. The prepared private copy and current canonical data therefore have **zero populated205-map/entrance pairs**.

TrinityCore a352b1fa `ObjectGridLoader::LoadN` passes the map's selected difficulty into `GetGridObjectGuids`; ObjectMgr indexes `{mapid, spawnMode}` and inserts each spawn under its declared difficulties. Difficulty's template fallback does not widen population tags. No synthetic entrance or Normal→205 population remap was added.

Local WorldSafeLocs1369425 extraction was attempted once; the current local CASC root has no such content/path. Record this source gap without CDN requests. User subsequently approved Normal population and Normal entrance reuse. Runtime now selects exact difficulty1 membership for creature/game-object populations in205 copies, without modifying source tags or trying an alternate population. Queue/instance identity and creature template/loot difficulty stay205. Ruby is chosen because304 Normal creature rows include Melidrussa Chillworn188252 and entrance100030 is authored. This workaround retires when local205 populations and authored entrances are complete. No static posed capture substitutes for live proof.

## First private live boundary (follower-ai3)

Private5750 admission now creates the1T/1H/3D Ruby party. Newly promoted companion display113757 (Crenna) lacks a runtime eye receipt: her real Regrowth fails `LineOfSight`, as required by the existing fail-closed LOS contract. Source/template SQL alone therefore does not prepare a playable companion. Native Ruby displays likewise expose incomplete appearance coverage. Refresh authenticated eye/appearance receipts privately, never bypass LOS or substitute models. The ignored `crates/los/tests/follower_bake.rs` fixture bakes every pose for the actual Ruby population plus eleven follower templates and reloads the produced table through the runtime reader; its explicit paths keep writes out of canonical caches. Admission is live evidence, not boss/loot or both-skin certification.

Private authenticated data refresh now supplies142 displays/1,420 pose rows through existing M2 eye extraction; strict appearance merge adds138 displays while preserving13,218 existing records. Crenna casts real Regrowth and all four companions follow movement. Missing verified asset receipts still prevent some Ruby models from rendering; direct extracted bytes are not publication receipts.

A second run reproduced wrong difficulty on a character saved inside Normal Ruby: `teleport_to` compared only map IDs, so the205 queue used a near teleport and stayed in copy1/difficulty1. Same-map near teleport now requires the destination's resolved instance to equal the current copy. A loopback transfer/WorldPortAck regression reproduces Normal→205 on the same authored entrance and asserts distinct copy IDs plus actual205 difficulty. This is a deliberate extension of the near-teleport rule for follower admission, not a change to source population tags.

## Combat ownership and reward boundary

A real difficulty205 Juggernaut kill exposed two ownership omissions. Followers copied the owner's faction but did not mark controlled ownership. Their inferred458324 bootstrap also gave type4 invisibility, which the old boolean visibility handling hid from every client and rejected as a hostile spell target. They also lacked `ClientId` (correctly: not persisted characters), and `CreatureTap::record_damage` only recognized direct player attackers, leaving companion-only kills untapped. Neither TDB nor runtime addon data assigns458324 to these templates. Lead explicitly approved removing the inferred application (not stripping an assigned aura or bypassing detection); native labelled-spec passives remain. No invis-detection merge or separate visibility model is part of this fix. Initialization marks controlled ownership; damage attribution resolves `Follower.owner` to the authenticated character before the existing subgroup reward logic. Ordinary unrelated NPCs still grant no player credit. Targeted tests exercise actual lethal Fireball/tap replication and Ruby's real faction16 retaliation boundary.

Source loot remains separate: Melidrussa188252 has lootid0 in base and every imported difficulty, and neither source nor runtime creature loot table has entry188252. Owner attribution cannot create absent authored drops; no loot rows were invented. The native first-boss/loot/both-skin goal remains open.

## Companion-group instance binding

A private same-map reposition reproduced another admission lifecycle bug: the player enters a solo205 copy before the companions form a group. The new group had no recent-instance binding, so another same-map teleport allocated a second205 copy and left the NPCs behind. Group difficulty was already205; the missing state was the group's recent-instance pointer. `join_party` now binds the new group to the already-admitted copy after checking its owner. The active run stores the exact `InMap`, not just map ID, and ends/unsummons its companions if the owner enters a different copy of the same dungeon. Two behavioral regressions reproduce actual party formation→same-map teleport and cross-copy cleanup; no synthetic entrance or geometry change.

## Both-skin first-boss proof — 2026-10-11

Server runtime `ceec9a1b` (binding fix72f4f280), engine329132e3f, protocolb452322. Private UDP5750/redb/world.db; `fb_follower_ai3`, character30 Followproof, Human Mage12. Level12 uses authored scaling/native un-geared stats, not injected combat buffs. Ruby2436/map2521/difficulty205 uses the approved same-map Normal population and entrance100030.

| Boundary | Observed proof | Limit |
|---|---|---|
| Queue/party | Independent Modern and Forever authoritative acceptance; Damage player + Garrick Tank + Crenna Healer + Shuja/Meredy Damage, five online | Other player roles have composition tests, not these rendered runs |
| Follow/combat | Position/health snapshots, rendered companions, Garrick target-of-target and boss swings, Regrowth to NPC tank, Fireball/Fire Blast and Lightning Bolt/Lava Burst | Four companion specs; not full11-profile/rotation/defensive/hazard parity |
| Trash/boss | Juggernaut235,651→0 and Melidrussa860,606→0 in each skin; real cast/damage/death events, five alive, owner kill XP | First boss only; no scripted encounter/full-instance certificate |
| Loot | Both in-range interact/take attempts return no loot window; inventory unchanged (Hearthstone only) | Source/base/all-difficulty lootid0 and zero188252 loot rows; no fabricated drop |
| Route/pixels | Eight inspected1280px PNGs in `/syncthing/AgentShared/2026-10-11/follower-ai/` | Native walk blocked near1434/-180; private admin boss-approach reposition; WMO material, random NPC receipt and shoulder-bone visual gaps remain |

Durable evidence: engine `data/diagnostics/follower-ai3-2026-10-11/`: `{modern,forever}-boss-final-health.log`, `*-boss-death-{combat,roster}.log`, `*-boss-loot-{interact,take}.log`, before/after inventory, authoritative queue snapshots and final client/server logs. One private server/client at a time; both stopped after captures. No canonical database writes, UDP5000 operations, master merge or posed scene. Source-required screenshot output only (PNG) was shared.

Private appearance cache strictly adds138 displays while preserving13,218; existing M2 baker produces142 displays/1,420 eye poses. Maintained frozen-root local-CASC importer publishes768 additive private asset receipts/422 aliases over six fragments. These restore the selected encounter/party render dependencies, not general asset closure. All cache/model/texture/product overlays remain private; no promotion to canonical data. The incomplete loot/walk gates keep the full requested goal open.

## Native class-level and follow navigation — 2026-10-13

All eleven authenticated profiles now have real-source basic-role tests (`profile_behavior_tests.rs`): damage, healer-first triage, both tanks' actual taunts and native resource publication. Initial probe passed10 and crashed Austin's real Cobra Shot193455 during missile resolution: its sourced SpellXDescriptionVariables331 scalar requires `Level`, while NPC initialization supplied only `UnitLevel`. fa4aa3fd adds the exact owner-derived `Level` alongside replicated `UnitLevel`; no default, panic guard or persisted-character identity. Eleven basic-role cases pass. This does not certify full rotations, pets, interrupts or defensives.

Two fixed-order Stockade regressions reproduced actual follow failures: straight movement crossed an authored wall at tick19, and recreating `FollowOwner` every idle AI tick stopped at4.8527yards rather than the actual4yard follow point. 59b264e8 retains in-progress follow state and uses existing baked `Navigation` for follower follow/chase. Missing or blocked bakes hold position, never straight movement; ordinary existing non-follower behavior is unchanged. Three actual-LOS/ground/route-progress/unbaked tests pass.

Ruby lacked static LOS/navigation in the prior private overlay. Existing source collectors now produce all12 authored WDT tiles for private LOS and NAV, zero missing/unreadable geometry. Nav payload is214,541,560bytes. Source data remains unchanged; no authored coordinates/connectors were invented. Inputs are local terrain/WMO/M2 plus read-only world snapshot; private output/hash ledger lives in `/home/osso/.worktrees/follower-dungeons-2026-10-13/ground/` and `ruby-ground-manifest.json`. Canonical publication commands belong only in the lead handoff. New strict-nav/full-route live proof remains pending.

## Native Group Finder catalog — 2026-10-13

`lfg/follower_catalog.rs` answers authenticated `QueryFollowerDungeons` with sorted IDs/names/maps from prepared catalog destinations. Known205 IDs lacking an authored entrance/Normal population are not offered; an empty prepared catalog produces an empty reply, never fabricated choices. Actual registered host-loopback RED→GREEN exercises both populated and empty replies. Existing admission validation remains authoritative. [Native UI and its proof boundary](../../../../game-engine/docs/wiki/systems/follower-dungeons.md) owns the Group Finder role/type/list rendering; no new full-dungeon or all-class claim follows from catalog publication.

## Lead import instructions

Agents prepared SQL only and applied it solely to the explicitly authorized private `world-followers.sqlite`. Lead confirmed canonical import of11 profiles,27 LFG rows,9,586 source spell rows and11 content templates. Agent writes remain private. Existing instance population is separate from follower profile/reference import.

From the server branch, wrap commands in engine `scripts/agent/agent-run follower-ai2`:

```text
python3 scripts/follower_import.py --lfg-db2 <follower-ai diagnostics>/db2/1361033.db2 --world-sql data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql --world-db data/world.db --output <diagnostics>/follower-reference.sql --report <diagnostics>/catalog-report.json
python3 scripts/follower_profiles.py --world-db data/world.db --output <diagnostics>/follower-profiles.sql
```

Prepared artifacts already exist under engine `data/diagnostics/follower-ai2-2026-10-11/`. The lead backs up canonical world.db, inspects both files, then applies reference SQL and profile SQL. The latter reuses the existing Retail template converter and promotes only the eleven named templates/models/addons/equipment plus the explicit bridge profile table. The approved runtime Normal-population bridge resolves admission without canonical spawn-tag changes.

## Proof ledger

- Profile SQL: initial RED; current2/2 behavioral tests pass, including repeated application, source display preservation and refusal to overwrite the source database; no ResourceWarnings.
- Role/executor: current8/8 targeted tests pass atb798d74, retained atceec9a1b (later changes are copy binding/cleanup only). Normal catalog plus actual NPC/game-object spawning:3/3 at7fb41eb. Actual Fireball damage, Hand of Reckoning threat, lowest-health Flash Heal/native cost, native initialization, role composition, source-range refusal and admission-source gating are exercised. The old458324 assertion described an unsupported bootstrap and is superseded by a no-unassigned-aura regression.
- Party integration: loopback roster/health test passes; five members remain online and changed Garrick health reaches the player. This is not rendered party-frame proof.
- Native host/transport:2/2 targeted tests pass, including ephemeral UDP request/refusal round trip and decoded native account state. Source protocol pinnedb452322 for that proof.
- Server cargo check and cargo fmt --check pass. No full suites were run. Evidence logs live in the diagnostics directory, not only slot targets.
- Copy/lifecycle: initial2/2 RED; actual party formation→same-map transfer and cross-copy cleanup2/2 GREEN atceec9a1b. Check/fmt exit0 on current Rust scope; no full suite. Eye-data fixture1/1 reloads all142/1,420 authored rows.
- Live: both-skin first-boss slice above passes; loot source and walked-route gates remain open. All11 specs/full-instance behavior are not certified. UDP5000 and canonical database writes untouched.

## Current strict-nav Ruby boundary — 2026-10-13

Matching private UDP5761 sources: server11ebfd32, engine7218be227, protocol7b76fcfb. Native Modern Group Finder admitted Damage owner30 into difficulty205 copy2 with four companions; real healing restored the owner's health. All24 private Ruby LOS/NAV artifact hashes matched the authored twelve-tile package. This is not full-dungeon proof.

Actual scene positions advanced owner1386.2→1439.6035/128.903/169.75151; companions held1406.0823/127.928696/169.71896. The route query rejected follow1436.7751/128.903/172.57994 and a nearby chase endpoint with no walkable polygon. Party-roster positions were stale: use scene exports and server logs, not that roster snapshot, for movement evidence.

The private diagnostic queried entrance100030→Melidrussa's TDB spawn1551.33/-257.688/129.782. Strict nearest-poly lookup rejected the boss endpoint. Runtime ground and collected collision both reported126.01431 there, with only one unclassified support triangle nearby (the later source-provenance probe identifies WMO4520699 face43953, not terrain). Grounding the destination still yielded an incomplete corridor. Zero missing collector assets does not prove connectivity. No spawn-height change, enlarged snapping window or synthetic connector was made; that initial boundary is superseded by the [authored support/vertical-search repair](../systems/navigation.md#ruby-authored-support-and-vertical-search--2026-10-13); full-instance/live acceptance remains open.

Evidence: `/home/osso/.worktrees/follower-dungeons-2026-10-13/routed-modern/`, `ruby-route-diagnostic.rs`, `ruby-entrance-route-probe.log`, `ruby-entrance-floor-trace.log`, `ruby-grounded-endpoint-trace.log`. Temporary diagnostic code is preserved outside the tracked tests. All owned live processes stopped after captures. Startup-only stale private `tdb_transports` schema and obsolete token/layout ownership were resolved with a read-only canonical SQLite backup and fresh private credential login, not product fallbacks.

## Advanced bridge utilities and bound fields — 2026-10-13

Real enemy Fireball fixtures now prove eight selected spec kicks: Rebuke96231, Pummel6552, Counterspell2139, Wind Shear57994, Counter Shot147362, Quell351338, Spear Hand Strike116705 and Skull Bash106839→93985. The missing Skull Bash DUMMY dispatch follows TrinityCore a352b1fa `spell_druid.cpp:2037–2053`, casting charge221514 then interrupt93985. Charge movement itself remains unproved. A replacement kick validates ordinary range/LOS/power/aura/school/cooldowns before cancelling the follower's active cast; unavailable kicks preserve it and normal damage priorities. Twelve utility cases passed atc4394e7a.

Eleven profiles also execute their class's ordinary defensive when wounded and actively targeted. Native damage after the cast proves the authored mitigation, immunity or Desperate Prayer healing, not merely spell selection. This bridge decision is not an authenticated NPC health-threshold policy. Twelve cases passed at64523113; the combined prior/utility source gate passed68/68 atb2bed3f3.

Bound `GroundPulses` and active `SpellAreaTrigger` actions provide field centers/radii/allegiance/lifetime. Escape considers eight supported perimeter candidates, requires complete baked corridors, rejects re-entry into hostile fields, and writes ordinary movement/interest/replication. No direct-movement or teleport substitute exists. Combat/follow targets still inside a field hold approach. Real hostile Consecration during combat originally held the follower in danger (RED); current Stockade proof exits with collision-clear steps, holds outside during continued combat, ignores friendly Consecration and stays put without a bake:3/3 atcee9c693. Ordinary chase paths crossing a field from outside, overlapping-field destinations, separately scripted shapes, cast/CC movement boundaries and live both-skin hazard behavior remain unproved; these bounded fixtures do not close full-dungeon acceptance.

Logs: `/home/osso/.worktrees/follower-dungeons-2026-10-13/utility-{interrupt,casting,defensive}-*.log`, `server-utility-gate-b2bed3f3.log`, `hazard-stockade-{combat-red,green,boundaries}.log`. No canonical data or shared-realm process was changed.

## Sources

- [Reference importer](../../../scripts/follower_import.py), [profile importer](../../../scripts/follower_profiles.py), [SQL tests](../../../scripts/tests/test_follower_profiles.py)
- [All labelled profiles](../../../crates/server/src/followers/profile_behavior_tests.rs), [real follow regression](../../../crates/server/src/followers/navigation_tests.rs), [native Cobra Shot scalar](../../../crates/server/src/spell_cast/scripts.rs)
- [Group Finder catalog](../../../crates/server/src/lfg/follower_catalog.rs), [catalog loopback](../../../crates/server/src/followers/wire_tests.rs)
- [Runtime bridge](../../../crates/server/src/followers/mod.rs), [group presence](../../../crates/server/src/group/mod.rs), [member health](../../../crates/server/src/group/member_state.rs)
- Local pinned Spell/Difficulty CSVs, canonical read-only world.db, full TDB1210.26091 SQL, diagnostics receipts
- [TrinityCore a352b1fa grid loader](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Grids/ObjectGridLoader.cpp#L95-L103), [exact difficulty key](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Globals/ObjectMgr.cpp#L2180-L2216)
- Retail Blizzard_APIDocumentationGenerated/LFGInfoDocumentation.lua: IsLFGFollowerDungeon/IsInLFGFollowerDungeon

## See also

- [Follower contract](../../specs/follower-dungeons.md)
- [Content import](../systems/content-import.md)
- [Groups](../../specs/groups.md), [navigation](../../specs/navigation.md), [creature aggro](../../specs/creature-aggro.md)

## Stockade diagnostic attachment — 2026-10-13

The lead authorized Stockade diagnostic full-party completion before Azure sourced partial work. `followers/stockade_diagnostic.rs` reads one named request from `GAME_SERVER_STOCKADE_FOLLOWER_DIAGNOSTIC` at startup. The approved diagnostic initial admission sends the authenticated solo Normal owner through ordinary MapTransfer/WorldPortAck to the real WorldSafeLoc3599; missing source or refused admission fails explicitly. Only after ACK, or for an already admitted Normal map34 owner, it summons the source-selected four profiles using the production initializer and joins their ordinary companion group without further movement, a Retail catalog row or an LFG queue. The separate diagnostic component owns exact-copy lifetime; existing follow/combat systems remain unchanged. Group presence now takes companion difficulty from the owner rather than hardcoding205. Shared summon/rollback and group join/leave primitives serve both paths. [Acceptance and limits](../../specs/follower-dungeons.md#stockade-diagnostic-completion-not-retail-admission). Bootstrap and full walked completion are distinct proof gates; the latter is not yet established.
