"""Source-qualified, DB2-selected debuff episodes for recorded damage replay."""
def key(event):
    return (event['targetID'], event['abilityGameID'], event['sourceID'],
            event.get('sourceInstance', 0))


def periodic_children(db, spell):
    children = {spell}
    pending = [spell]
    while pending:
        parent = pending.pop()
        for child, in db.execute(
                'SELECT EffectTriggerSpell FROM spell_effect WHERE SpellID=? '
                'AND DifficultyID=0 AND EffectTriggerSpell>0', (parent,)):
            if child not in children:
                children.add(child)
                pending.append(child)
    return children


def convert(events, hits, db, actors, duration_ms):
    if not events:
        return []
    eligible = {row[0] for row in db.execute(
        'SELECT SpellID FROM spell_categories WHERE DifficultyID=0 AND DispelType>0')}
    active = {}
    episodes = []
    for event in sorted(events, key=lambda row: row['fight_ms']):
        if event['targetID'] not in actors or event['abilityGameID'] not in eligible:
            continue
        time = int(event['fight_ms'])
        if time >= duration_ms:
            continue
        identity = key(event)
        kind = event['type']
        if kind == 'removedebuff':
            if identity in active:
                active.pop(identity)['end_ms'] = max(0, time)
            continue
        if kind not in ('applydebuff', 'refreshdebuff', 'applydebuffstack', 'removedebuffstack'):
            continue
        if identity not in active:
            episode = {'source_actor': identity[2], 'source_instance': identity[3],
                       'target_actor': identity[0], 'spell_id': identity[1],
                       'end_ms': duration_ms, 'updates': []}
            active[identity] = episode
            episodes.append(episode)
        updates = active[identity]['updates']
        stacks = event.get('stack', updates[-1]['stacks'] if updates else 1)
        updates.append({'time_ms': max(0, time), 'stacks': stacks, 'evidence': kind})
    episodes = [episode for episode in episodes if episode['updates'][0]['time_ms'] < episode['end_ms']]
    descendants = {episode['spell_id']: periodic_children(db, episode['spell_id'])
                   for episode in episodes}
    for hit in hits:
        if not hit['periodic']:
            continue
        matches = [index for index, episode in enumerate(episodes)
                   if hit['target_actor'] == episode['target_actor']
                   and hit['source_actor'] == episode['source_actor']
                   and hit['source_instance'] == episode['source_instance']
                   and hit['spell_id'] in descendants[episode['spell_id']]
                   and episode['updates'][0]['time_ms'] <= hit['time_ms'] <= episode['end_ms']]
        if len(matches) > 1:
            raise ValueError(f'ambiguous debuff tick episode: {hit}')
        if matches:
            hit['debuff_episode'] = matches[0]
    return episodes
