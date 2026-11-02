-- Lead-only delta. Apply after the map 1877 population. Review before COMMIT.
BEGIN IMMEDIATE;
CREATE TEMP TABLE temple_heal_guard (ok INTEGER NOT NULL CHECK(ok = 1));
INSERT INTO temple_heal_guard SELECT COUNT(*) = 1 FROM raid_encounter_creature
 WHERE map_id=1877 AND creature_entry=133392 AND encounter_id=2127 AND bit=3;
CREATE TABLE IF NOT EXISTS encounter_heal_objective (
 map_id INTEGER NOT NULL, creature_entry INTEGER NOT NULL,
 start_health_pct REAL NOT NULL CHECK(start_health_pct > 0 AND start_health_pct < 100),
 PRIMARY KEY(map_id, creature_entry)
);
INSERT INTO temple_heal_guard SELECT COUNT(*) = 0 FROM encounter_heal_objective
 WHERE map_id=1877 AND creature_entry=133392;
-- INFERRED_WCL: post-heal resource samples from M4cdBNJkTjavYgQV/11,
-- ax1vNFnfPdXrzRQ2/16, ZgBCXQvcnhaAYT1r/5 imply 25% before the first heal.
INSERT INTO encounter_heal_objective VALUES (1877, 133392, 25.0);
-- LittleWigs Avatar.lua identifies Retail gossip 48126, "We will restore you!".
-- TDB menu 22561 option 0 has the same text (broadcast 153194).
INSERT INTO temple_heal_guard SELECT count(*)=1 FROM content_creature_template
 WHERE entry=133392 AND gossip_menu_id=0 AND npcflag=1;
INSERT INTO temple_heal_guard SELECT count(*)=1 FROM tdb_creature_template_gossip
 WHERE CreatureID=133392 AND MenuID=22561;
INSERT INTO temple_heal_guard SELECT count(*)=1 FROM tdb_gossip_menu_option
 WHERE MenuID=22561 AND OptionID=0 AND OptionBroadcastTextID=153194;
INSERT INTO temple_heal_guard SELECT count(*)=0 FROM content_gossip_menu WHERE MenuID=22561;
INSERT INTO temple_heal_guard SELECT count(*)=0 FROM content_gossip_menu_option WHERE MenuID=22561;
INSERT INTO temple_heal_guard SELECT count(*)=0 FROM content_npc_text WHERE ID=34714;
INSERT INTO temple_heal_guard SELECT count(*)=1 FROM tdb_npc_text WHERE ID=34714 AND BroadcastTextID0=153204;
INSERT INTO content_npc_text (ID, Probability0, BroadcastTextID0, VerifiedBuild)
 SELECT ID, Probability0, BroadcastTextID0, VerifiedBuild FROM tdb_npc_text WHERE ID=34714;
INSERT INTO temple_heal_guard SELECT changes()=1;
INSERT INTO content_gossip_menu (MenuID,TextID) VALUES (22561,34714);
INSERT INTO content_gossip_menu_option
 (MenuID,OptionID,OptionIcon,OptionText,OptionBroadcastTextID,OptionType,OptionNpcFlag,ActionMenuID,ActionPoiID,BoxCoded,BoxMoney,BoxText,BoxBroadcastTextID,VerifiedBuild)
 SELECT MenuID,OptionID,0,OptionText,OptionBroadcastTextID,1,1,0,0,0,0,NULL,0,VerifiedBuild
 FROM tdb_gossip_menu_option WHERE MenuID=22561 AND OptionID=0;
INSERT INTO temple_heal_guard SELECT changes()=1;
UPDATE content_creature_template SET gossip_menu_id=22561 WHERE entry=133392 AND gossip_menu_id=0;
INSERT INTO temple_heal_guard SELECT changes()=1;
DROP TABLE temple_heal_guard;
-- Intentionally no COMMIT.
