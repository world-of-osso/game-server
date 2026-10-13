# Dungeon achievements

Server loads pinned local Retail DB2 criteria, tracks copy-specific defeated encounters, sends achievement snapshots/toasts and dungeon objectives, and serves bounded achievement catalog pages on request. [Contract](../../specs/dungeon-achievements.md).

## Target-creature alternatives — verified 2026-10-13

Pinned TC1738–1750 Some8 with Amount0/1 requires one successful child; TargetCreatureId4 reads the request's native unit (1782–1785). Compiler preserves every plain SingleTrue target-entry alternative for DoEmote54 as exact set membership, not AND and not an unconditional event. Native63288/64798 Love requests each credit20817 independently; wrong/foreign/player/absent targets and Wave reject. Six authored Wave selectors grant account40624; idle/reopened counters are covered. Higher amounts, mixed/nested/unknown children and other producer types remain disabled. IDs are sourced selectors, not certification of authored NPC spawns/models. [Inventory](../../achievement-criteria-producer-inventory.md).

## Single-child Some trees — verified 2026-10-13

TC1738–1750 Some8 requires max(Amount,1) successful children. With Amount0/1 and exactly one child, compiler uses the existing All-child evaluator without new runtime fields/groups. Wrong target, map, player and absent target remain negatives; native channel requests prove68751/3236 and controlled18-unit account40475. Pinned criteria authenticate target IDs, not currently missing NPC spawn/model data. Other multi-child non-difficulty OR and unknown children stay disabled; the bounded target-creature exception is described above. [Inventory](../../achievement-criteria-producer-inventory.md).

## Empty All modifier trees — verified 2026-10-13

TC a352b1fa CriteriaHandler1733–1737 returns true after looping over zero children. Compiler accepts only plain empty All4; empty Some8/SingleTrue2, nonplain roots and other unsupported conditions still reject. Native127117–127123 triggered casts produce each authored goggle77533–77539 and account18905, with refused depth and reopened state. Canonical content_item lacks those seven rows: private fixture projects exact existing Retail Item/ItemSparse metadata through `content_policy.ITEM_COLUMNS` without changing costs/effects or canonical data. This is cast/predicate/persistence proof, not the full production engineering flow. [Inventory](../../achievement-criteria-producer-inventory.md).

## Exact reference-player levels — verified 2026-10-13

Modifier39 (TC a352b1fa CriteriaHandler1956–1959) compiles as the existing69 minimum and71 maximum with the same value; no new runtime field or comparator. Native Unit::Kill at levels1/2/3 proves orphan16188/16402 exactness, lower/higher negatives and reopened persistence. Twenty-four additional raw rows become evaluable while current-root census stays unchanged; internal/title21 and orphan relinking remain disabled. [Inventory](../../achievement-criteria-producer-inventory.md).

## Instance-death capacity — verified 2026-10-13

Native accepted player deaths emit18 only when MapStore says IsDungeon and InstanceManager has the current copy. `copy_capacity` exposes its real admission maximum; no occupancy or difficulty-to-group-size mapping. TC a352b1fa Player1164/CriteriaHandler1481–1493 requires that exact value. Unit::Kill fixtures allocate actual33/party5 and533/raid10/25 copies, reject continents/fake copies and other capacity/modifier counters, and preserve duplicate/idle/reopened state. [Inventory](../../achievement-criteria-producer-inventory.md) records three structural statistics, not rendered raids or certified delves.

## Solo group predicates — verified 2026-10-13

Modifier35 tests existing `ReferencePlayer.group_size == Some(0)`; no new state channel. The size comes from authoritative GroupRegistry plus character identity, includes offline members and distinguishes unknown from solo (TC a352b1fa CriteriaHandler1940–1943). Native triggered marker280339 casts prove41658/13026, real invite/join exclusion, missing-authority and depth negatives, idle/reopen persistence. This proves cast/predicate semantics, not the missing Nazmir timed-invader quest trigger. [Inventory](../../achievement-criteria-producer-inventory.md).

## Player zone predicates — verified 2026-10-13

Modifier41 reuses `AreaTable::zone_id` at catalog load to compile accepted current-area IDs. Unlike modifier17's descendant closure, this uses exactly one flagged subzone parent (TC a352b1fa CriteriaHandler1964–1973). Reference-player terrain area is sampled at credit time; missing area rejects. Native pet-battle settlement at real Stormwind terrain proves criterion19845, loser/wrong-city exclusion, idle stability and redb reopen. Catalog tests preserve unflagged-parent and two-level rejection. [Inventory and census](../../achievement-criteria-producer-inventory.md) remain structural, not city content certification.

## Data and provenance

Verified 2026-10-06: `/mnt/c/World of Warcraft/.build.info` names retail build `12.1.0.69933`, build key `dcfc90fffd79ba00406ae46f5f657592`. Local community listfile maps Achievement=1260179, Criteria=1263817, CriteriaTree=1263818, Achievement_Category=1324299. Phase 2 extracts Category from the same local build to server data; layout `67B2B4BD`, 243 rows. Achievement display metadata adds Category column 7 and IconFileID column 12. Category columns are Name/ID/Parent/Ui_order; the relationship repeats Ui_order, not a parent ID.

Extraction from the game-engine working directory, using its existing casc-local executable under agent-run (no CDN/web, outputs only to server data):

```sh
scripts/agent/agent-run dungeon2 target/release/casc-local 1260179 1263817 1263818 1324299 -o /home/osso-test/.worktrees/game-server-recup/data/db2/12.1.0.69933
python3 /home/osso-test/.worktrees/game-server-recup/scripts/import_achievements.py --db2-dir /home/osso-test/.worktrees/game-server-recup/data/db2/12.1.0.69933 --db /home/osso-test/.worktrees/game-server-recup/data/world.db
```

Layout hashes: Achievement `6FC5281B`, Criteria `53D03513`, CriteriaTree `FF97AE12`. Achievement / Category / Criteria / CriteriaTree contain 14,081 / 243 / 66,340 / 120,821 rows (copies included). Missing TACT key `CF9022A9913B1B4D` leaves one Criteria and eight CriteriaTree records unavailable; decoder reports zero-filled sections and skips them. Supported achievement trees require every referenced row to exist. Stockade and level rows are present.

Existing `scripts/db2.py extract` uses `db2_casc.py`; this importer follows that decoder path. Engine core's lighting reader is table-specific, not a general CSV exporter. Adjacent `.csv.import` files are Godot's `csv_translation` metadata, not DB2 export tools. No generated import/uid files are needed. Phase-1 binaries remain under game-engine `data/db2/12.1.0.69933/`; phase 2 copies these to server `data/db2/12.1.0.69933/` alongside the locally extracted Category and all CSV exports. None is committed. SQL reference tables live in server `data/world.db`; `db2_build` stamps all four tables. Tests load concrete imported SQL rows; `sync_test_data` already mirrors `game-server/world.db`, so they need no additional CSV mirror entries.

## Runtime

`MapStore::encounters_for_map` filters map/difficulty (0/-1 apply to all difficulties) and sorts OrderIndex/ID. Each copy's existing LockData keeps script DONE bits separately from DB2 encounter bits. Every successful SetBossState(DONE) updates masks before achievement evaluation; unchanged DONE and wipes do not credit. Scheduled locks retain both masks in the existing redb instance-lock table. Normal Stockade checkpoints retain owner/map/difficulty/copy identity and both masks in `normal_instance_locks`, saved in the same transaction as scheduled locks. Startup reserves checkpoint IDs and resumes the owner's copy. Normal checkpoints do not impose scheduled binds or block reset: manual reset and copy unload remove them.

Stockade map 34 Normal has encounters 1144/1145/1146 at OrderIndex 1000/2000/3000, bits 3/4/5. Script boss indices run in a different order. Achievement 633, “Stormwind Stockade”, uses root 3385 (Any, amount 1), child 3386 (Single, amount 1), criterion 18527 (kill creature 46254 Hogger). Phase 1 adds the requested full-dungeon grant boundary: even when Hogger dies first, its achievement toast waits for the copy's entire encounter mask. This full-run boundary is a server requirement, not a claim that Retail's Hogger-only criterion itself requires all bosses.

Supported counters: kill creature type 0 accumulates, reach-level type 5 retains highest level, encounter type 165 accumulates matching DungeonEncounter IDs. Scripted bosses credit their configured creature on successful encounter completion; ordinary death credit excludes these bosses to avoid double counting. Current progress and earned IDs persist per character as JSON in existing redb `achievements`. Old JSON rows without counters undergo a one-time migration of placeholder IDs 1–6 to the actual earned Retail level IDs; the rewritten row includes counters and is never remapped again.

DB2 Retail level rows: 6/7/9/14783/15805/19459 for levels 10/20/40/60/70/80; these are not the old placeholder 1–6. Additional active Retail level rows (30,45,50) are also data-driven.

## Quest and earned-achievement criteria (slice 1, 2026-10-10)

`quest_requests::choose_reward` queues achievement credit only after its inventory/quest-log transaction commits. Deferred execution resolves the authenticated character's live player, updates type27 matching quest IDs to 1 and persists the achievement snapshot/date before sending existing toast messages. Login replays persisted rewarded quests, recovering pre-feature history or a process exit between reward commit and deferred credit; login suppresses toasts. No redb schema migration is needed: the existing JSON counters/dates and quest-log rewarded IDs already represent this state.

Type8 counters derive directly from earned IDs rather than a second stored counter. Grant evaluation repeats until no additional definitions complete, supporting multi-level dependency chains independently of ID order without duplicate grants. Missing-row, conditional, timer and unsupported-tree exclusions remain unchanged. Faction and account-mask corrections below supersede the historical phase-1 exclusions. No protocol/client change or world.db addition is needed.

Faction matching uses `faction_template_for_race` and imported FactionTemplate group bits (Alliance2/Horde4), avoiding the older coarse race-team helper. Grant, snapshot and catalog/criteria eligibility use the same raw Achievement.Faction encoding: -1 both,0 Horde,1 Alliance. `DBCEnums.h:69-74` and `AchievementMgr.cpp:97-102` at TrinityCore `a352b1fa` confirm encoding/rejection. Retail Lua enumerates faction-filtered native API rows (`Mainline/Blizzard_AchievementUI.lua:862,1019,1235`); it does not implement numeric faction mapping itself.

Retail `Blizzard_FrameXMLBase/Constants.lua:123` defines `ACHIEVEMENT_FLAGS_ACCOUNT = 0x00020000`; AchievementUI `:1243` tests that bit. The old0x10000 exclusion was SHOW_CRITERIA_MEMBERS, not account scope. Slice1 excluded0x20000 and kept account rows unevaluated while retaining historical JSON. Slice2 below replaces that exclusion with account aggregation; character counters are never presented as account totals. Stockade633 remains character-scope (flags0).

Corrected pinned census: old0/5/165, both-faction207 (all factions225); new0/5/8/27/165 gives common492, Alliance564, Horde559, union631. Types27 and8 separately add357 and47 against the corrected all-faction baseline; their combination adds406. Earlier561→2135 figures used the wrong account mask and are historical, not final coverage.

Source: TrinityCore `a352b1fa`, `src/server/game/Achievements/CriteriaHandler.cpp:1446-1449` checks the required earned achievement; `:1503-1516` matches quest ID and checks rewarded quest status at login. The following CriteriaDataSet checks remain outside this DB2-only evaluator. Retail `Blizzard_AchievementUI/Mainline/Blizzard_AchievementUI.lua` remains the existing presentation source; Forever changes artwork only. Native proof and final trackable counts are recorded in the slice handoff, not implied by source inspection.

## Account aggregation and additional criteria (slice2, 2026-10-11)

`achievement_account.rs` writes the character contribution and `account_achievements_v1` JSON in one redb transaction. Lazy migration reads all current account characters' historical achievement JSON without rewriting sibling rows. Contributions are retained after character deletion. Additive kill/cast/target/encounter counters sum across contributions; boolean quest facts and absolute level/skill values take max. Earned IDs survive, and known dates take the earliest recorded value; unknown dates remain unknown. A presentation-only `account` field is skipped by character serialization. Account dependencies may consume any character's earned IDs; character dependencies consume their own IDs and only account-flag IDs from the aggregate. Cross-scope grants iterate to a fixed point.

Types29 and28 require locally imported spell IDs and match that exact asset. Cast credit occurs only after cast/target/resource validation; valid triggered casts also credit. Target credit uses distinct hit recipients after missile/defense filtering, before effects, not the caster. Type7 requires an authored local SkillLine and sets the current nonzero base rank on load/change, not its cap or an increment. Pinned primary references: TrinityCore a352b1fa `Spell.cpp:3541–3550,2876–2880`; `CriteriaHandler.cpp:601–604,1463–1469`. Local TDB1210 `criteria_data` adds holiday404 to cast criteria11118–11127; these remain disabled. Criterion5258 has data_type0, not an added condition.

Trackable structural union: slice1 **631**; account support **2584** (631 character/1953 account); +29 **2882** (704/2178); +7 **3151** (928/2223); +28 **3388** (1050/2338). These counts describe conditional-free trees, not playable content or full Retail parity. Types43/46 leave3388 unchanged: no local WorldMapOverlay-to-AreaID mapping is imported/exported, and current shared reputation values use Hated0/Neutral12000/Exalted63000 rather than Retail signed standing. Do not treat exploration assets as AreaTable IDs or shared63000 as a Retail standing. Primary requirements: `CriteriaHandler.cpp:1493–1509,686–691`.

Tests include real4899 quests split across account characters, unchanged sibling persistence, isolation, duplicate suppression/reopen, historical dates/counter aggregation/deletion, native43779 cast and24733 target paths, skill299→250/max450, and explicit627/944 unsupported-grant refusal. Durable RED/GREEN logs and read-only census live in canonical game-engine `data/diagnostics/achievements2-2026-10-11/`; slice handoff carries the final revision/proof ledger. Existing protocol carries the aggregate snapshots/pages/toasts; no client/protocol edit.

## CriteriaTree coverage

No TrinityCore checkout is available under `~/Repos`. Pinned primary source was reviewed for event, faction and flag semantics above; the remaining operator interpretation below uses observed local DB2 fields. Operator 0 compares a criterion counter with Amount; 4 requires all children; 8 requires Amount completed children; 7 counts direct children with positive criterion counters. Operators 5/6/9 aggregate descendant counters by sum/max/weighted sum. Supported catalog trees exclude missing references, cycles, unknown/inverse operators, unsupported criterion types and conditional/timed criteria. These interpretations are inferred DB2 semantics, not independently source-verified Retail parity. Concrete level and Stockade trees anchor the All/Any behavior; count tests distinguish positive progress from completed thresholds.

## Proof ledger

- Slice2 code/test revision `4109b84`, after merging origin/master at `05dab72`: `agent-run ach2 cargo test --locked -p server --bin game-server -- achievement dungeonach --nocapture` **31 passed/0 failed**,13.29s. `cargo fmt --all -- --check` and `cargo check --locked -p server`: exit0, no warnings. Logs: canonical game-engine `data/diagnostics/achievements2-2026-10-11/final-{achievement-tests-corrected,format,check}.log`. First integration run29/31 failed solely because two old tests compared transient account presentation with durable character JSON; updated assertions prove account reconstruction and unchanged character serialization. No full suite or live/deploy claim.

- RED: baseline `c8cd38f` + Retail-ID assertion failed because placeholder achievement 1 was granted instead of Retail 6. `/tmp/claude/dungeonach-red.log`.
- `702cb80`: detached build-lock + agent-run + desktop-server-build with explicit `--build-host local`, filter `dungeonach`: eight tests passed; Stockade fixture lost its toast by draining after two frames. The Lightyear host receiver clears each frame (`lightyear_messages-0.28.0/src/receive.rs`, `clear_typed`). Saved `/tmp/claude/dungeonach-second-green.log`.
- `96cad53`: only that fixture changed to drain both frames. Same locked detached runner, filter `dungeonach_stockade_real_boss_deaths_record_ordered_progress_and_send_final_toast`: PASS, including real scripted boss deaths, ordered masks, copy isolation, final toast/snapshot, duplicate DONE and persisted character state. `target/server-test.log`, `/tmp/claude/dungeonach.out`.
- `702cb80`: native helper `--build-host local --check -p server`: PASS without warnings, 78s. Later fixture-only edit does not invalidate production check. `/tmp/claude/dungeonach-check.out`.
- `96cad53`: `cargo fmt --all -- --check`: PASS. Existing WDC5 decoder's `test_decrypted_sections_and_virtual_string_offsets`: PASS; later changes are Rust-only.

The eight other passing tests cover actual level thresholds, real encounter asset matching, All/Any/direct count, character progress reopen, one-time placeholder migration, scheduled lock restore and normal Stockade checkpoint restore/manual reset. These phase-1 proofs used no broad suite, protocol edits, push or merge.

## Phase 2 runtime and client handoff

[Client hookup contract, bounds and pinned Retail file:line references](../../dungeon-achievements-client.md). `achievement_window.rs` keeps full presentation metadata separate from the supported grant catalog: listing an unsupported achievement never enables its criteria. Fixed ID-cursor pages include the first criteria page, plus a separate continuation request. Criterion descriptions and order come from CriteriaTree. Catalog data is request-only; old snapshot/toast delivery remains unchanged.

`dungeon_progress.rs` detects new InWorld and copy changes, broadcasting a full replacement snapshot to authenticated in-world connections in that copy. The successful encounter hook sends after the mask is saved. Duplicate DONE is already rejected upstream. Encounter names/flags load from the existing localized DungeonEncounter table. Maximum 32 rows per applicable map/difficulty is validated at startup. Optionality is explicitly unknown: no verified optional Flags bit is assumed. DungeonEncounter also contains continent world bosses (map 0, Nathanos Blightcaller 2416). Snapshot rows must come from an actual InstanceManager copy, not from a map lookup with an invented difficulty 0; otherwise leaving Stockade incorrectly displays that world boss. The transfer fixture reproduces this with real DB2 data.

Newly earned character IDs acquire `earned_at` Unix seconds from ServerClock at the gameplay boundary and persist with counters. Old saved JSON lacks dates; serde defaults the map empty, preserving unknown dates rather than inventing history. Catalog evaluation remains pure; only the world event boundary reads the clock.

Phase-2 RED: locked detached local helper compiled the new Stockade/continuation tests against missing messages/page method and failed with unresolved imports and missing `page`, exit 101. Python import RED lacked achievement_category; GREEN returned the concrete Stockade category 14808 (Classic, parent 168), icon 134163, 10 points and 243 categories. The existing missing encrypted records still produce explicit decoder warnings; none affects these fixtures. Rust GREEN and final gates. Helper tests ran from game-engine, detached and polled, with the exact prefix:

```sh
/home/osso-test/.worktrees/build-lock.sh env DEPOT_SIBLING_SHARED_PROTOCOL=/home/osso-test/.worktrees/shared-protocol-dungeon scripts/agent/agent-run dungeon2 python3 scripts/desktop-server-build.py --root /home/osso-test/.worktrees/game-server-recup
```

Append each row's arguments. The protocol test uses `--manifest-path /home/osso-test/.worktrees/shared-protocol/Cargo.toml`: the helper snapshots the selected dungeon slot into that Docker-only sibling path, not the host's other protocol checkout.

| Proof | Exact scope | Result / artifact |
|---|---|---|
| Locked detached helper `--build-host local --test -p server --no-fail-fast -- dungeon2` | server `46bdcd6`, protocol `f115ea5` | 5 passed: real Stockade entry, all ordered kills, both members/copy isolation, new connection/entity loaded from persisted state, all three catalog request variants, metadata/earned date, two-page categories/achievements/criteria, unknown requests, legacy dates and UTF-8 bounds. `/tmp/claude/dungeon2-final.out` |
| Same helper, filter `dungeonach` | `46bdcd6` + `f115ea5` | 9 passed, including Stockade plus eight phase-1 threshold/criteria/persistence/lock regressions. `/tmp/claude/dungeon2-phase1.out` |
| Same helper, filter `stockade --skip dungeon2` | `46bdcd6` + `f115ea5` | 24 passed; existing `stockade_guard_wields_its_virtual_items`, `stockade_models_are_chosen_by_probability`, and `stockade_spawns_take_their_addon_pose` remain ignored. `/tmp/claude/dungeon2-stockade.out` |
| Same helper, standalone protocol manifest and `--test dungeon2_wire -- dungeon2 --nocapture` | protocol `f115ea5` | 4 passed; maximum category payload 56,397 bytes. `/tmp/claude/dungeon2-wire.out` |
| Same helper, filter `dungeon2_leaving_instance` | server `fb4f44c` | Behavioral RED: a continent snapshot wrongly includes world boss 2416. `/tmp/claude/dungeon2-clear-red2.out` |
| Same helper, filter `scripted_ai::tests::dungeonach::dungeon2` | server `b937cb3`, protocol `f115ea5` | 2 passed: Stockade lifecycle/reconnect/category and continent clear. This re-proves the changed snapshot scope. `/tmp/claude/dungeon2-runtime.out` |
| Locked agent-run native `scripts/build-host.py --build-host local --check -p server` with explicit Cargo patch overrides for shared and its vendored netcode | `b937cb3` + `f115ea5` | PASS, no warnings; log confirms both exact protocol-slot paths. `/tmp/claude/dungeon2-check-final.out` |
| `cargo fmt --manifest-path <slot>/Cargo.toml --all -- --check` (protocol), `-p server -- --check` (server), under agent-run | `b937cb3` + `f115ea5` | Both PASS. Manual changed-Rust readability audit: no suppressions or new complexity/nesting violations; new production functions at most 30 lines. |

The standalone native check uses `--config 'patch."https://github.com/world-of-osso/shared-protocol.git".shared.path="/home/osso-test/.worktrees/shared-protocol-dungeon"'` and `--config 'patch.crates-io.lightyear_netcode.path="/home/osso-test/.worktrees/shared-protocol-dungeon/vendor/lightyear_netcode"'`. Do not use Cargo `paths` overrides (they emitted dependency-graph warnings), or add `--locked` again (the native helper already injects it). Neither workaround is retained in source/config. Import code stays at `7cb75b9`; presentation/criteria code stays at `46bdcd6`. The later snapshot-only change does not invalidate their proofs. Final docs-only edits do not invalidate code evidence. No broad suite, client execution, push, merge or deployment.

## Full raw producer continuation (2026-10-12)

[Producer inventory and current-root coverage](../../achievement-criteria-producer-inventory.md) supersedes historical slice counts and records every raw69933 type, current wiring, source/state gaps and proof ledger. The [contract](../../specs/dungeon-achievements.md) owns per-type semantics.

Earned player levels are published at the native XP persistence boundary, not from `Changed<Level>`; generated max-level class predicates admit actual current tree leaves without stripping conditions. Area transitions use authored full ancestry for IsInArea but retain the separate one-parent terrain-zone rule. Traditional faction rank totals exclude friendship, renown and paragon-target state. Raw CriteriaTree references outside all Achievement roots remain orphan counters, not artificial achievement gains.

## Rated arena win criteria (verified 2026-10-13)

`pvp_arena_rated_results::apply_result` collects online winners while persisting their ratings, sets the match result’s once-only `applied` marker, then publishes Type37 credit through the authenticated achievement boundary. Pre-result rating follows TC Arena.cpp249 (minimum1); the criterion increments by one, not the rating amount. Native-channel 2v2/3v3 tests cover winner/loser isolation, skirmishes, no queue/preparation credit, idle replay and redb reopen. Type39 remains disabled: pinned TC uses team sizes2/3/5 while current Retail assets are slot-like0/1/2; mapping is unresolved. [Producer inventory](../../achievement-criteria-producer-inventory.md) owns the current census and remaining gaps.

## Current-head live sample (verified 2026-10-13)

At server `33a58e14`, engine `ebedc08ab`, protocol `98f1bd9`, fresh Modern and Forever clients on private UDP5898 paid for Ring/Bear/Mammoth, earned2084/2076/2078, advanced actual known-mount progress2→4 and compared against a separate online peer (own Completed, peer Incomplete). Sixteen1280×720 PNGs reside in `/syncthing/AgentShared/2026-10-12/achievements/criteria17/`; logs and source-placement receipt are under the achievements-criteria logs’ `live/criteria17/`. This is a commerce/mount/comparison sample, not proof of every enabled criterion; arena37 has separate real-channel persistence proof. All owned processes stopped.

## Identity PlayerCondition criteria (verified 2026-10-13)

The criteria condition compiler now carries ModifierTree type2 PlayerCondition IDs. Catalog loading delegates to `game_objects::identity_conditions::load_condition`, retaining every flag/non-identity/TDB source34 restriction; `permits_race_class` is shared by object interaction and criteria evaluation. No second mask implementation or general PlayerCondition interpreter was added. Native XP tests prove Human Paladin and Orc Mage cap transitions earn Double Agent7380 through account aggregation and remain earned after redb reopen, without opposite-faction or admin/login event credit. The [inventory](../../achievement-criteria-producer-inventory.md) owns the updated structural census.

## Per-request emote targets (verified 2026-10-13)

`EmoteIntent.target_entity` carries the client-selected server entity with each request. `messaging::emote_target_entry` resolves its NPC template on the sender's map copy before queuing the criteria event; it never reads current CombatTarget or interprets a creature entry as an entity ID. The compiler admits All4/SingleTrue TargetCreatureId4 only for DoEmote54, alongside already-supported player/difficulty leaves. Love225 maps to text only, preserving the pose. Primary source: pinned TC a352b1fa ChatHandler.cpp703–753 and CriteriaHandler.cpp1782–1785; pinned69933 EmotesText225 EmoteID0. Native RED/GREEN exercises two different requests in one channel batch, eligibility and invalid-target controls, no idle replay and reopened counters3615/3739. Other target modifiers/producers remain disabled; [inventory](../../achievement-criteria-producer-inventory.md) owns structural counts, not creature spawn completeness.

## Rated map wins and participation (verified 2026-10-13)

The same native rated settlement publishes map-specific WinArena32 for winners and ParticipateInArena33 for every online in-world participant, after persistence and the result's once-only applied marker. Events carry the completed source map; PlayerInArenaWithTeamSize24 (2/3) and PlayerInRankedArenaMatch60 now read the current accepted native arena (next section), not a guessed rating asset or queue. Native-channel2v2/3v3 RED/GREEN covers counters8613/8617/8605/8609, losers receiving participation only, wrong map/team size, no preparation/skirmish/idle replay and redb reopen. Primary source: TC a352b1fa Arena.cpp241–250, CriteriaHandler.cpp1652–1655/1879–1885/2059–2065.5v5 remains unsupported; type39 still has a source/data slot mapping gap. The [inventory](../../achievement-criteria-producer-inventory.md) owns the structural census; no new rendered arena proof is claimed.

## Full-roster limit (verified 2026-10-13)

ModifierTree29 LessThanTappers is a group roster condition (TC a352b1fa CriteriaHandler1902–1905), not a count of nearby attackers. `ReferencePlayer.group_size` snapshots `GroupRegistry.group_members` including offline members;0 means known solo and None means unknown authority. All4/SingleTrue limits remain conjunctive with legacy/current difficulty and other supported leaves. Native authored Malygos spawning/combat/death in copy616/3 proves1869 and criterion7174 for solo/roster8, rejects roster9 and25-player7175, and survives idle ticks/redb reopen. Fixture's actor UnitLevel initially lacked character Level; corrected before the retained valid RED/GREEN pair. This is criterion lifecycle proof, not full Malygos combat/render parity. The [inventory](../../achievement-criteria-producer-inventory.md) owns counts and remaining source/data gaps.

## Battleground kill rewards (verified 2026-10-13)

`spell_cast::log_event` forwards a real lethal event's controlling player/victim to `pvp::kill_criteria`. The active copy and accepted roster identify the killer's team; authenticated in-world members still in that copy are credited once. Killer ignores distance. Other recipients use corpse position when dead and source GetDistance: exact3D distance minus each combat reach, within74 yards. Player reach is1.5×native scale (TC Player1574–1577); Corpse inherits reach0 (Object.h302). Primary producer: Battleground1354–1383, including Arena's base call; distance: Player26397–26409/Object430–433.

Native effect/death tests prove untimed **orphan** counter26004 with actual health loss, reach boundaries77/75.5, remote corpse/ghost, other-copy/far/unaccepted recipients, inactive/world/self rejection, replay and reopen. The initial fixture lacked Time/SpellStore; the first chosen11491 also has start/fail conditions and correctly stayed disabled. Corrected untimed RED/GREEN retained;11491 and aura-gated20124 remain uncredited. No current achievement grant or rendered battle is claimed. World honor eligibility/kill rewards, lifetime honor113 and scoreboard honorable-kill availability remain distinct gaps; [inventory](../../achievement-criteria-producer-inventory.md) owns structural counts.

## General arena player predicates (verified 2026-10-13)

`pvp::player_arena_conditions` follows the player's current battleground ID into the native ledger, verifies current map copy/accepted character, then reads native team size and RatedMatch presence. ReferencePlayer snapshots these for all gated event producers; the compiler no longer restricts24/60 to32/33. Result events no longer carry redundant team-size fields. Native-source rated2/3 and skirmish2/3 tests prove real killing blows5541/5542, deaths4962/4963 and kill rewards4959/4960, with size isolation, idle/reopen and unrated rejection. Primary leaves: TC CriteriaHandler1879–1885/2059–2065.5v5 is absent; current Shuffle is intentionally unrated, and pinned239/240 lack progress implementation, so neither lifecycle presence nor an enum name authorizes credit. [Inventory](../../achievement-criteria-producer-inventory.md) owns census and gaps.

## Map instance type (verified 2026-10-13)

Modifier122 uses authenticated Map.InstanceType rows to load a set of map IDs per requested raw type; runtime checks the reference player's actual map. World0 is valid. Missing maps/type memberships and out-of-range assets remain disabled. Source: pinned TC CriteriaHandler2394–2397. Native real-channel rated/skirmish arena2/3 plus BG effect→queued death-system tests prove arena50456/50457 versus BG50458/50460, cross-type rejection, idle/reopened counters. Fixture now runs `process_player_death_transitions` with its real required resources; test-only counter visibility changed, production death logic did not. Raid-death18 remains a separate missing producer. [Inventory](../../achievement-criteria-producer-inventory.md) owns the +4-statistics census; no rendered-runtime claim.

## Sources

- [Producer inventory](../../achievement-criteria-producer-inventory.md) — raw/current-root census and continuation proof.
- `scripts/import_achievements.py`, `scripts/db2_casc.py` — local extraction/export/import.
- `crates/server/src/achievement_account.rs`, `achievement_catalog.rs`, `achievement_window.rs`, `achievement.rs`, `dungeon_progress.rs` — catalog, bounded presentation and gameplay delivery.
- [Client hookup](../../dungeon-achievements-client.md) — protocol fields, bounds and Retail sources.
- `crates/server/src/maps/store.rs`, `instances.rs`, `locks.rs` — encounter ordering, masks and lock persistence.
- `crates/server/src/scripted_ai/tests/dungeonach.rs` — real Stockade death/message proof.

## See Also

- [[boss-scripts]] — encounter state transitions.
- [[persistence]] — character and instance storage.
- [[zone-architecture]] — map copies and lock lifecycle.
