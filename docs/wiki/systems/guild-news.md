# Guild news

[Contract](../../specs/guild-news.md). The additive JSON history stores a guild's monotonic next ID and latest250 entries; current persisted membership selects queries. Achievement account/contribution transactions compare previous durable character-earned IDs and actual new account toasts, then append only installed0x1000-flagged definitions in that same transaction. Initial sticky comes from0x2000; title/description use existing bounded installed metadata. Test-first acceptance earns installed achievement14782 through the real catalog/update/account-persistence boundary: RED observed zero news despite the real Level50 toast. Producer GREEN and retention/sticky/UDP acceptance remain pending. Item producers and authenticated network handlers are not implemented yet.

## Sources

Mainline `GuildNews.lua` / `GuildUtil.lua` defines rendering, filters and CanEditMOTD sticky controls. TrinityCore a352b1fa `DBCEnums.h:90–91` uses0x1000 for guild-news eligibility and0x2000 for an initial header/sticky. `SharedDefines.h:6627` caps news at250, distinct from lifecycle100. Actual item producers use quality>Epic or Epic itemlevel>=353; vendor purchase checks base level, loot/craft effective level (`Player.cpp:23904,27194`, `SpellEffects.cpp:1452`, `Guild.h:260`). Unsupported guild-achievement/encounter/calendar producers do not generate rows.

## See Also

- [Guild lifecycle](guild-lifecycle.md) — separate real invite/join/rank/remove/quit ledger.
- [Banks](banks.md) — separate item/money history and authority.
