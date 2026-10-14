#!/usr/bin/env python3
"""Prepare source-authenticated primary-actor binding SQL; never modify world.db.

Pinned TC instance/header rows identify Kystia, Golden Serpent, Koranthal,
Adarogg, Slagmaw, Gordoth, Wailing Caverns and Silverlaine primaries.
Companions never acquire completion credit. The lead applies generated SQL to
reviewed copies; structural binding is not mechanic or native certification.
"""

import argparse
import hashlib
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
PROFILES = {
    "silverlaine": (SILVERLAINE_PRIMARY, SHADOWFANG_SOURCES),
    "kystia": (PRIMARY, SOURCES),
    "golden-serpent": (GOLDEN_PRIMARY, GOLDEN_SOURCES),
    "koranthal": (KORANTHAL_PRIMARY, KORANTHAL_SOURCES),
    "adarogg": (ADAROGG_PRIMARY, ADAROGG_SOURCES),
    "slagmaw": (SLAGMAW_PRIMARY, SLAGMAW_SOURCES),
    "gordoth": (GORDOTH_PRIMARY, GORDOTH_SOURCES),
    "cobrahn": (COBRAHN_PRIMARY, WAILING_SOURCES),
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
    encounters = read_journal(journal, 0xB88D11CC, 13, {2: 2})
    actors = read_journal(creatures, 0x81A3C769, 9)
    if not any(
        r[4] == primary["journal_id"] and r[6] & 0xFFFF == primary["encounter_id"]
        for r in encounters
    ):
        raise ValueError(
            f"missing journal encounter relation: {primary['encounter_id']}"
        )
    if not any(
        r[2] == primary["actor_id"]
        and r[3] & 0xFFFF == primary["journal_id"]
        and r[4] & 0xFFFFFFFF == primary["display_id"]
        for r in actors
    ):
        raise ValueError(f"missing journal primary actor relation: {primary['entry']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--encounter", choices=list(PROFILES), default="kystia")
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--tc-dir", type=Path, required=True)
    parser.add_argument("--journal", type=Path, required=True)
    parser.add_argument("--creatures", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    primary, sources = PROFILES[args.encounter]
    validate_source_files(args.tc_dir, sources)
    validate_journal(args.journal, args.creatures, primary)
    with closing(
        sqlite3.connect(args.db.resolve().as_uri() + "?mode=ro", uri=True)
    ) as conn:
        rows = prepare_bindings(conn, [primary])
    args.output.write_text(binding_sql(rows))
    args.report.write_text(
        json.dumps(
            dict(
                build="12.1.0.69933",
                tc_revision="a352b1fa",
                source_sha256=sources,
                primary_actor=primary,
                bindings=rows,
            ),
            indent=2,
        )
        + "\n"
    )
    print(f"Prepared {len(rows)} primary-actor binding; no database modified")


if __name__ == "__main__":
    main()
