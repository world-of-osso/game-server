# Guild news

[Contract](../../specs/guild-news.md). Additive guild-keyed JSON stores monotonic IDs and latest250 insertion-ordered entries, separate from lifecycle/bank logs. Achievement contribution/account transactions compare prior durable earned IDs and append installed0x1000 definitions in the same transaction;0x2000 marks an initial sticky. Item news follows successful real loot-slot, CREATE_ITEM24 and vendor commits only. Loot/craft use installed effective item level; purchases use base level. Generic acquisitions, moves, refusals and duplicate/re-taken slots do not fabricate provenance.

Authenticated GuildChannel query/sticky requests resolve persisted membership and Edit MOTD permission. Subscriptions receive changes on real inventory transactions and newly earned achievements, not per-frame history polling. Sticky requests broadcast to current member subscribers; each delivery revalidates current membership. Separate GoldUpdateQueue news notifications cover achievements without inventing bag changes.

## Proof ledger —2026-10-15

At serverd6f5c859/protocol9bb88b6, targeted `guild_news_` GREEN6/6 (`logs/guild-completion-news-handler-green2.log`): installed Level50 actual earn/duplicate/timestamp/reopen;250 retention/tied order/abort/sticky rights/cross-guild/reopen; actual Lillehoff content vendor50335 buy; actual controlled Thunderfury loot receipt (not a Kobold authored-drop claim), refusal/retake/ordinary controls; actual Sulfuras recipe21160 consumes17193+17204 and creates17182 with failed-recast controls; authenticated HostClient query/refusal/committed live update. Loot/craft REDs observed actual item ownership but zero news. Handler first fixture incorrectly expected unauthenticated transport buffers to survive; corrected to resend after authentication. Private real-UDP/native acceptance and the final source gate are recorded below.

## Sources

Mainline `GuildNews.lua` / `GuildUtil.lua` defines rendering, filters and CanEditMOTD sticky controls. TrinityCore a352b1fa `DBCEnums.h:90–91` uses0x1000 for guild-news eligibility and0x2000 for an initial header/sticky. `SharedDefines.h:6627` caps news at250, distinct from lifecycle100. Actual item producers use quality>Epic or Epic itemlevel>=353; vendor purchase checks base level, loot/craft effective level (`Player.cpp:23904,27194`, `SpellEffects.cpp:1452`, `Guild.h:260`). Unsupported guild-achievement/encounter/calendar producers do not generate rows.

## See Also

- [Guild lifecycle](guild-lifecycle.md) — separate real invite/join/rank/remove/quit ledger.
- [Banks](banks.md) — separate item/money history and authority.

## Private native/UDP milestone

At engine3304bcace/server205f1d2b/protocol9bb88b6, real Lillehoff purchases/sales and both-skin native physical overflow pass. Original private history is retained through an explicit fixture continuation after its monitor budget expired; the continuation queries durable IDs and performs only68 missing acquisitions to target261. Actual retained rows are12–261 (250), with one native sticky. Initiate receives Permissions for sticky; actual quit then receives empty NotInGuild news. Native query/filter/context/sticky ACKs also pass. No news or catalog rows are seeded. At final merged source engine5e1661761/server5001d9d1/protocol9bb88b6, refreshed both-skin native controls and fresh server/client exact250-row/timestamp/sticky readback pass with unchanged SHA256 receipts. Post-merge backend6/6, model4/4, wire1/1 and all three cargo checks/format checks pass; binaries build without warnings. Lead published the two locally extracted source textures, and16 full-HD captures were inspected against installed XML. Evidence: `/home/osso/.worktrees/logs/guild-news-2026-10-15-run1/`; independent lead integration/landing remains separate.
