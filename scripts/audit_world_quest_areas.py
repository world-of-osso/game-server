#!/usr/bin/env python3
"""Read local CASC POIs/buffs for the four authenticated world-quest durations.

No activation radius is inferred from a pin. Matching layouts come from WoWDBDefs.
"""
import argparse
import hashlib
import json
import struct
from pathlib import Path

from db2_casc import decode, read_header

AREA_POI_COLUMNS = (
    "Name", "Description", "ID", "X", "Y", "Z", "PortLocID", "PlayerConditionID",
    "Field_12_0_5_66529_006", "UiTextureAtlasMemberID", "Flags", "WMOGroupID",
    "PoiDataType", "PoiData", "Field_9_1_0_38783_011", "ContinentID", "AreaID",
    "WorldStateID", "Field_10_0_0_45141_012", "UiWidgetSetID", "UiTextureKitID",
    "Field_9_1_0_38783_017", "Importance", "Icon",
)


def read_rows(directory, fdid, layout, columns, **options):
    data = (directory / f"{fdid}.db2").read_bytes()
    header = read_header(data)
    if header["layout_hash"] != layout:
        raise ValueError(f"FDID {fdid}: expected layout {layout:08X}, got {header['layout_hash']:08X}")
    _, rows = decode(data, **options)
    mapped = {}
    for row_id, values in rows.items():
        if len(values) != len(columns):
            raise ValueError(f"FDID {fdid}, row {row_id}: {len(values)} values for {len(columns)} declared columns")
        mapped[row_id] = dict(zip(columns, values))
    return mapped, {"fdid": fdid, "layout": f"{layout:08X}", "rows": len(rows), "sha256": hashlib.sha256(data).hexdigest()}


def audit(directory, receipt):
    quest_ids = {row[0] for row in receipt["managed_world_state_quests"]}
    states = receipt["managed_world_states"]
    stage_ids = {state["CurrentStageWorldStateID"] for state in states.values()}
    poi, poi_source = read_rows(directory,1000630,0xA8930CBB,AREA_POI_COLUMNS,
        text_columns={0,1},inline_arrays={3:3},relationship_column=15)
    matching_poi = {key:row for key,row in poi.items() if row["WorldStateID"] in stage_ids
        or row["PoiData"] in quest_ids or any(name in row["Name"].lower()
        for name in ("ivus", "doom's howl", "lion's roar"))}
    for row in matching_poi.values():
        for coordinate in ("X","Y","Z"):
            row[coordinate] = struct.unpack("<f",struct.pack("<I",row[coordinate]))[0]
    poi_states, state_source = read_rows(directory,1000631,0x60943110,
        ("ID","Description","WorldStateValue","IconEnumValue","UiTextureAtlasMemberID","AreaPoiID"),text_columns={1})
    buffs, buff_source = read_rows(directory,1587155,0xB8C3D344,
        ("ID","BuffSpellID","PlayerConditionID","OccurrenceValue","ManagedWorldStateID"))
    return {
        "sources": [poi_source,state_source,buff_source],
        "world_quest_ids": sorted(quest_ids),
        "matching_poi": matching_poi,
        "matching_poi_states": {key:row for key,row in poi_states.items() if row["AreaPoiID"] in matching_poi},
        "matching_managed_buffs": {key:row for key,row in buffs.items() if str(row["ManagedWorldStateID"]) in states},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db2-dir",type=Path,required=True)
    parser.add_argument("--duration-receipt",type=Path,required=True)
    parser.add_argument("--output",type=Path,required=True)
    args = parser.parse_args()
    result = audit(args.db2_dir,json.loads(args.duration_receipt.read_text()))
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps({"rows": [source["rows"] for source in result["sources"]],
        "matching_poi": len(result["matching_poi"]), "matching_poi_states": len(result["matching_poi_states"]),
        "matching_managed_buffs": len(result["matching_managed_buffs"]), "output": str(args.output)}))


if __name__ == "__main__":
    main()
