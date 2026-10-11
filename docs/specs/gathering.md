# Gathering professions

Retail Mainline governs Mining, Herbalism and Skinning. Forever changes art only.

- Herb/ore nodes use authored gameobject spawns, pools, model displays, lock alternatives, loot templates and timers. No fabricated nodes, loot IDs or skill formulas.
- Known gathering skill is required; cast, interruption, range, map-copy and depleted-resource checks are authoritative. Loot and profession persistence reuse existing systems.
- Find Herbs / Find Minerals track eligible nearby resources on the native minimap.
- Authored skill progression uses local skill-line data and pinned TrinityCore rules; unavailable mappings are explicit gaps.
- Depletion and respawn follow authored node metadata and spawn pools.
- Acceptance requires behavioral tests and inspected Modern + Forever PNG proof of tracking, herb/ore gathering, skinning a kill and skill progression.

## Server/client boundary

`GatherableResource.parent_skill` identifies a node or fully looted corpse for native tracking and interaction. A corpse becomes eligible only after its normal loot roll has completed and all personal/shared slots and group rolls are resolved; a completed zero-drop roll is distinct from loot not yet generated. `UseGameObject` requests gathering for those replicated resources; the server selects an authored gathering spell for a learned skill, then uses the normal cast/interrupt pipeline. Resource loot uses the existing personal loot window and reference/group roller. Authored per-player claims, depletion delay and pool respawn prevent repeat collection of one node. Profession progression reuses changed-state persistence/snapshots.

The named Classic baseline has [integrated both-skin native proof](../wiki/systems/gathering.md#proof-boundary). Missing PlayerCondition mappings and nonpositive respawn timers remain explicit gaps; they are not replaced with invented behavior.

## Source import prerequisite — not runtime acceptance

`gathering_metadata.py` imports local Lock FDID1343608/layoutDA418152 alternatives and TDB1210 skinning loot using existing decoders/remappers. It also repairs Retail type50 loot namespace mapping. The lead applies it; agents never modify world.db.

```text
python3 scripts/gathering_metadata.py --lock-db2 /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/gathering-nodes-2026-10-11/1343608.db2 --tdb-dump data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql --db data/world.db
```

Reapply after a full world content rebuild until the gathering import is integrated into that pipeline. Missing item/currency mappings are reported, not replaced. Neither metadata import nor source inspection proves a playable gathering loop.
