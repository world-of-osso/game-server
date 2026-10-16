# Slow native test fixtures

## Scope and measurements (2026-10-16)

The lead's master gate reported eleven native gameplay tests above60s. Three isolated CPU profiles on server379c2c6b/protocoldc30250 retained every assertion:

| Test | Test-body duration | Runner duration including perf |
|---|---:|---:|
| All sixteen authored arena maps |469.15s|475.06s|
| Absent-player-aura marks/kills/reopen |55.11s|60.26s|
| Orb of Deception race/sex matrix |144.40s|148.25s|

Evidence: `game-engine/data/diagnostics/server-slow-tests-2026-10-16/` contains exact argv/revision/admission JSON, logs and49Hz user CPU perf recordings. Host contention varies; these are test-harness diagnostics, not release budget measurements. DWARF call-chain unwinding was incomplete; percentages are self samples, not complete loader attribution.

## Shared cause

Each arena builds a fresh app but reparses the same creature/achievement inputs; Solo Shuffle also reloads full class/spell data. The aura test loads the full achievement catalog five times. Orb reloads the complete spell catalog for every one of its twenty mapped race/sex cases and the unmapped refusal. Profiles show SQL VM/decoding and catalog construction (`criteria_conditions`, spell `find_mut`) rather than sleeping. Boss fixtures repeatedly load all creature templates before selecting their encounter subset. Pet fixtures repeatedly snapshot/import the same class summon source and reload its creatures/spells on relog.

## Change and authority boundaries

Test-only process caches parse canonical creature and achievement inputs once. The existing native spell fixture supplies complete spell/class copies; new clone implementations exist only under `cfg(test)`. Each app still owns mutable resources, entities and redb files. Achievement progress, dates, inventory, characters and protocol replies remain independent.

Class summon preparation imports one process-owned snapshot before sharing it read-only. Its creature product is cloned before subset selection. Prepared encounter copies retain their own loaders: the canonical cache must never replace a copy's newly imported authority. Orb's producer changes creature/item/map tables, not spells; its spell input remains the unchanged canonical catalog.

The affected harnesses already use manual50ms Bevy time. No wall-clock wait or gameplay duration was shortened; all matches, casts, kills, race/sex combinations, cooldowns and durable-reopen assertions remain. No development profile or production loader changed.

## Verification

Initial implementationb2df8d8e; fixture-isolation and affected gameplay verification pending. Full-suite timing remains lead-owned.

## Related

- [Server tick lag](server-tick-lag.md) — distinct runtime/FIFO work; no release acceptance follows from faster tests.
