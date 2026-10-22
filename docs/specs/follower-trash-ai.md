# Follower dungeon trash AI

## Contract

Every spawned trash entry in the current27 follower-dungeon maps is inventoried from the union of legacy, content and TDB spawn slots. Boss/encounter actors and noncombat/service candidates remain visible with explicit exclusions. Counts derive from the current canonical DB, not the historical627/650 estimate.

Ability authority: Retail12.1.5.70448 Spell* rows, TDB1210 creature_template_spell/SmartAI/addons and TrinityCore a352b1fa scripts, then LittleWigs2777a9d trash cast events, then cached Wowhead NPC Abilities. Passive addon auras do not establish a cast kit. Warning-only aura/damage events never become casts. Wowhead requests use only the host-wide locked/cache fetcher, at most one request per10seconds.

If a trash creature has no sourced kit, `FOLLOWER_TRASH_SIMILAR_CREATURE_KIT` records a single-hop sourced donor from the same dungeon or another-difficulty template. Exact name, display, nonzero family, shared name stem and shared role are descending similarity evidence. Donor entry, source and basis remain inspectable. No matching donor remains `none`; no spell identity is invented.

## Execution

Reuse the existing native SmartAI and spell-casting pipeline. Authentic TDB rows remain unchanged in `tdb_smart_scripts`; project-generated rows belong to `content_smart_scripts`, loaded by the same native interpreter. Existing authored SmartAI/ported TC behavior takes precedence. Project rows carry `FOLLOWER_TRASH_*` provenance, including donor policy where applicable.

Native Spell* records own cast times, ranges, cooldown/category admission and spell effects. Where source supplies no combat cadence and DB2 has no recovery time, the sole project default is `FOLLOWER_TRASH_UNSOURCED_CADENCE_MS = 10000`. This is project behavior, not a Retail timing claim. Source-authored event clocks remain source-authored, not reconstructed from warning bars.

## Publication and proof

Implementer never writes canonical world.db. Deliver stable-key SQL and a receipt tested by replay on a fresh canonical SQLite backup; repeated application preserves authored source rows and unrelated content. Lead publishes.

Stockade and Azure are the first native proof batch. A sourced trash creature must fail its native cast test before implementation, then emit a real native cast completion after projection. Controls cover creature_ai, scripted_ai, dungeon_simulator and followers under the shared gate environment, at most6test threads, plus check/fmt. Census/source coverage is not native mechanic certification; missing effects, unsupported selectors and unresolved TC-only actors remain named gaps.
