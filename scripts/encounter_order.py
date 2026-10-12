"""Work order only; never configures the runtime Mythic+ season/pool."""
from db2_casc import decode, read_header


DISPLAY_SEASON = 37
JOURNAL_TIER = 505
AVAILABILITY = 156363
CURRENT_ROSTER = {2993, 2813, 2825, 2859, 2923, 1762, 2521, 1877, 3004}
LAYOUTS = {4521365: 0x03958F0D, 1285009: 0xBFCBF1F0,
           1361033: 0x34B02DE8, 801709: 0xAAC13A73, 4279827: 0x95C01F30}


def build_order(maps, tracked, challenges, tiers, journal_maps, lfg, tuning):
    order = {mid: dict(tier=3, sort=[3, -row['ExpansionID'], mid],
                      reason='remaining hosted instance; newest expansion first')
             for mid, row in maps.items()}
    challenge_maps = {r[1]: r[2] & 0xFFFF for r in challenges}
    season_maps = {challenge_maps[r[1]]: (r[0], r[1]) for r in tracked
                   if r[2] == DISPLAY_SEASON and r[1] in challenge_maps}
    for row in lfg:
        mid, tid = row['map_id'], row['tuning_id']
        if mid not in maps or maps[mid]['InstanceType'] != 1 or row['difficulty_id'] != 1 or tid not in tuning:
            continue
        level = tuning[tid]
        # Normal LFG rows with positive sub-cap authored min level are leveling entries.
        if not 0 < level < 90:
            continue
        candidate = dict(tier=2, sort=[2, level, row['id'], mid], lfg_row=row['id'],
                         tuning_row=tid, min_level=level, reason='Normal LFG ContentTuning.MinLevelSquish')
        if order[mid]['tier'] == 3 or candidate['sort'] < order[mid]['sort']:
            order[mid] = candidate
    for row in tiers:
        if row[1] != JOURNAL_TIER or row[4] != AVAILABILITY:
            continue
        mid = journal_maps.get(row[2])
        if mid not in maps or mid not in CURRENT_ROSTER:
            continue
        is_raid = maps[mid]['InstanceType'] == 2
        if not is_raid and mid not in season_maps:
            continue
        order[mid] = dict(tier=1, sort=[1, row[3], mid], journal_tier_row=row[0],
                          journal_instance_row=row[2], reason='JournalTier505 / AvailabilityCondition156363')
        if not is_raid:
            tracked_id, challenge_id = season_maps[mid]
            order[mid].update(tracked_row=tracked_id, challenge_row=challenge_id, display_season_row=DISPLAY_SEASON)
    return order


def read_rows(source, fdid, strings):
    data = (source / f'{fdid}.db2').read_bytes()
    if read_header(data)['layout_hash'] != LAYOUTS[fdid]:
        raise ValueError(f'Unexpected work-order DB2 layout for {fdid}')
    return list(decode(data, strings)[1].values())


def load_order(connection, source, journal_maps):
    # DisplaySeason row37 explicitly names Midnight Season2; this is a work priority,
    # not proof that the server has a season activation/affix policy.
    seasons = read_rows(source, 4279827, {0})
    if not any(r[1] == DISPLAY_SEASON and 'Midnight Season 2' in r[0] for r in seasons):
        raise ValueError('Missing local Midnight Season2 work-order evidence')
    lfg = [dict(id=r[0], map_id=r[10] & 0xFFFF, difficulty_id=r[11] & 0xFFFF, tuning_id=r[32])
           for r in read_rows(source, 1361033, {1, 2})]
    maps = {r['ID']: dict(r) for r in connection.execute('SELECT ID,ExpansionID,InstanceType FROM map')}
    tuning = dict(connection.execute('SELECT ID,MinLevelSquish FROM content_tuning'))
    return build_order(maps, read_rows(source,4521365,set()), read_rows(source,801709,{0}),
                       read_rows(source,1285009,set()), journal_maps, lfg, tuning)
