"""Explicit source bindings where native TriggerSpell and pinned TC leave no edge.

These describe intrinsic child reachability, not unconditional execution or APL
selection. Never infer an edge merely because a description mentions a spell.
"""

# SimC76f8118 sc_druid.cpp4060–4090 /4114–4140 chooses each tick_action;
# native70448 descriptions independently reference these exact damage children.
SIMC_DESCRIPTION_CHILDREN = {
    274837: (274838, 'sc_druid.cpp:4114-4140'),
    1243807: (1244079, 'sc_druid.cpp:4060-4090'),
}


def read_named_child_edges(db, spell):
    if spell not in SIMC_DESCRIPTION_CHILDREN:
        return []
    child, source = SIMC_DESCRIPTION_CHILDREN[spell]
    row = db.execute('SELECT Description_lang FROM spell WHERE ID=?', (spell,)).fetchone()
    if row is None or f'${child}' not in row[0]:
        raise ValueError(f'native spell{spell} description no longer confirms sourced child{child}')
    return [dict(parent_spell_id=spell, child_spell_id=child,
                 kind='named_source_policy', policy='INTRINSIC_CHILD_FROM_SIMC',
                 source_revision='76f8118', source=source,
                 native_description=row[0])]
