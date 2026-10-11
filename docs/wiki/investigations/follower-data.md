# Retail follower data and approved spec bridge

Verified: 2026-10-11. [Contract](../../specs/follower-dungeons.md). No authored NPC combat kit was found; the user explicitly approved labelled-spec player spells as a workaround. Source/behavioral implementation is present; user approval of same-map Normal population/entrance reuse resolves the previous admission-data blocker. Dungeon entry and both-skin live completion still require proof.

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

`followers/` loads verified profiles and populated difficulty205 destinations, fills the selected player role to1 tank/1 healer/3 damage, uses authenticated templates/models and applies aura458324. Companion group identities occupy the high half of the runtime group-ID space; they are not persisted characters. Existing party rosters and health updates include the NPCs.

Role AI uses ordinary `creature_cast`, native cooldowns/costs/passives, threat/taunt, chase and owner-follow systems. Tanks recover an ally's attacker with their real taunt and approach melee reach; healers pick the lowest living party-health fraction before damage; damage roles use bounded cooldown/filler priorities. Missing cast-range metadata stops combat instead of substituting melee. The bridge uses native class/spec/level stats and consistent health/mana; it does not infer NPC gear. Holy Priest137031 activates the sourced Flash Heal mana-cost row, so passives must be applied rather than allowing a free heal.

The native Godot host now forwards existing LFG IPC requests on LfgChannel and retains authoritative state/refusals. Send acknowledgment does not mean server acceptance. Full pet/guardian rotation, interrupts/defensives, hazard avoidance, NPC portraits, relog recovery and full-instance completion are not established by these bounded proofs.

## Admission data and approved Normal-population bridge

Canonical instance population has now been promoted; it is no longer correct to describe all candidate maps as reference-only. Latest read-only audit is `game-engine/data/diagnostics/follower-ai2-2026-10-11/admission-data-audit.json`.

Ruby Life Pools2436/map2521 has306 runtime spawns, but304 are tagged1/2/8/23 and2 are8/23: **zero205 spawns**. It has authored entrance100030. Conversely, map2830 (Eco-Dome Al'dani2991) has167 actual205 spawns, map2813 has2, maps2652/2669 have1 each, but none has a `tdb_world_safe_locs` entrance. The full pinned TDB world-safe-location table likewise contains no entrance row for those four maps. The prepared private copy and current canonical data therefore have **zero populated205-map/entrance pairs**.

TrinityCore a352b1fa `ObjectGridLoader::LoadN` passes the map's selected difficulty into `GetGridObjectGuids`; ObjectMgr indexes `{mapid, spawnMode}` and inserts each spawn under its declared difficulties. Difficulty's template fallback does not widen population tags. No synthetic entrance or Normal→205 population remap was added.

Local WorldSafeLocs1369425 extraction was attempted once; the current local CASC root has no such content/path. Record this source gap without CDN requests. User subsequently approved Normal population and Normal entrance reuse. Runtime now selects exact difficulty1 membership for creature/game-object populations in205 copies, without modifying source tags or trying an alternate population. Queue/instance identity and creature template/loot difficulty stay205. Ruby is chosen because304 Normal creature rows include Melidrussa Chillworn188252 and entrance100030 is authored. This workaround retires when local205 populations and authored entrances are complete. No static posed capture substitutes for live proof.

## First private live boundary (follower-ai3)

Private5750 admission now creates the1T/1H/3D Ruby party. Newly promoted companion display113757 (Crenna) lacks a runtime eye receipt: her real Regrowth fails `LineOfSight`, as required by the existing fail-closed LOS contract. Source/template SQL alone therefore does not prepare a playable companion. Native Ruby displays likewise expose incomplete appearance coverage. Refresh authenticated eye/appearance receipts privately, never bypass LOS or substitute models. The ignored `crates/los/tests/follower_bake.rs` fixture bakes every pose for the actual Ruby population plus eleven follower templates and reloads the produced table through the runtime reader; its explicit paths keep writes out of canonical caches. Admission is live evidence, not boss/loot or both-skin certification.

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
- Role/executor/catalog: current7/7 targeted tests pass. Actual Fireball damage, Hand of Reckoning threat, lowest-health Flash Heal/native cost, authored458324 initialization, role composition, source-range refusal and admission-source gating are exercised.
- Party integration: loopback roster/health test passes; five members remain online and changed Garrick health reaches the player. This is not rendered party-frame proof.
- Native host/transport:2/2 targeted tests pass, including ephemeral UDP request/refusal round trip and decoded native account state. Source protocol pinnedb452322 for that proof.
- Server cargo check and cargo fmt --check pass. No full suites were run. Evidence logs live in the diagnostics directory, not only slot targets.
- **Missing:** UDP5750 dungeon queue→entry→trash/boss completion and inspected Modern/Forever PNGs. No rendered client/private game server was started; UDP5000 and canonical database writes were untouched by this agent.

## Sources

- [Reference importer](../../../scripts/follower_import.py), [profile importer](../../../scripts/follower_profiles.py), [SQL tests](../../../scripts/tests/test_follower_profiles.py)
- [Runtime bridge](../../../crates/server/src/followers/mod.rs), [group presence](../../../crates/server/src/group/mod.rs), [member health](../../../crates/server/src/group/member_state.rs)
- Local pinned Spell/Difficulty CSVs, canonical read-only world.db, full TDB1210.26091 SQL, diagnostics receipts
- [TrinityCore a352b1fa grid loader](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Grids/ObjectGridLoader.cpp#L95-L103), [exact difficulty key](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Globals/ObjectMgr.cpp#L2180-L2216)
- Retail Blizzard_APIDocumentationGenerated/LFGInfoDocumentation.lua: IsLFGFollowerDungeon/IsInLFGFollowerDungeon

## See also

- [Follower contract](../../specs/follower-dungeons.md)
- [Content import](../systems/content-import.md)
- [Groups](../../specs/groups.md), [navigation](../../specs/navigation.md), [creature aggro](../../specs/creature-aggro.md)
