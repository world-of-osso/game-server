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
DROP TABLE temple_heal_guard;
-- Intentionally no COMMIT.
