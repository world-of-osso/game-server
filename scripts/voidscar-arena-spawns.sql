-- Lead-only candidate, to apply after voidscar-arena-templates.sql. No COMMIT.
-- INFERRED_WCL_POSITION: Aegyra 267545 uses first damage coordinates in
-- h82XJKQYxVztdnqv/1 pull1 (71.648s relative to pull); Raj'kess 267546 uses
-- NYqJhxcCaVZQpf8t/6 (58.5s). First combat coordinates can be displaced from
-- their spawn, especially Aegyra (TFfB8XRw6HPVLjN2/11 first hit differs ~9 yd).
-- INFERRED_WMO_FLOOR: the local floor resolver reports multiple candidates at
-- each XY; -110.47446/-110.486176 are the WMO candidates consistent with the
-- nearest local branch markers, approximately 26/24 yd away. This is not a
-- verified actor floor. Facing 0 is unknown. Retire these rows when a Retail
-- spawn or reliable unengaged-position/floor observation is available.
-- Difficulty 23 fills the Mythic copy; active Mythic+ uses difficulty 8.
BEGIN IMMEDIATE;
CREATE TEMP TABLE voidscar_spawn_guard(ok INTEGER NOT NULL CHECK(ok=1));
INSERT INTO voidscar_spawn_guard SELECT count(*)=2 FROM content_creature_template WHERE entry IN (267545,267546);
INSERT INTO voidscar_spawn_guard SELECT count(*)=0 FROM content_creature WHERE map=2923 AND id1 IN (267545,267546);
INSERT INTO voidscar_spawn_guard SELECT count(*)=0 FROM content_creature WHERE guid IN (8100247681,8100247682);
INSERT INTO content_creature
 (guid,id1,id2,id3,map,zoneId,areaId,spawnMask,phaseMask,equipment_id,position_x,position_y,position_z,orientation,spawntimesecs,wander_distance,currentwaypoint,curhealth,curmana,MovementType,npcflag,unit_flags,dynamicflags,ScriptName,VerifiedBuild,CreateObject,Comment,modelid,spawnDifficulties)
 VALUES
 (8100247681,267545,0,0,2923,0,0,1,1,0,4618.68,-378.03,-110.47446,0,7200,0,0,1,0,0,0,0,0,'',NULL,0,'INFERRED_WCL_OPENING',0,'8,23'),
 (8100247682,267546,0,0,2923,0,0,1,1,0,4618.82,-526.75,-110.486176,0,7200,0,0,1,0,0,0,0,0,'',NULL,0,'INFERRED_WCL_OPENING',0,'8,23');
INSERT INTO voidscar_spawn_guard SELECT changes()=2;
DROP TABLE voidscar_spawn_guard;
-- Intentionally no COMMIT.
