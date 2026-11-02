-- Lead-only candidate. Apply after the Temple population; no COMMIT.
-- INFERRED_WCL_HOSTILE: player melee hit all three entries in M4cdBNJkTjavYgQV/11 pull12.
-- TDB 69497 has faction 0 for these encounter adds; faction 16 is inferred.
BEGIN IMMEDIATE;
CREATE TEMP TABLE temple_add_guard(ok INTEGER NOT NULL CHECK(ok=1));
INSERT INTO temple_add_guard SELECT count(*)=0 FROM content_creature_template WHERE entry IN (268344,268427,268491);
INSERT INTO temple_add_guard SELECT count(*)=0 FROM content_creature_template_model WHERE CreatureID IN (268344,268427,268491);
INSERT INTO temple_add_guard SELECT count(*)=3 FROM tdb_creature_template WHERE entry IN (268344,268427,268491) AND VerifiedBuild=69497 AND faction=0;
INSERT INTO temple_add_guard SELECT count(*)=3 FROM tdb_creature_template_difficulty WHERE Entry IN (268344,268427,268491) AND DifficultyID=0;
INSERT INTO temple_add_guard SELECT count(*)=3 FROM tdb_creature_template_model WHERE CreatureID IN (268344,268427,268491);
INSERT INTO content_creature_template
 (entry,KillCredit1,KillCredit2,name,subname,IconName,minlevel,maxlevel,exp,faction,npcflag,speed_walk,speed_run,scale,rank,dmgschool,DamageModifier,BaseAttackTime,RangeAttackTime,BaseVariance,RangeVariance,unit_class,unit_flags,unit_flags2,family,type,type_flags,VehicleId,mingold,maxgold,AIName,MovementType,HealthModifier,ManaModifier,ArmorModifier,ExperienceModifier,RacialLeader,movementId,RegenHealth,flags_extra,ScriptName,VerifiedBuild,ContentTuningID,HealthScalingExpansion,LevelScalingDeltaMin,LevelScalingDeltaMax,StaticFlags1,StaticFlags2,StaticFlags3,StaticFlags4,StaticFlags5,StaticFlags6,StaticFlags7,StaticFlags8)
 SELECT t.entry,t.KillCredit1,t.KillCredit2,t.name,t.subname,t.IconName,1,1,0,16,t.npcflag,t.speed_walk,t.speed_run,t.scale,t.Classification,t.dmgschool,d.DamageModifier,t.BaseAttackTime,t.RangeAttackTime,t.BaseVariance,t.RangeVariance,t.unit_class,t.unit_flags,t.unit_flags2,t.family,t.type,d.TypeFlags,t.VehicleId,d.GoldMin,d.GoldMax,t.AIName,t.MovementType,d.HealthModifier,d.ManaModifier,d.ArmorModifier,t.ExperienceModifier,t.RacialLeader,t.movementId,t.RegenHealth,t.flags_extra,t.ScriptName,t.VerifiedBuild,d.ContentTuningID,d.HealthScalingExpansion,d.LevelScalingDeltaMin,d.LevelScalingDeltaMax,d.StaticFlags1,d.StaticFlags2,d.StaticFlags3,d.StaticFlags4,d.StaticFlags5,d.StaticFlags6,d.StaticFlags7,d.StaticFlags8
 FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry AND d.DifficultyID=0
 WHERE t.entry IN (268344,268427,268491);
INSERT INTO temple_add_guard SELECT changes()=3;
INSERT INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID IN (268344,268427,268491);
INSERT INTO temple_add_guard SELECT changes()=3;
DROP TABLE temple_add_guard;
-- Intentionally no COMMIT.
