# Guild news

[Contract](../../specs/guild-news.md). Implementation currently contains only member-scoped query scaffolding; actual producers/storage remain pending. Test-first acceptance earns installed achievement14782 through the real achievement catalog/update/account-persistence boundary and expects one source-qualified durable row, with duplicate/reopen controls.

## Sources

Mainline `GuildNews.lua` / `GuildUtil.lua` defines rendering, filters and CanEditMOTD sticky controls. TrinityCore a352b1fa `DBCEnums.h:90–91` uses0x1000 for guild-news eligibility and0x2000 for an initial header/sticky. `SharedDefines.h:6627` caps news at250, distinct from lifecycle100. Actual item producers use quality>Epic or Epic itemlevel>=353; vendor purchase checks base level, loot/craft effective level (`Player.cpp:23904,27194`, `SpellEffects.cpp:1452`, `Guild.h:260`). Unsupported guild-achievement/encounter/calendar producers do not generate rows.

## See Also

- [Guild lifecycle](guild-lifecycle.md) — separate real invite/join/rank/remove/quit ledger.
- [Banks](banks.md) — separate item/money history and authority.
