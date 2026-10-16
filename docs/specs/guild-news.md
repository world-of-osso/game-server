# Guild news

Real member-scoped guild news, separate from lifecycle/bank history. Source: Mainline `GuildNews.lua`, `GuildUtil.lua`; TrinityCore a352b1fa `AchievementMgr.cpp:475`, `Player.cpp:23904,27194`, `SpellEffects.cpp:1452`, `Guild.h:260`, `SharedDefines.h:6627`. [Producer boundaries](../wiki/systems/guild-news.md).

## What it must do

- [ ] Append only actual new player achievements flagged0x1000 and actual qualifying item loot/craft/purchases. Capture source names/IDs/metadata; no administrative/roster backfill or repeated-achievement rows.
- [ ] Commit achievement news with its persisted earned state. Keep250 insertion-ordered durable rows and immutable IDs; abort/reopen preserves mutation/news consistency.
- [ ] Resolve current membership from the authenticated actor. Nonmembers receive no news. Sticky writes require Edit MOTD, use real stable entry IDs and survive restart.
- [ ] Preserve producer-specific item qualification: quality above Epic, or Epic effective item level>=353 (vendor purchase uses base item level).
- [ ] Real UDP/native Modern+Forever query/filter/sticky/refusal/privacy and process-restart acceptance, with inspected full-HD captures.

## How it works

- [Guild news producers](../wiki/systems/guild-news.md).

## Implementation inventory

| File | Role |
|---|---|
| shared-protocol `src/protocol/guild_news_messages.rs` | Separate member-only news query/sticky request and typed reply |
| `crates/server/src/guild_news.rs` | Member query, additive250-row storage and transactional achievement append |
| `crates/server/src/achievement_account.rs`, `achievement_window.rs`, `persistence.rs` | Atomic actual-earned producer, installed flags/metadata and initialization |
| `crates/server/src/achievement_tests.rs` | Actual installed Level50 earn/duplicate/reopen acceptance |

## Tests asserting this spec

- `achievement_tests.rs::guild_news_real_flagged_player_achievement_commits_once_and_survives_restart` — RED observed0 news despite actual installed Level50 toast; GREEN for durable sourced row/duplicate/reopen controls; actual vendor/loot/craft/authenticated controls documented in [proof ledger](../wiki/systems/guild-news.md#proof-ledger-2026-10-15).

## Known gaps (current cycle)

- [x] Actual achievement/vendor/loot/craft producers, persistence/retention/sticky authority and authenticated handler (targeted6/6).
- [ ] Both-skin real native/persistence/pixel acceptance and independent gates.

## Out of scope

Guild-achievement, encounter and calendar news lack corresponding guild producers. Perks/rewards and invitation tabards are separate slices; never substitute player achievements or ordinary NPC deaths for guild events.
