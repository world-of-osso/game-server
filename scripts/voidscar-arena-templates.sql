-- Lead-only candidate; apply in a transaction after inspecting the guards. No COMMIT.
-- VERIFIED_TDB: template/model/difficulty source: TrinityCore 69497, entries
-- 267545 Aegyra, 267546 Raj'kess and 267550 Champion's Spear.
-- VERIFIED_DB2: spell 1298908 effect 28 summons entry 267550.
-- INFERRED_WCL_HOSTILE: the three actors die in completed Mythic+ logs
-- h82XJKQYxVztdnqv/1 and TFfB8XRw6HPVLjN2/11; TDB faction 0 is not hostile.
-- Faction 16 is a local hostile substitute, not a verified retail faction.
-- Retire that substitute when the retail faction is identified.
BEGIN IMMEDIATE;
CREATE TEMP TABLE voidscar_template_guard(ok INTEGER NOT NULL CHECK(ok=1));
INSERT INTO voidscar_template_guard SELECT count(*)=0 FROM content_creature_template WHERE entry IN (267545,267546,267550);
INSERT INTO voidscar_template_guard SELECT count(*)=0 FROM content_creature_template_model WHERE CreatureID IN (267545,267546,267550);
INSERT INTO voidscar_template_guard SELECT count(*)=3 FROM tdb_creature_template WHERE entry IN (267545,267546,267550) AND VerifiedBuild=69497 AND faction=0;
INSERT INTO voidscar_template_guard SELECT count(*)=3 FROM tdb_creature_template_difficulty WHERE Entry IN (267545,267546,267550) AND DifficultyID=0;
INSERT INTO voidscar_template_guard SELECT count(*)=3 FROM tdb_creature_template_model WHERE CreatureID IN (267545,267546,267550);
INSERT INTO content_creature_template
 (entry,KillCredit1,KillCredit2,name,subname,IconName,minlevel,maxlevel,exp,faction,npcflag,speed_walk,speed_run,scale,rank,dmgschool,DamageModifier,BaseAttackTime,RangeAttackTime,BaseVariance,RangeVariance,unit_class,unit_flags,unit_flags2,family,type,type_flags,VehicleId,mingold,maxgold,AIName,MovementType,HealthModifier,ManaModifier,ArmorModifier,ExperienceModifier,RacialLeader,movementId,RegenHealth,flags_extra,ScriptName,VerifiedBuild,ContentTuningID,HealthScalingExpansion,LevelScalingDeltaMin,LevelScalingDeltaMax,StaticFlags1,StaticFlags2,StaticFlags3,StaticFlags4,StaticFlags5,StaticFlags6,StaticFlags7,StaticFlags8)
 SELECT t.entry,t.KillCredit1,t.KillCredit2,t.name,t.subname,t.IconName,1,1,0,16,t.npcflag,t.speed_walk,t.speed_run,t.scale,t.Classification,t.dmgschool,d.DamageModifier,t.BaseAttackTime,t.RangeAttackTime,t.BaseVariance,t.RangeVariance,t.unit_class,t.unit_flags,t.unit_flags2,t.family,t.type,d.TypeFlags,t.VehicleId,d.GoldMin,d.GoldMax,t.AIName,t.MovementType,d.HealthModifier,d.ManaModifier,d.ArmorModifier,t.ExperienceModifier,t.RacialLeader,t.movementId,t.RegenHealth,t.flags_extra,t.ScriptName,t.VerifiedBuild,d.ContentTuningID,d.HealthScalingExpansion,d.LevelScalingDeltaMin,d.LevelScalingDeltaMax,d.StaticFlags1,d.StaticFlags2,d.StaticFlags3,d.StaticFlags4,d.StaticFlags5,d.StaticFlags6,d.StaticFlags7,d.StaticFlags8
 FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry AND d.DifficultyID=0
 WHERE t.entry IN (267545,267546,267550);
INSERT INTO voidscar_template_guard SELECT changes()=3;
INSERT INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID IN (267545,267546,267550);
INSERT INTO voidscar_template_guard SELECT changes()=3;
DROP TABLE voidscar_template_guard;
-- Intentionally no COMMIT.
