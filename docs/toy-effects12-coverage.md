# Toy-effects12 non-summon coverage

Verified: 2026-10-11. Implementation awaiting GREEN proof; no rendered-client claim.

Baseline prepared census130/1166. Selected additions:50471 The Heartbreaker,129149 Death's Door Charm,205045 B.B.F. Fist,211931 Abyss Caller Horn. Behavioral RED5/5: actual UseToy rejects aura4 for the first three and effect3 for the horn; prepared census rejects50471. Final census and exact retained gaps will be recorded after testing.

Ranking excludes unsupported-item/condition rows and any reachable effect28: aura4=35, effect3=31, effect77=16, effect50=11, aura56=8. Counts overlap. Implement the highest-yield source-complete cosmetic subset of the first two groups; no blanket dummy success, no new summon control types.

Sources: local Retail12.1.0.69933 DB2, read-only world.db and actual TDB1210 archive. Durable receipts: server `data/diagnostics/toy-effects12-2026-10-11/` (`visual-source.json`, `model-paths.json`, `archive-script-bindings.json`, `red.log`, `red-census.json`). [Contract](specs/toys.md), [system](wiki/systems/toys.md#toy-effects12-non-summon-cosmetics-2026-10-11). No database preparation, live server or engine changes.
