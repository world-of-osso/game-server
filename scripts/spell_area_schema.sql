-- Incremental TDB1210.26091 import: replaces only authored spell-area requirements.
DROP TABLE IF EXISTS tdb_spell_area;
CREATE TABLE tdb_spell_area (
  spell INTEGER NOT NULL DEFAULT 0,
  area INTEGER NOT NULL DEFAULT 0,
  quest_start INTEGER NOT NULL DEFAULT 0,
  quest_end INTEGER NOT NULL DEFAULT 0,
  aura_spell INTEGER NOT NULL DEFAULT 0,
  racemask INTEGER NOT NULL DEFAULT 0,
  gender INTEGER NOT NULL DEFAULT 2,
  flags INTEGER NOT NULL DEFAULT 3,
  quest_start_status INTEGER NOT NULL DEFAULT 64,
  quest_end_status INTEGER NOT NULL DEFAULT 11,
  PRIMARY KEY (spell, area, quest_start, aura_spell, racemask, gender)
);
