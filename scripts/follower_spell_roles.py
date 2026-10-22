"""Native effect/DB2/TC graph evidence for intrinsic spell roles, independent of APL policy."""
from collections import defaultdict, deque
from functools import lru_cache
import json
from pathlib import Path

# TC a352b1fa SharedDefines.h1348–1511 and SpellAuraDefines.h90–249.
DAMAGE_EFFECTS = frozenset((2, 7, 9, 17, 31, 58, 62, 121, 165))
HEAL_EFFECTS = frozenset((10, 67, 75, 136))
DAMAGE_AURAS = frozenset((3, 43, 53, 89, 162))
HEAL_AURAS = frozenset((8, 20, 62))
AURA_EFFECTS = frozenset((6, 27, 35, 65, 119, 128, 129, 143))
EFFECT_FIELDS = ('ID', 'SpellID', 'EffectIndex', 'Effect', 'EffectAura',
                 'EffectTriggerSpell', 'ImplicitTarget_0', 'ImplicitTarget_1',
                 'EffectBasePointsF')


def read_tc_child_artifact(path):
    data = json.loads(Path(path).read_text())
    if data['schema_version'] != 1 or data['source_revision'] != 'a352b1fa':
        raise ValueError('expected pinned a352b1fa TC child artifact schema1')
    return data


def effect_roles(effect):
    aura = effect['EffectAura'] if effect['Effect'] in AURA_EFFECTS else 0
    damage = effect['Effect'] in DAMAGE_EFFECTS or aura in DAMAGE_AURAS
    # TC SpellEffects.cpp1332–1370 / SpellAuraEffects.cpp5737–5835: leech also heals.
    healing = effect['Effect'] in HEAL_EFFECTS or aura in HEAL_AURAS
    healing = healing or effect['Effect'] == 9 or aura == 53
    return tuple(role for role, applies in (('damage', damage), ('heal', healing)) if applies)


class SpellRoleCatalog:
    """Read-only native graph traversal; caches queries for one unchanged source DB."""
    def __init__(self, db, tc_artifact):
        self.db = db
        self.tc_edges = defaultdict(list)
        self.unresolved = defaultdict(list)
        for edge in tc_artifact['edges']:
            self.tc_edges[edge['parent_spell_id']].append(edge)
        for call in tc_artifact['unresolved_calls']:
            self.unresolved[call['parent_spell_id']].append(call)

    @lru_cache(maxsize=None)
    def read_effects(self, spell):
        fields = ','.join(EFFECT_FIELDS)
        return tuple(dict(zip(EFFECT_FIELDS, row)) for row in self.db.execute(
            f'SELECT {fields} FROM spell_effect WHERE SpellID=? AND DifficultyID=0 ORDER BY EffectIndex',
            (spell,)))

    @lru_cache(maxsize=None)
    def read_role(self, spell):
        facts = self.read_closure(spell)
        evidence = [dict(spell_id=row['SpellID'], effect_id=row['ID'], role=role)
                    for row in facts['closure_effects'] for role in effect_roles(row)]
        roles = {row['role'] for row in evidence}
        if len(roles) == 2:
            intrinsic = 'mixed'
        elif roles:
            intrinsic = next(iter(roles))
        else:
            intrinsic = 'utility'
        return dict(facts, intrinsic=intrinsic, role_evidence=evidence)

    def read_closure(self, spell):
        queue = deque([spell])
        seen = set()
        effects, edges, unresolved, missing = [], [], [], []
        while queue:
            current = queue.popleft()
            if current in seen:
                continue
            seen.add(current)
            native = self.read_effects(current)
            if not native:
                missing.append(current)
            effects.extend(native)
            unresolved.extend(self.unresolved[current])
            children = self.read_child_edges(current, native)
            edges.extend(children)
            queue.extend(edge['child_spell_id'] for edge in children)
        return dict(spell_id=spell, root_effects=list(self.read_effects(spell)), closure_effects=effects,
                    child_edges=edges, unresolved_tc_calls=unresolved,
                    missing_native_effects=sorted(missing))

    def read_child_edges(self, spell, effects):
        native = [dict(parent_spell_id=spell, child_spell_id=row['EffectTriggerSpell'],
                       kind='native_trigger', effect_id=row['ID'])
                  for row in effects if row['EffectTriggerSpell'] > 0]
        return native + self.tc_edges[spell]
