-- TrinityCore TDB1210 guild reward tables (GuildMgr::LoadGuildRewards, GuildMgr.cpp:495),
-- loaded by mysql_dump.py --prefix tdb_ from data/reference/tdb1210/<table>.sql.
-- MinGuildRep is a ReputationRank (0 Hated .. 7 Exalted); RaceMask is a RaceMask raw value
-- (0 = every race).

DROP TABLE IF EXISTS tdb_guild_rewards;
CREATE TABLE tdb_guild_rewards (
  ItemID INTEGER NOT NULL DEFAULT 0,
  MinGuildRep INTEGER DEFAULT 0,
  RaceMask INTEGER DEFAULT 0,
  Cost INTEGER DEFAULT 0,
  PRIMARY KEY (ItemID)
);

DROP TABLE IF EXISTS tdb_guild_rewards_req_achievements;
CREATE TABLE tdb_guild_rewards_req_achievements (
  ItemID INTEGER NOT NULL DEFAULT 0,
  AchievementRequired INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (ItemID, AchievementRequired)
);
