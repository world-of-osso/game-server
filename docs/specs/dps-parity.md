# Native item-instance DPS parity

## Requirements

- [ ] Item instances retain bonus lists, an optional explicit item level, permanent enchant and gem item IDs by GUID, with backward-compatible redb storage. Crafted stat selections must survive where the shipped profile specifies them.
- [ ] Equipment stats resolve native70448 ItemBonus, ItemSparse, RandPropPoints and Midnight scaling/configuration tables, including stat allocation, sockets and enchant/gem effects. Unsupported effects are named gaps, never invented stats.
- [ ] A headless300s follower dummy harness consumes the exact shipped MID1 profile gear through the production gear/stat path and supported talents/consumables, on one target and a clearly specified three-target DungeonSlice-like setup.
- [ ] Reports compare primary/crit/haste/mastery/versatility/AP/SP first, then action counts/average hit/damage share and buff uptimes. Flag every delta>5% and classify stats/gear, coefficients, missing proc/aura or rotation/decision.
- [ ] Compare SimC70391 and native70448 effect rows before attributing a coefficient mismatch to a server bug. Rotation/decision changes remain lead-owned.
- [ ] Reference all11 requested specs66/73/105/257/262/63/253/1467/269/266/103. Missing shipped105/257 healer profiles are explicit HPS-reference gaps, not synthetic BiS profiles.

## Inputs and proof

Official SimulationCraft76f8118cf3f7f41dad20708e8328e1f4ab2deebc, CLI-only local build. Native read-only exports `game-engine/data/db2/12.1.5.70448/`; no re-export, package install or canonical data mutation. Canonical producer application belongs to the lead.

`scripts/item_instance_data.py` imports eight native tables into a supplied existing SQLite copy, preserving signed values and recording source hashes/build. Two behavior tests prove replacement/idempotence and rollback on duplicate identities. This proves the producer only, not item-instance runtime support.

`scripts/dps_parity.py` extracts exact shipped MID1 item identities, modifiers, talents, consumables and action text into manifests; parsing tests cover identity preservation and missing-item rejection. It does not simulate or claim gear equality.

Evidence: `game-engine/data/diagnostics/dps-parity-2026-10-21/`. SimC's embedded data is70391; the server uses70448 throughout.
