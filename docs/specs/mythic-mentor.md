# Mythic Mentor party keystones

- Authenticated in-world clients receive a complete `PartyKeystoneSnapshot` on the reliable ordered InstanceChannel when their party key data changes (one-second refresh).
- Recipient first, then server party roster order. Solo receives self; raids receive no rows. Only current party members are disclosed, including members without a key.
- Keystone summaries come from persisted bag items joined to their persisted keystone records, qualified by the imported [Mythic+ catalog](mythic-plus.md). Stale records without a matching bag item are not keys.
- Summaries include character identity, name, class, leader status, challenge ID, level, authored dungeon name and time limit. Peer item GUIDs, unrelated parties and addon messages are excluded.
- Roster/leadership changes, bag changes, depletion/upgrades, logout and character switching replace earlier state; recipients cannot request arbitrary characters' keys.

Reference: installed `BossHelper/Modules/KeystoneTracker.lua:33-98`, `UI/KeystoneWidget.lua:501-600`. The native UI contract lives in [game-engine](../../../game-engine/docs/specs/mythic-mentor.md).
