-- Import authored TrinityCore Retail spell_target_position excerpts using mysql_dump.py.
-- No destination seed rows: SpellIDs and positions must come from the supplied source.
CREATE TABLE IF NOT EXISTS spell_target_position (
    ID INTEGER NOT NULL,
    EffectIndex INTEGER NOT NULL,
    MapID INTEGER NOT NULL,
    PositionX REAL NOT NULL,
    PositionY REAL NOT NULL,
    PositionZ REAL NOT NULL,
    Orientation REAL NOT NULL,
    VerifiedBuild INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (ID, EffectIndex)
);
