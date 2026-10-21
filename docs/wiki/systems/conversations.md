# Conversations

Source ingress is implemented; server runtime/payload and then protocol/Godot TalkingHead presentation are the active ordered slices. [Contract](../../specs/conversations.md).

## Local source ingress

`scripts/conversation_data.py` reads pinned TDB1210 conversation templates248, actors424 and line templates702, verifies whole-table fingerprints, and imports native70448 ConversationLine73,256/BroadcastTextDuration1 from the existing local CSV receipt. The five destination tables are `tdb_conversation_template`, `tdb_conversation_actors`, `tdb_conversation_line_template`, `conversation_line` and `broadcast_text_duration`. Only their two DB2 build keys are updated. Other tables remain untouched.

The CLI requires an existing owned database, rejects canonical publication, and can emit repeatable scoped SQL. SQLite encodes publication literals; both existing metadata and unrelated rows survive independent replay. Source receipt authentication flags/unreadable-record counts remain unchanged, including ConversationLine's false table-authentication flag and96 unreadable records.

```
python3 scripts/conversation_data.py --db /owned/world.db --sql /owned/conversations.sql
```

Actual source/parser RED→GREEN and native-row write RED→GREEN precede publication/refusal RED2→GREEN. Final four tests pass6.83s at a034edf54 with no warnings after fixing explicit SQLite connection closure. Fresh canonical mode=ro SQLite backup plus the actual CLI and independent pre-import backup SQL replay match all five tables. Artifacts: engine `data/diagnostics/follower-dungeons-2026-10-21/conversation-producer-a034edf54/`; logs `conversation-{source,import,publication}-{red,green}.log` and `conversation-data-final4.log`. Canonical publication remains lead-owned.

## Orator consumer inputs

Source conversation24642 starts line67085/broadcast259911;24643 starts67091/259917;24644 starts67092/259918. All use native actor51642/display116692 and UI camera1751. The first actor's creature is220769, not a fabricated alias for the216619 creator; the other two use216619. Resolved TDB broadcast rows provide authored text and sound kits250774–250776.

Actual static ingress266 and268 are rotated boxes with extents10/50 and total height5;267 is a sphere of radius50. Source `AreaTrigger.cpp793–821` contains box feet directly, without inflating the box by unit reach; sphere selection uses native unit distance. These shapes must not be substituted with a generic radius-only trigger.

## Pending runtime boundaries

TC `Conversation.cpp` creates private-owner actor/line objects, computes per-locale line starts from BroadcastTextDuration plus AdditionalDuration, and adds10s after the last line. Current imported duration data has no row for the three Orator broadcasts; preserve the native missing-duration branch rather than inventing voice timings. Generic Actor/line creation, owner isolation, lifetime and real Orator ingress are next. Protocol and authored70448 Retail TalkingHead presentation for both skins/native1920×1080 PNGs follow afterward, not dropped.

## Sources

- TC a352b1fa `Conversation.cpp`, `ConversationDataStore.cpp`, `DB2Stores.cpp` and `boss_orator_krix_vizk.cpp`; pinned local snapshots in engine diagnostics `sources/`.
- Local `TDB_full_world_1210.26091_2026_09_09.sql`, active12.1.5.70448 CSVs/receipt and resolved `content_broadcast_text`.

## See Also

- [World-static ingress](world-area-triggers.md)
- [Orator source boundaries](boss-scripts.md#tww-city-of-threads-orator--2026-10-21)
