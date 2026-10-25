-- Literal Retail LittleWigs schedule fixtures, revision 2777a9d51418934b5fdd5b92d937b1af6070bcad.
-- Classic/DireMaul/Hydrospawn.lua lines41,42,50,51,56,57.
-- Classic/DireMaul/ZevrimThornhoof.lua lines35,36,37,45,46,51,61,62.
CREATE TABLE bossmod_authority(encounter_id INTEGER PRIMARY KEY,addon TEXT);
INSERT INTO bossmod_authority VALUES(344,'LittleWigs'),(343,'LittleWigs'),(346,'LittleWigs');
CREATE TABLE bossmod_rule(id INTEGER PRIMARY KEY,addon TEXT,encounter_id INTEGER,trigger TEXT,trigger_spell_id INTEGER,health_pct REAL,required_phase REAL,action TEXT,spell_id INTEGER,delay_ms INTEGER,phase REAL,severity INTEGER);
INSERT INTO bossmod_rule VALUES
(1,'LittleWigs',344,'engage',NULL,NULL,NULL,'timer',22419,6900,NULL,NULL),
(2,'LittleWigs',344,'engage',NULL,NULL,NULL,'timer',22421,5700,NULL,NULL),
(3,'LittleWigs',344,'cast_success',22419,NULL,NULL,'warning',22419,NULL,NULL,2),
(4,'LittleWigs',344,'cast_success',22419,NULL,NULL,'timer',22419,6100,NULL,NULL),
(5,'LittleWigs',344,'cast_start',22421,NULL,NULL,'warning',22421,NULL,NULL,1),
(6,'LittleWigs',344,'cast_start',22421,NULL,NULL,'timer',22421,8500,NULL,NULL),
(7,'LittleWigs',343,'engage',NULL,NULL,NULL,'timer',22478,3700,NULL,NULL),
(8,'LittleWigs',343,'engage',NULL,NULL,NULL,'timer',22651,7100,NULL,NULL),
(9,'LittleWigs',343,'engage',NULL,NULL,NULL,'timer',17228,8300,NULL,NULL),
(10,'LittleWigs',343,'cast_start',22478,NULL,NULL,'warning',22478,NULL,NULL,1),
(11,'LittleWigs',343,'cast_start',22478,NULL,NULL,'timer',22478,8500,NULL,NULL),
(12,'LittleWigs',343,'cast_success',22651,NULL,NULL,'timer',22651,17000,NULL,NULL),
(13,'LittleWigs',343,'cast_start',17228,NULL,NULL,'warning',17228,NULL,NULL,2),
(14,'LittleWigs',343,'cast_start',17228,NULL,NULL,'timer',17228,17000,NULL,NULL),
-- AlzzinTheWildshaper.lua:66,81,82 (same source revision).
(15,'LittleWigs',346,'engage',NULL,NULL,NULL,'timer',22662,900,NULL,NULL),
(16,'LittleWigs',346,'cast_start',22662,NULL,NULL,'warning',22662,NULL,NULL,2),
(17,'LittleWigs',346,'cast_start',22662,NULL,NULL,'timer',22662,7300,NULL,NULL);
-- Rendered from enUS_common.lua CL.casting and pinned SpellName12.1.0.69933.
ALTER TABLE bossmod_rule ADD COLUMN warning_text TEXT;
UPDATE bossmod_rule SET warning_text = CASE spell_id
 WHEN 22419 THEN 'Riptide'
 WHEN 22421 THEN 'Massive Geyser'
 WHEN 22478 THEN 'Casting Intense Pain'
 WHEN 17228 THEN 'Casting Shadow Bolt Volley'
 WHEN 22662 THEN 'Wither'
 END WHERE action='warning';
