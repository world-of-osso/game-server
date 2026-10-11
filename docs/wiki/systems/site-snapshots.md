# Static armory and ranking snapshots

Offline exporter `game-server-site-export` produces the public JSON contract in [site snapshots](../../specs/site-snapshots.md). Static publishing opens no game-host listener. All characters in the realm redb are public by default minus character-ID opt-outs ([user decision and privacy contract](../../specs/site-snapshots.md#privacy-and-catalog-preparation)), never account-name inference.

## Content

### Export and publication boundary

Build with `python3 scripts/build-host.py --bin game-server-site-export` through the normal native adapter. Agents wrap local commands with `agent-run`. Export a consistent closed/offline redb backup and matching read-only SQLite catalog into a **new** directory:

```text
game-server-site-export --redb PRIVATE_BACKUP.redb --world-db WORLD.db --opt-outs opted-out-ids.json --textures LOCAL_TEXTURE_CACHE --output NEW_SNAPSHOT_DIR --realm world-of-osso --realm-name "World of Osso"
```

The opt-out file is a JSON array of decimal character ID strings (`[]` publishes all). `as_of` is read once from the actual export-start clock; only tests inject a deterministic clock. A single redb read transaction and SQLite read transaction cover each snapshot. All reads, decodes and icon conversions complete before destination creation. An existing destination is rejected so privacy removals cannot leave stale character pages. File-write failure may leave an incomplete new directory; the publisher must publish only a successful complete export. Publishing/atomic directory switching is lead-owned, outside exporter scope. The installed agent-server user refresh job performs it only when explicitly run/enabled by the lead; [refresh operations](../../specs/site-snapshots.md#agent-server-refresh-job) covers installation, dry-run, opt-outs, timer and atomic exchange.

Read-only opening refuses crash-dirty backups. Explicit `--repair-copy SOURCE NEW_PRIVATE_COPY` copies into an exclusively created destination and repairs only that new copy; source bytes never change. See [copy recovery safety](../../specs/site-snapshots.md#privacy-and-catalog-preparation). The regular exporter opens no auth/account tables.

### Future ranking history

Azure Vault's existing four authored creature/encounter/saved-ordinal bindings live in the shared `scripted_ai/encounter_compat.csv` registry after the master merge; retired bespoke dungeon modules are not restored. Both challenge and raid history still receive the same DONE lifecycle boundary.

`mythic_completed_runs_v1` is JSON keyed by run ID and written in the same transaction as key settlement and existing challenge history. Capture players actually in that copy at completion, including class and active spec. Only completed challenges become ranking records; abandonment and legacy anonymous runs are excluded. Timer/duration come from the authoritative runtime; no rating formula is added. Active-season selection is currently absent, so season remains null.

`raid_first_kills_v1` records first character/guild kills at `SetBossState(DONE)` and the existing unambiguous authored raid-creature death path. At-kill participant records preserve character and guild membership IDs/names; qualifying guild first kills use the approved 80% rule. Repeated kills, resets and later guild transfers cannot overwrite first timestamps. Lifetimes are explicitly post-deploy observations; existing reset locks remain a separate display, not a ranking backfill. [Guild rule, timestamp tie-break, and privacy requirements](../../specs/site-snapshots.md#lifetime-raid-first-kill-history).

### Asset/data limitations

Local BLP/PNG item and achievement icons become web-ready PNG files. Missing cache assets yield null URLs. No CDN access or cache mutation. The copy-only `prepare_site_catalog.py` projects cached product-isolated ChrRaces into the reference copy; FactionTemplate supplies faction labels. Neck/finger/trinket/ranged storage, complete effective equipped item level, character renders and pre-deploy historical credit remain explicit source gaps. Paperdoll/bag slots have explicit equipped/empty/unavailable state; zero-game PvP brackets are unrated and unsourced legacy conquest caps are discarded. Physical equipment, transmog GUID overrides, named talent loadouts, character achievement contributions and saved PvP state are read from their existing authoritative tables; private inventories and account aggregates are not emitted.

## Sources

- [JSON contract](../../specs/site-snapshots.md)
- [Exporter](../../../crates/server/src/site_snapshot.rs), [M+ reader](../../../crates/server/src/site_snapshot_mythic.rs), [raid reader](../../../crates/server/src/site_snapshot_raids.rs)
- [M+ completion history](../../../crates/server/src/mythic_plus_history.rs), [raid persistence](../../../crates/server/src/raid_history_store.rs)
- [Raid boundary](../../../crates/server/src/raid_history.rs)

## See Also

- [[mythic-plus]] — authored challenge lifecycle and unsourced season/rating boundaries.
- [[raid-entry]] — source-qualified death credit and reset/instance locks.
