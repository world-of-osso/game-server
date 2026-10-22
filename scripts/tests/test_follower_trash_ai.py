"""Behavioral source extraction and stable-key projection controls."""

import json
import sys
import sqlite3
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import follower_trash_ai as trash

LW = Path("/home/osso/Repos/LittleWigs")


class TrashSources(unittest.TestCase):
    def test_retail_display_comes_from_tdb_not_legacy_model_table(self):
        with sqlite3.connect(
            "file:/home/osso/Projects/world-of-osso/game-engine/data/server-world/world.db?mode=ro",
            uri=True,
        ) as world:
            report = trash.census(
                world,
                dict(
                    maps=[dict(MapID=2515, MapName_lang="The Azure Vault")],
                    encounters=[],
                ),
                LW,
            )
        lasher = next(actor for actor in report["actors"] if actor["entry"] == 196102)
        self.assertEqual(lasher["models"], [76261])

    def test_player_friendly_mereldar_citizen_is_not_a_trash_recipient_or_donor(self):
        with sqlite3.connect(
            "file:/home/osso/Projects/world-of-osso/game-engine/data/server-world/world.db?mode=ro",
            uri=True,
        ) as world:
            report = trash.census(
                world,
                dict(
                    maps=[dict(MapID=2662, MapName_lang="The Dawnbreaker")],
                    encounters=[],
                ),
                LW,
            )
        citizen = next(actor for actor in report["actors"] if actor["entry"] == 225601)
        self.assertEqual(citizen.get("excluded_reason"), "player_friendly_faction")

    def test_lua_cast_events_belong_to_registered_named_actor(self):
        kits = trash.read_lua_kits(LW)
        self.assertEqual(kits[196102]["spells"], [387564])
        self.assertEqual(kits[187159]["spells"], [370225, 397726])
        self.assertEqual(kits[187155]["spells"], [377488])
        self.assertNotIn(194602, kits, "transport books have no combat kit")
        self.assertNotIn(
            375649, kits[191164]["spells"], "periodic damage is not a cast"
        )

    def test_wowhead_abilities_only_not_loot_or_related_spells(self):
        page = """new Listview({template: 'item', id: 'drops', data: [{"id":7}]});
new Listview({template: 'spell', id: 'abilities', data: [{"id":387564,"name":"Mystic Vapors",modes:{"mode":[1,2,23]}},{"id":317898,"name":"Blinding Sleet",modes:{"mode":[1]}}]});"""
        self.assertEqual(trash.read_wowhead_abilities(page), [317898, 387564])
        self.assertEqual(
            trash.read_wowhead_abilities(json.dumps(page)), [317898, 387564]
        )
        self.assertEqual(trash.read_wowhead_abilities("<html>No abilities</html>"), [])

    def test_similar_kit_is_named_policy_with_donor_not_new_spell(self):
        actors = [
            dict(
                entry=187160,
                name="Crystal Fury",
                family=0,
                models=[106135],
                maps=[2515],
                spells=[],
                classification="none",
            ),
            dict(
                entry=196116,
                name="Crystal Fury",
                family=0,
                models=[106135],
                maps=[2515],
                spells=[370764],
                classification="Lua-only",
            ),
        ]
        trash.infer_similar_kits(actors)
        self.assertEqual(actors[0]["spells"], [370764])
        self.assertEqual(actors[0]["donor_entry"], 196116)
        self.assertEqual(actors[0]["policy"], "FOLLOWER_TRASH_SIMILAR_CREATURE_KIT")
        self.assertEqual(actors[0]["classification"], "inferred")

    def test_same_name_stem_records_the_actual_sourced_donor(self):
        actors = [
            dict(
                entry=46249,
                name="Riverpaw Slayer",
                family=0,
                models=[],
                maps=[34],
                spells=[],
                classification="none",
            ),
            dict(
                entry=46252,
                name="Riverpaw Shaman",
                family=0,
                models=[],
                maps=[34],
                spells=[12167, 23381],
                classification="Wowhead-only",
            ),
        ]
        trash.infer_similar_kits(actors)
        self.assertEqual(actors[0]["spells"], [12167, 23381])
        self.assertEqual(actors[0]["donor_entry"], 46252)

    def test_uncertain_encounter_candidate_is_not_a_donor(self):
        actors = [
            dict(
                entry=187160,
                name="Crystal Fury",
                family=0,
                models=[106135],
                maps=[2515],
                spells=[],
                classification="none",
            ),
            dict(
                entry=196116,
                name="Crystal Fury",
                family=0,
                models=[106135],
                maps=[2515],
                spells=[370764],
                classification="Lua-only",
                excluded_reason="uncertain_encounter_candidate",
            ),
        ]
        trash.infer_similar_kits(actors)
        self.assertEqual(actors[0]["spells"], [])

    def test_tc_tender_combat_kit_retains_source_clocks(self):
        kit = trash.read_tc_combat_kit(
            trash.TC / "DragonIsles/AzureVault/boss_leymor.cpp", "npc_arcane_tender"
        )
        self.assertEqual(
            kit,
            dict(
                spells=[375596, 375652],
                spell_schedules={
                    "375596": dict(initial_ms=22000, repeat_ms=22000),
                    "375652": dict(initial_ms=12000, repeat_ms=12000),
                },
            ),
        )

    def test_source_bear_model_family_supplies_warbringer_with_named_donor(self):
        actors = [
            dict(
                entry=245146,
                name="Grizzled Warbringer",
                family=0,
                models=[131630],
                model_files=[123058],
                maps=[2825],
                spells=[],
                classification="none",
            ),
            dict(
                entry=241808,
                name="Territorial Matriarch",
                family=0,
                models=[14316, 822],
                model_files=[1266661],
                maps=[2825],
                spells=[1237888, 1241219],
                classification="Wowhead-only",
            ),
        ]
        trash.infer_similar_kits(actors)
        self.assertEqual(actors[0]["spells"], [1237888, 1241219])
        self.assertEqual(actors[0]["donor_entry"], 241808)
        self.assertEqual(actors[0]["policy"], "FOLLOWER_TRASH_SIMILAR_CREATURE_KIT")
        self.assertIn(
            "bear_model_family: bearmount.m2/bear2.m2", actors[0]["inference_basis"]
        )

    def test_no_cross_dungeon_or_zero_family_match(self):
        actors = [
            dict(
                entry=187160,
                name="Crystal Fury",
                family=0,
                models=[],
                maps=[2515],
                spells=[],
                classification="none",
            ),
            dict(
                entry=196116,
                name="Unrelated",
                family=0,
                models=[],
                maps=[2521],
                spells=[370764],
                classification="Lua-only",
            ),
        ]
        trash.infer_similar_kits(actors)
        self.assertEqual(actors[0]["classification"], "none")


class TrashProjection(unittest.TestCase):
    def setUp(self):
        self.world = sqlite3.connect(":memory:")
        self.world.row_factory = sqlite3.Row
        source = sqlite3.connect(
            "file:/home/osso/Projects/world-of-osso/game-engine/data/server-world/world.db?mode=ro",
            uri=True,
        )
        for table in [
            "content_creature_template",
            "tdb_smart_scripts",
            "spell_misc",
            "spell_effect",
            "spell_cooldowns",
            "spell_cast_times",
            "spell_range",
        ]:
            schema = source.execute(
                "SELECT sql FROM sqlite_master WHERE name=?", (table,)
            ).fetchone()[0]
            self.world.execute(schema)
            if table in ["content_creature_template", "tdb_smart_scripts"]:
                key = "entry" if table == "content_creature_template" else "entryorguid"
                rows = source.execute(
                    f"SELECT * FROM {table} WHERE {key} IN (476,191164,196102)"
                ).fetchall()
            elif table in ["spell_cast_times", "spell_range"]:
                rows = source.execute(f"SELECT * FROM {table}").fetchall()
            else:
                rows = source.execute(
                    f"SELECT * FROM {table} WHERE SpellID IN (387564,317898,375596,375652)"
                ).fetchall()
            if rows:
                self.world.executemany(
                    f"INSERT INTO {table} VALUES ({','.join('?' for _ in rows[0])})",
                    rows,
                )
        schema = source.execute(
            "SELECT sql FROM sqlite_master WHERE name='tdb_smart_scripts'"
        ).fetchone()[0]
        self.world.execute(
            schema.replace("tdb_smart_scripts", "content_smart_scripts", 1)
        )
        source.close()
        self.actor = dict(
            entry=196102,
            name="Conjured Lasher",
            spells=[387564],
            classification="Lua-only",
            sources=["LittleWigs2777a9d:Dragonflight/TheAzureVault/Trash.lua"],
            maps=[2515],
        )

    def tearDown(self):
        self.world.close()

    def test_projection_is_content_only_replayable_and_preserves_authored_ai(self):
        before = self.world.execute("SELECT * FROM tdb_smart_scripts").fetchall()
        sql, receipt = trash.prepare_projection(self.world, [self.actor])
        self.world.executescript(sql)
        self.world.executescript(sql)
        self.assertEqual(
            self.world.execute("SELECT COUNT(*) FROM content_smart_scripts").fetchone()[
                0
            ],
            1,
        )
        self.assertEqual(
            self.world.execute(
                "SELECT action_param1 FROM content_smart_scripts WHERE entryorguid=196102"
            ).fetchone()[0],
            387564,
        )
        self.assertEqual(
            self.world.execute(
                "SELECT AIName FROM content_creature_template WHERE entry=196102"
            ).fetchone()[0],
            "SmartAI",
        )
        self.assertEqual(
            self.world.execute("SELECT * FROM tdb_smart_scripts").fetchall(), before
        )
        self.assertEqual(receipt["casts"][0]["spell"], 387564)

    def test_unsourced_cadence_is_one_named_default_native_metadata_is_recorded(self):
        sql, receipt = trash.prepare_projection(self.world, [self.actor])
        self.world.executescript(sql)
        self.assertEqual(
            self.world.execute("SELECT COUNT(*) FROM content_smart_scripts").fetchone()[
                0
            ],
            1,
        )
        self.assertEqual(
            self.world.execute(
                "SELECT event_param1,event_param3,target_type FROM content_smart_scripts"
            ).fetchone()[:],
            (10000, 10000, 1),
        )
        cast = receipt["casts"][0]
        self.assertEqual(cast["cadence_policy"], "FOLLOWER_TRASH_UNSOURCED_CADENCE_MS")
        native_time = self.world.execute(
            "SELECT Base FROM spell_cast_times WHERE ID=19"
        ).fetchone()[0]
        self.assertEqual(cast["cast_time_ms"], native_time)
        self.assertEqual(cast["range_index"], 6)

    def test_refresh_removes_only_superseded_project_kit_spells(self):
        old = dict(self.actor, spells=[317898], classification="Wowhead-only")
        sql, _ = trash.prepare_projection(self.world, [old])
        self.world.executescript(sql)
        sql, _ = trash.prepare_projection(self.world, [self.actor])
        self.world.executescript(sql)
        self.assertEqual(
            [
                row[0]
                for row in self.world.execute(
                    "SELECT action_param1 FROM content_smart_scripts WHERE entryorguid=196102"
                )
            ],
            [387564],
        )

    def test_literal_tc_combat_lanes_preserve_script_name_and_source_clocks(self):
        kit = trash.read_tc_combat_kit(
            trash.TC / "DragonIsles/AzureVault/boss_leymor.cpp", "npc_arcane_tender"
        )
        actor = dict(
            self.actor,
            entry=191164,
            name="Arcane Tender",
            source_script="npc_arcane_tender",
            classification="sourced-kit",
            **kit,
        )
        sql, receipt = trash.prepare_projection(self.world, [actor])
        self.world.executescript(sql)
        rows = [
            tuple(row)
            for row in self.world.execute(
                "SELECT action_param1,event_param1,event_param3 FROM content_smart_scripts ORDER BY action_param1"
            )
        ]
        self.assertEqual(rows, [(375596, 22000, 22000), (375652, 12000, 12000)])
        self.assertEqual(
            self.world.execute(
                "SELECT ScriptName FROM content_creature_template WHERE entry=191164"
            ).fetchone()[0],
            "npc_arcane_tender",
        )
        self.assertTrue(
            all(cast["cadence_policy"] is None for cast in receipt["casts"])
        )

    def test_existing_tc_script_is_not_silently_replaced(self):
        actor = dict(self.actor, entry=191164, name="Arcane Tender")
        sql, receipt = trash.prepare_projection(self.world, [actor])
        self.world.executescript(sql)
        self.assertEqual(
            self.world.execute(
                "SELECT ScriptName FROM content_creature_template WHERE entry=191164"
            ).fetchone()[0],
            "npc_arcane_tender",
        )
        self.assertEqual(receipt["casts"], [])
        self.assertEqual(
            receipt["skipped"], [dict(entry=191164, reason="existing_authored_ai")]
        )


if __name__ == "__main__":
    unittest.main()
