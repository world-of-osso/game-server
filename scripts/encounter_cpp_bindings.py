#!/usr/bin/env python3
"""Prepare source-authenticated primary-actor binding SQL; never modify world.db.

Pinned TC instance/header rows identify Kystia, Golden Serpent, Koranthal,
Adarogg, Slagmaw, Gordoth, Wailing Caverns, Silverlaine, Electrocutioner and Crowd Pummeler primaries.
Companions never acquire completion credit. The lead applies generated SQL to
reviewed copies; structural binding is not mechanic or native certification.
"""

import argparse
import csv
import hashlib
import io
import json
import sqlite3
from contextlib import closing
from pathlib import Path

from raid_encounters import read_journal


SOURCES = {
    "QuelThalas/MurderRow/boss_kystia_manaheart.cpp": "48277cefac3dce24529564b2685b6a75a15a10d1063a875ba06dcf14e382bb2b",
    "QuelThalas/MurderRow/instance_murder_row.cpp": "b1022ae88166015dd3b85ef01a920bbbcb24be9e48334ca365c5133c71d0b222",
    "QuelThalas/MurderRow/murder_row.h": "a1129bb03bf3c83795942e342125d07ee69b7472761e35b56e4a395d0d4050e6",
}
PRIMARY = dict(
    map_id=2813,
    entry=234648,
    encounter_id=3101,
    bit=0,
    script="boss_kystia_manaheart",
    journal_id=2679,
    actor_id=5847,
    display_id=124578,
)
GOLDEN_SOURCES = {
    "Zandalar/KingsRest/boss_golden_serpent.cpp": "f72bf8ec35669d5d2c861b5e6b0e1f713590ca37a869293c35a9262e4dd8cc51",
    "Zandalar/KingsRest/instance_kings_rest.cpp": "b8f1aca56c4d4577acb3340be4759eda1d04eb9b1ad4f028ef8d077e7b636a16",
    "Zandalar/KingsRest/kings_rest.h": "671801740347d9bd6b0ae426ff72f252d86589ca07758acd3cd709e1a159944a",
}
GOLDEN_PRIMARY = dict(
    map_id=1762,
    entry=135322,
    encounter_id=2139,
    bit=0,
    script="boss_the_golden_serpent",
    journal_id=2165,
    actor_id=4904,
    display_id=84202,
)
KORANTHAL_SOURCES = {
    "Kalimdor/RagefireChasm/boss_dark_shaman_koranthal.cpp": "6671ef80cf7c774380051fce58d6408c5ca0dd558ae57e3e2eebf0ea9a8edf97",
    "Kalimdor/RagefireChasm/instance_ragefire_chasm.cpp": "5271197a0157eb46f06bd3f2fb3ebb916f484622fb09af8ca12a87534aa45a72",
    "Kalimdor/RagefireChasm/ragefire_chasm.h": "29da8a54bc8a15786835fcee8b5f14fd1bffac2803f480b056eaf6ed5d304f08",
}
KORANTHAL_PRIMARY = dict(
    map_id=389,
    entry=61412,
    encounter_id=1444,
    bit=1,
    script="boss_dark_shaman_koranthal",
    journal_id=695,
    actor_id=1321,
    display_id=42101,
)
RAGEFIRE_INSTANCE_SOURCES = {
    "Kalimdor/RagefireChasm/instance_ragefire_chasm.cpp": "5271197a0157eb46f06bd3f2fb3ebb916f484622fb09af8ca12a87534aa45a72",
    "Kalimdor/RagefireChasm/ragefire_chasm.h": "29da8a54bc8a15786835fcee8b5f14fd1bffac2803f480b056eaf6ed5d304f08",
}
ADAROGG_SOURCES = {
    "Kalimdor/RagefireChasm/boss_adarogg.cpp": "1962c785a8f405b4dcf779ab1b2ce3caef7ea26c8a84e8ef685f7b8a20f38237",
    **RAGEFIRE_INSTANCE_SOURCES,
}
ADAROGG_PRIMARY = dict(
    map_id=389,
    entry=61408,
    encounter_id=1443,
    bit=0,
    script="boss_adarogg",
    journal_id=694,
    actor_id=1343,
    display_id=38759,
)
SLAGMAW_SOURCES = {
    "Kalimdor/RagefireChasm/boss_slagmaw.cpp": "cd19880df4b1c3a8121a447a2aad3be126d8a44b664e7cf32dd747d43ec21a73",
    **RAGEFIRE_INSTANCE_SOURCES,
}
# C++ NPC_SLAGMAW/BOSS_SLAGMAW identifies the primary independently of appearance.
# display_id is Journal preview38391; the live TDB actor retains model42247.
SLAGMAW_PRIMARY = dict(
    map_id=389,
    entry=61463,
    encounter_id=1445,
    bit=2,
    script="boss_slagmaw",
    journal_id=696,
    actor_id=1344,
    display_id=38391,
)
GORDOTH_SOURCES = {
    "Kalimdor/RagefireChasm/boss_lava_guard_gordoth.cpp": "305aca6990850c74372f4461ab7659f0e35d243c015136d3e2bcca6a0641d395",
    **RAGEFIRE_INSTANCE_SOURCES,
}
GORDOTH_PRIMARY = dict(
    map_id=389,
    entry=61528,
    encounter_id=1446,
    bit=3,
    script="boss_lava_guard_gordoth",
    journal_id=697,
    actor_id=1345,
    display_id=42100,
)
WAILING_SOURCES = {
    "Kalimdor/WailingCaverns/wailing_caverns.h": "ea7fbb0733c7dac751470e0c36fb7ab40e8f5a92a3b7790ba22959b26b955f0d",
    "Kalimdor/WailingCaverns/instance_wailing_caverns.cpp": "01d26ba675612d4a4b06a5c7306da5cd1be29ff2388b80d4685ef61ed667c5fc",
}
COBRAHN_PRIMARY = dict(
    map_id=43,
    entry=3669,
    encounter_id=586,
    bit=1,
    script="",
    ai="SmartAI",
    journal_id=475,
    actor_id=1046,
    display_id=4213,
)
SHADOWFANG_SOURCES = {
    "EasternKingdoms/ShadowfangKeep/instance_shadowfang_keep.cpp": "996e91b82bba4dd2f5003fe03f4db6802bd384ddb1b3772df48f359777d8a418",
    "EasternKingdoms/ShadowfangKeep/shadowfang_keep.h": "8f7db01cd0ed1e89d180d6cc3b4f6c4507b22a3fea66b7f9d24ebd78a9331a39",
}
SILVERLAINE_PRIMARY = dict(
    map_id=33,
    entry=3887,
    encounter_id=1070,
    bit=1,
    script="",
    ai="SmartAI",
    journal_id=97,
    actor_id=367,
    display_id=37288,
)
GNOMEREGAN_SOURCES = {
    "EasternKingdoms/Gnomeregan/gnomeregan.h": "d78e089544edb621fab73b8e91aa7e32d53defebeac260376b049995e49b7e68",
    "EasternKingdoms/Gnomeregan/instance_gnomeregan.cpp": "17e6fdf82b17956d2d9a229da5556a3dfc1f87c87f18b4bff548a4d41e79c8bb",
}
# TC DATA_ELECTROCUTIONER is ordinal2; installed Normal encounter380 uses bit3.
ELECTROCUTIONER_PRIMARY = dict(
    map_id=90,
    entry=6235,
    encounter_id=380,
    bit=3,
    script="",
    ai="SmartAI",
    journal_id=421,
    actor_id=991,
    display_id=36558,
)
# TC DATA_CROWD_PUMMELER is ordinal3; installed Normal encounter381 uses bit4.
PUMMELER_PRIMARY = dict(
    map_id=90,
    entry=6229,
    encounter_id=381,
    bit=4,
    script="",
    ai="SmartAI",
    journal_id=418,
    actor_id=989,
    display_id=36560,
)
THERMAPLUGG_PRIMARY = dict(
    map_id=90,
    entry=7800,
    encounter_id=382,
    bit=5,
    script="",
    ai="SmartAI",
    journal_id=422,
    actor_id=992,
    display_id=36563,
)
MARAUDON_SOURCES = {
    "Kalimdor/Maraudon/maraudon.h": "c4b5a0d7278887ab361f2336b01bfd0f9695d66dad5c7ce48e1f6b53bc452ea0",
    "Kalimdor/Maraudon/instance_maraudon.cpp": "d1b011494852577a78feae6894acdb8b5b33a5619785c4d9bf1009ef9f7f9524",
}
RAZORLASH_PRIMARY = dict(
    map_id=349,
    entry=12258,
    encounter_id=423,
    bit=1,
    script="",
    ai="SmartAI",
    journal_id=424,
    actor_id=994,
    display_id=12389,
)
ROTGRIP_PRIMARY = dict(
    map_id=349,
    entry=13596,
    encounter_id=428,
    bit=6,
    script="",
    ai="SmartAI",
    journal_id=430,
    actor_id=1000,
    display_id=13589,
)
NOXXION_PRIMARY = dict(
    map_id=349,
    entry=13282,
    encounter_id=422,
    bit=0,
    script="boss_noxxion",
    journal_id=423,
    actor_id=993,
    display_id=11172,
)
NOXXION_SOURCES = {
    **MARAUDON_SOURCES,
    "Kalimdor/Maraudon/boss_noxxion.cpp": "1ccc69132bb6a4dc95bb9b0ce4e9d81586447e59ca494d575e95b36f1ee8f211",
}
# TC ordinal2 is not the installed Normal427 completion bit5.
GIZLOCK_PRIMARY = dict(
    map_id=349,
    entry=13601,
    encounter_id=427,
    bit=5,
    script="",
    ai="SmartAI",
    journal_id=425,
    actor_id=995,
    display_id=7125,
)
# TC ordinal3 is not the installed Normal424 completion bit2.
VYLETONGUE_PRIMARY = dict(
    map_id=349,
    entry=12236,
    encounter_id=424,
    bit=2,
    script="",
    ai="SmartAI",
    journal_id=427,
    actor_id=997,
    display_id=12334,
)
CELEBRAS_PRIMARY = dict(
    map_id=349,
    entry=12225,
    encounter_id=425,
    bit=3,
    script="boss_celebras_the_cursed",
    journal_id=428,
    actor_id=998,
    display_id=12350,
)
CELEBRAS_SOURCES = {
    **MARAUDON_SOURCES,
    "Kalimdor/Maraudon/boss_celebras_the_cursed.cpp": "eb52d44aa838b6012155ceb9214eb3fab7d6b9d459dfdbd70370a65a2328c077",
}
LANDSLIDE_PRIMARY = dict(
    map_id=349, entry=12203, encounter_id=426, bit=4,
    script="boss_landslide", journal_id=429, actor_id=999, display_id=12293,
)
LANDSLIDE_SOURCES = {
    **MARAUDON_SOURCES,
    "Kalimdor/Maraudon/boss_landslide.cpp": "37e958ab7e2e1cdea078bb39227d209ee42f7bc781c3906ca4d4cc3b7201074b",
}
THERADRAS_PRIMARY = dict(
    map_id=349, entry=12201, encounter_id=429, bit=7,
    script="boss_princess_theradras", journal_id=431, actor_id=1001, display_id=12292,
)
THERADRAS_SOURCES = {
    **MARAUDON_SOURCES,
    "Kalimdor/Maraudon/boss_princess_theradras.cpp": "fb6db01bb7d463145e32c0218504624c7668b1070dd9bf38d07532fd99ad3244",
}
GARGOLMAR_PRIMARY = dict(
    map_id=543, entry=17306, encounter_id=1893, bit=2,
    script="boss_watchkeeper_gargolmar", journal_id=527, actor_id=1104, display_id=18236,
)
GARGOLMAR_SOURCES = {
    "Outland/HellfireCitadel/HellfireRamparts/hellfire_ramparts.h": "9896de99ab8f2049a445af5168b152387bdd019315d898a44e8bf83d72967bce",
    "Outland/HellfireCitadel/HellfireRamparts/instance_hellfire_ramparts.cpp": "7f907d01ba05818b1dd4f2e945353433569f56a79773722f5d05a622009af49e",
    "Outland/HellfireCitadel/HellfireRamparts/boss_watchkeeper_gargolmar.cpp": "bfec0a2e7e3fb7a125047d5c7a7acafa85a6d89a78337219e0d02679284b5df1",
}
OMOR_PRIMARY = dict(
    map_id=543, entry=17308, encounter_id=1891, bit=0,
    script="boss_omor_the_unscarred", journal_id=528, actor_id=1105, display_id=18237,
)
OMOR_SOURCES = {
    "Outland/HellfireCitadel/HellfireRamparts/hellfire_ramparts.h": "9896de99ab8f2049a445af5168b152387bdd019315d898a44e8bf83d72967bce",
    "Outland/HellfireCitadel/HellfireRamparts/instance_hellfire_ramparts.cpp": "7f907d01ba05818b1dd4f2e945353433569f56a79773722f5d05a622009af49e",
    "Outland/HellfireCitadel/HellfireRamparts/boss_omor_the_unscarred.cpp": "958cac5caa7d131b172988e15943afd5571a48db2013b23010ef50cf3c2bbe2d",
}
HERALD_PRIMARY = dict(
    map_id=543, entry=17307, encounter_id=1892, bit=1,
    script="boss_vazruden_the_herald", journal_id=529, actor_id=1106, display_id=18944,
)
HERALD_SOURCES = {
    "Outland/HellfireCitadel/HellfireRamparts/hellfire_ramparts.h": "9896de99ab8f2049a445af5168b152387bdd019315d898a44e8bf83d72967bce",
    "Outland/HellfireCitadel/HellfireRamparts/instance_hellfire_ramparts.cpp": "7f907d01ba05818b1dd4f2e945353433569f56a79773722f5d05a622009af49e",
    "Outland/HellfireCitadel/HellfireRamparts/boss_vazruden_the_herald.cpp": "7a79dab1f28d5915d77c4d972c5f8f611e95a8bed5640374a490fc73a0806bba",
}
# Header NPC_BLOOD_GUARD_PORUNG20923 and instance encounter1935 disambiguate
# the shared17725 display from Shattered Hand Blood Guard17461.
PORUNG_PRIMARY = dict(
    map_id=540, entry=20923, encounter_id=1935, bit=0,
    script="", ai="SmartAI", journal_id=728, actor_id=1394, display_id=17725,
    spawn_guid=34038, spawn_difficulties="2", retail_build="12.1.5.70448",
)
PORUNG_SOURCES = {
    "Outland/HellfireCitadel/ShatteredHalls/shattered_halls.h": "9327cbd528c9ccf1280cbf69052ee85393fb976b649d6325ee02ea9bf1a7a4b3",
    "Outland/HellfireCitadel/ShatteredHalls/instance_shattered_halls.cpp": "df983cfb054a895f2563dd00a1ece3312906998f5e0dc2d53cd3667bad31ed39",
}

# TC header/instance authenticate the primary, not a nonexistent C++ boss AI.
MELIDRUSSA_PRIMARY = dict(
    map_id=2521, entry=188252, encounter_id=2609, bit=1,
    script="", ai="", journal_id=2488, actor_id=5358, display_id=106891,
    spawn_guid=9001489, spawn_difficulties="1,2,8,23", retail_build="12.1.5.70448",
)
KOKIA_PRIMARY = dict(
    map_id=2521, entry=189232, encounter_id=2606, bit=0,
    script="", ai="", journal_id=2485, actor_id=5354, display_id=106851,
    spawn_guid=9001667, spawn_difficulties="1,2,8,23", retail_build="12.1.5.70448",
)
RUBY_SOURCES = {
    "DragonIsles/RubyLifePools/ruby_life_pools.h": "7e33402fa5acaf55adfcf7fd774fecb377522d82f138da0c26e255c76e874070",
    "DragonIsles/RubyLifePools/instance_ruby_life_pools.cpp": "0a6b5857df9ee918fa1a9c494ca0bb58119d4b91173e59f9ad074764b8ed9c7a",
}

HERALD_RETAIL_BUILD = "12.1.5.70448"
HERALD_CREDIT_POLICY = "ENCOUNTER_CREDIT_FROM_RETAIL_DUNGEONENCOUNTER"
HERALD_IDENTITY_GAP = "HERALD_RETAIL_IDENTITY_GAP"

PROFILES = {
    "melidrussa": (MELIDRUSSA_PRIMARY, RUBY_SOURCES),
    "kokia": (KOKIA_PRIMARY, RUBY_SOURCES),
    "porung": (PORUNG_PRIMARY, PORUNG_SOURCES),
    "omor": (OMOR_PRIMARY, OMOR_SOURCES),
    "gargolmar": (GARGOLMAR_PRIMARY, GARGOLMAR_SOURCES),
    "theradras": (THERADRAS_PRIMARY, THERADRAS_SOURCES),
    "landslide": (LANDSLIDE_PRIMARY, LANDSLIDE_SOURCES),
    "celebras": (CELEBRAS_PRIMARY, CELEBRAS_SOURCES),
    "vyletongue": (VYLETONGUE_PRIMARY, MARAUDON_SOURCES),
    "gizlock": (GIZLOCK_PRIMARY, MARAUDON_SOURCES),
    "noxxion": (NOXXION_PRIMARY, NOXXION_SOURCES),
    "rotgrip": (ROTGRIP_PRIMARY, MARAUDON_SOURCES),
    "razorlash": (RAZORLASH_PRIMARY, MARAUDON_SOURCES),
    "thermaplugg": (THERMAPLUGG_PRIMARY, GNOMEREGAN_SOURCES),
    "crowd-pummeler": (PUMMELER_PRIMARY, GNOMEREGAN_SOURCES),
    "electrocutioner": (ELECTROCUTIONER_PRIMARY, GNOMEREGAN_SOURCES),
    "silverlaine": (SILVERLAINE_PRIMARY, SHADOWFANG_SOURCES),
    "kystia": (PRIMARY, SOURCES),
    "golden-serpent": (GOLDEN_PRIMARY, GOLDEN_SOURCES),
    "koranthal": (KORANTHAL_PRIMARY, KORANTHAL_SOURCES),
    "adarogg": (ADAROGG_PRIMARY, ADAROGG_SOURCES),
    "slagmaw": (SLAGMAW_PRIMARY, SLAGMAW_SOURCES),
    "gordoth": (GORDOTH_PRIMARY, GORDOTH_SOURCES),
    "cobrahn": (COBRAHN_PRIMARY, WAILING_SOURCES),
    "herald": (HERALD_PRIMARY, HERALD_SOURCES),
}

# Normal DB2 bits, not the source SAI ordinals. Journal actors/models are exact
# local matches; Wailing header/instance maps the same eight encounter identities.
WAILING_ROWS = {
    "anacondra": (3671, 585, 0, 474, 1045, 4313, "SmartAI"),
    "kresh": (3653, 587, 2, 477, 1048, 5126, ""),
    "pythas": (3670, 588, 3, 476, 1047, 4214, "SmartAI"),
    "skum": (3674, 589, 4, 478, 1049, 4203, "SmartAI"),
    "serpentis": (3673, 590, 5, 479, 1050, 4215, "SmartAI"),
    "verdan": (5775, 591, 6, 480, 1051, 4256, "SmartAI"),
    "mutanus": (3654, 592, 7, 481, 1052, 4088, "SmartAI"),
}
for name, (entry, encounter, bit, journal, actor, display, ai) in WAILING_ROWS.items():
    PROFILES[name] = (dict(map_id=43, entry=entry, encounter_id=encounter, bit=bit,
                           script="", ai=ai, journal_id=journal, actor_id=actor,
                           display_id=display), WAILING_SOURCES)


def prepare_bindings(connection, bindings):
    rows = []
    for binding in bindings:
        encounter = connection.execute(
            "SELECT MapID,Bit FROM dungeon_encounter WHERE ID=?",
            (binding["encounter_id"],),
        ).fetchone()
        if encounter != (binding["map_id"], binding["bit"]):
            raise ValueError(f"encounter identity changed: {binding['encounter_id']}")
        template = connection.execute(
            "SELECT ScriptName FROM tdb_creature_template WHERE entry=?",
            (binding["entry"],),
        ).fetchone()
        if template != (binding["script"],):
            raise ValueError(f"creature script identity changed: {binding['entry']}")
        if "ai" in binding:
            source_ai = connection.execute(
                "SELECT AIName FROM tdb_creature_template WHERE entry=?",
                (binding["entry"],),
            ).fetchone()
            if source_ai != (binding["ai"],):
                raise ValueError(f"creature AI identity changed: {binding['entry']}")
        if "spawn_guid" in binding:
            spawns = connection.execute(
                'SELECT guid,map,spawnDifficulties FROM tdb_creature WHERE id=? ORDER BY guid',
                (binding['entry'],),
            ).fetchall()
            expected = [(binding['spawn_guid'],binding['map_id'],binding['spawn_difficulties'])]
            if spawns != expected:
                raise ValueError(f"creature spawn identity changed: {binding['entry']}")
        existing = connection.execute(
            "SELECT encounter_id,bit FROM raid_encounter_creature WHERE map_id=? AND creature_entry=?",
            (binding["map_id"], binding["entry"]),
        ).fetchone()
        if existing is not None and existing != (
            binding["encounter_id"],
            binding["bit"],
        ):
            raise ValueError(
                f"existing encounter binding conflicts: {binding['entry']}"
            )
        rows.append(
            tuple(
                int(binding[field])
                for field in [
                    "map_id",
                    "entry",
                    "encounter_id",
                    "bit",
                    "journal_id",
                    "actor_id",
                    "display_id",
                ]
            )
        )
    return rows


def binding_sql(bindings):
    statements = ["BEGIN TRANSACTION;"]
    for row in bindings:
        values = ",".join(map(str, row))
        statements.append(
            "INSERT INTO raid_encounter_creature "
            "(map_id,creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id) "
            f"VALUES({values}) ON CONFLICT(map_id,creature_entry) DO UPDATE SET "
            "encounter_id=excluded.encounter_id,bit=excluded.bit,"
            "journal_encounter_id=excluded.journal_encounter_id,"
            "journal_creature_id=excluded.journal_creature_id,display_id=excluded.display_id;"
        )
    return "\n".join(statements + ["COMMIT;"]) + "\n"


def validate_source_files(source, sources=SOURCES):
    for name, expected in sources.items():
        path = source / name
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f"pinned TC source hash changed: {path}")


def validate_journal(journal, creatures, primary=PRIMARY):
    if journal.suffix == '.csv' and creatures.suffix == '.csv':
        with journal.open() as stream:
            encounters = [(int(row['ID']), int(row['DungeonEncounterID']))
                          for row in csv.DictReader(stream)]
        with creatures.open() as stream:
            actors = [(int(row['ID']), int(row['JournalEncounterID']),
                       int(row['CreatureDisplayInfoID'])) for row in csv.DictReader(stream)]
    else:
        encounters = [(row[4], row[6] & 0xFFFF)
                      for row in read_journal(journal, 0xB88D11CC, 13, {2: 2})]
        actors = [(row[2], row[3] & 0xFFFF, row[4] & 0xFFFFFFFF)
                  for row in read_journal(creatures, 0x81A3C769, 9)]
    if (primary['journal_id'], primary['encounter_id']) not in encounters:
        raise ValueError(
            f"missing journal encounter relation: {primary['encounter_id']}"
        )
    if (primary['actor_id'], primary['journal_id'], primary['display_id']) not in actors:
        raise ValueError(f"missing journal primary actor relation: {primary['entry']}")


def read_herald_identity_csv(path, expected):
    if path is None or path.suffix != '.csv':
        raise ValueError(f'{HERALD_IDENTITY_GAP}: required active70448 CSV input: {path}')
    try:
        content = path.read_bytes()
        rows = csv.DictReader(io.StringIO(content.decode('utf-8')))
        matching = [row for row in rows if row.get('ID') == str(expected['ID'])]
    except (OSError, UnicodeError, csv.Error) as error:
        raise ValueError(f'{HERALD_IDENTITY_GAP}: cannot read {path}: {error}') from error
    if len(matching) != 1:
        raise ValueError(f'{HERALD_IDENTITY_GAP}: expected one {expected["ID"]} row in {path}')
    if any(matching[0].get(key) != str(value) for key, value in expected.items()):
        raise ValueError(f'{HERALD_IDENTITY_GAP}: active70448 identity disagrees in {path}')
    return hashlib.sha256(content).hexdigest()


def read_herald_retail_identity(dungeon, journal, creatures, build):
    if build != HERALD_RETAIL_BUILD:
        raise ValueError(f'{HERALD_IDENTITY_GAP}: expected Retail {HERALD_RETAIL_BUILD}, got {build}')
    primary = HERALD_PRIMARY
    name = 'Vazruden the Herald'
    inputs = [
        (dungeon, dict(ID=primary['encounter_id'], MapID=primary['map_id'],
                       Bit=primary['bit'], DifficultyID=0, Name_lang=name)),
        (journal, dict(ID=primary['journal_id'], DungeonEncounterID=primary['encounter_id'],
                       JournalInstanceID=248, Name_lang=name)),
        (creatures, dict(ID=primary['actor_id'], JournalEncounterID=primary['journal_id'],
                         CreatureDisplayInfoID=primary['display_id'], Name_lang=name)),
    ]
    hashes = {str(path): read_herald_identity_csv(path, expected) for path, expected in inputs}
    return dict(credit_identity_policy=HERALD_CREDIT_POLICY, retail_source_sha256=hashes)


def read_binding_arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--encounter", choices=list(PROFILES), default="kystia")
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--tc-dir", type=Path, required=True)
    parser.add_argument("--journal", type=Path, required=True)
    parser.add_argument("--journal-build", required=True)
    parser.add_argument("--creatures", type=Path, required=True)
    parser.add_argument("--dungeon-encounter", type=Path,
                        help="active70448 DungeonEncounter.csv, required for Herald")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    return parser.parse_args()


def main():
    args = read_binding_arguments()
    primary, sources = PROFILES[args.encounter]
    if 'retail_build' in primary and args.journal_build != primary['retail_build']:
        raise ValueError(f"expected Retail {primary['retail_build']}, got {args.journal_build}")
    validate_source_files(args.tc_dir, sources)
    identity_report = {}
    if args.encounter == 'herald':
        identity_report = read_herald_retail_identity(
            args.dungeon_encounter, args.journal, args.creatures, args.journal_build)
    else:
        validate_journal(args.journal, args.creatures, primary)
    with closing(
        sqlite3.connect(args.db.resolve().as_uri() + "?mode=ro", uri=True)
    ) as conn:
        rows = prepare_bindings(conn, [primary])
    args.output.write_text(binding_sql(rows))
    args.report.write_text(
        json.dumps(
            dict(
                build=args.journal_build,
                tc_revision="a352b1fa",
                source_sha256=sources,
                primary_actor=primary,
                bindings=rows,
                **identity_report,
            ),
            indent=2,
        )
        + "\n"
    )
    print(f"Prepared {len(rows)} primary-actor binding; no database modified")


if __name__ == "__main__":
    main()
