#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["luaparser==4.0.0"]
# ///
"""Statically parse boss-mod Lua into a copy-only SQLite schedule catalog.

luaparser supplies an AST (including original line numbers); addon code is never
loaded, imported or executed. Unknown branches are recorded, not guessed.
"""
import argparse
import hashlib
import json
import sqlite3
from contextlib import closing
from pathlib import Path

from luaparser import ast
from luaparser import astnodes as lua

BUILD = '12.1.0.69933'
COLORS = {'cyan': 0, 'green': 0, 'blue': 0, 'yellow': 0, 'orange': 1, 'purple': 1, 'red': 2}
SCHEMA = '''
CREATE TABLE bossmod_encounter(addon TEXT,version TEXT,encounter_id INTEGER,map_id INTEGER,npc_entry INTEGER,source_file TEXT,PRIMARY KEY(addon,encounter_id,npc_entry));
CREATE TABLE bossmod_rule(id INTEGER PRIMARY KEY,addon TEXT,version TEXT,encounter_id INTEGER,trigger TEXT,trigger_spell_id INTEGER,health_pct REAL,required_phase REAL,action TEXT,spell_id INTEGER,delay_ms INTEGER,phase REAL,severity INTEGER,warning_key TEXT,warning_text TEXT,source_file TEXT,source_line INTEGER,event_kind TEXT);
CREATE TABLE bossmod_import_issue(addon TEXT,source_file TEXT,source_line INTEGER,reason TEXT);
CREATE TABLE bossmod_import_meta(build TEXT,parser TEXT);
CREATE TABLE bossmod_static_source(addon TEXT,encounter_id INTEGER,source_file TEXT,kind TEXT,data_json TEXT);
CREATE TABLE bossmod_authority(encounter_id INTEGER PRIMARY KEY,addon TEXT NOT NULL);
CREATE TABLE bossmod_binding(map_id INTEGER,npc_entry INTEGER,encounter_id INTEGER,bit INTEGER,PRIMARY KEY(map_id,npc_entry));
'''
RULE_FIELDS = ['addon', 'version', 'encounter_id', 'trigger', 'trigger_spell_id', 'health_pct', 'required_phase', 'action', 'spell_id', 'delay_ms', 'phase', 'severity', 'warning_key', 'warning_text', 'source_file', 'source_line', 'event_kind']


def method_name(node):
    if isinstance(node, lua.Invoke):
        return node.func.id
    return None


def literal(node):
    if isinstance(node, lua.Number):
        return node.n
    if isinstance(node, lua.String):
        return node.s.decode() if isinstance(node.s, bytes) else node.s
    if isinstance(node, lua.TrueExpr):
        return True
    if isinstance(node, lua.FalseExpr):
        return False
    if isinstance(node, lua.UMinusOp):
        value = literal(node.operand)
        return -value if isinstance(value, (int, float)) else None
    return None


def retail_condition(node):
    name = method_name(node)
    if name == 'Retail':
        return True
    if name in ('Classic', 'Vanilla', 'BC', 'Wrath', 'Cata', 'Mists'):
        return False
    if isinstance(node, lua.ULNotOp):
        value = retail_condition(node.operand)
        return None if value is None else not value
    if isinstance(node, (lua.AndLoOp, lua.OrLoOp)):
        left, right = retail_condition(node.left), retail_condition(node.right)
        if isinstance(node, lua.AndLoOp):
            return False if left is False or right is False else True if left is True and right is True else None
        return True if left is True or right is True else False if left is False and right is False else None
    if isinstance(node, (lua.EqToOp, lua.NotEqToOp)) and expression(node.left) == 'GetLocale()':
        language = literal(node.right)
        if isinstance(language, str):
            matches = language == 'enUS'
            return matches if isinstance(node, lua.EqToOp) else not matches
    return None


def static_number(node):
    value = literal(node)
    if isinstance(value, (int, float)):
        return value
    # Retail's encounter identity: mod:Retail() and retailId or classicId.
    if isinstance(node, lua.OrLoOp) and isinstance(node.left, lua.AndLoOp):
        choice = retail_condition(node.left.left)
        if choice is not None:
            return static_number(node.left.right if choice else node.right)
    return None


def static_value(node, bindings, context):
    """Evaluate data expressions only, never calls or addon control flow."""
    value = static_number(node)
    if value is not None:
        return value
    value = literal(node)
    if value is not None:
        return value
    key = expression(node)
    if key in ('spellId', 'args.spellId'):
        return context.get('trigger_spell_id')
    if key in bindings:
        return bindings[key]
    if isinstance(node, lua.Table):
        values, index = {}, 1
        for entry in node.fields:
            if entry.key is None:
                key, index = index, index + 1
            elif isinstance(entry.key, lua.Name) and not entry.between_brackets:
                key = entry.key.id
            else:
                key = static_value(entry.key, bindings, context)
                if not isinstance(key, (int, float, str, bool)):
                    return None
            values[key] = static_value(entry.value, bindings, context)
        return values
    if isinstance(node, lua.Index):
        table = static_value(node.value, bindings, context)
        key = static_value(node.idx, bindings, context)
        return table.get(key) if isinstance(table, dict) else None
    operations = {lua.AddOp: lambda a, b: a + b, lua.SubOp: lambda a, b: a - b,
                  lua.MultOp: lambda a, b: a * b, lua.FloatDivOp: lambda a, b: a / b}
    operation = operations.get(type(node))
    if operation:
        left, right = static_value(node.left, bindings, context), static_value(node.right, bindings, context)
        if type(left) in (int, float) and type(right) in (int, float):
            if not isinstance(node, lua.FloatDivOp) or right != 0:
                return operation(left, right)
    return None


def bind_assignment(node, bindings, context):
    values = [static_value(value, bindings, context) for value in node.values]
    for index, target in enumerate(node.targets):
        value = values[index] if index < len(values) else None
        if isinstance(target, lua.Index):
            root = target
            while isinstance(root, lua.Index):
                root = root.value
            bindings[expression(root)] = None
        else:
            bindings[expression(target)] = value


def extract_exboss_timeline(source):
    """Preserve the authored config declaration, never its runtime normalization."""
    tree = ast.parse(source)
    bindings = {}
    for node in tree.body.body:
        if isinstance(node, lua.LocalAssign):
            bind_assignment(node, bindings, {})
            if any(isinstance(target, lua.Name) and target.id == 'encounterTriggers' for target in node.targets):
                config = bindings.get('encounterTriggers')
                if not isinstance(config, dict):
                    raise ValueError('encounterTriggers declaration has unresolved keys or values')
                return config
    raise ValueError('encounterTriggers declaration absent')


def flatten_timer_table(values, path=()):
    for key, value in values.items():
        lookup = (*path, key)
        if isinstance(value, dict):
            yield from flatten_timer_table(value, lookup)
        elif type(value) in (int, float) and value > 0:
            yield dict(lookup_path=list(lookup), seconds=value)


def audit_timer_calls(tree, result, timer_vars=None):
    """Preserve timer calls not reachable from an understood event boundary."""
    handled = {r['source_line'] for r in result['rules']} | {g['source_line'] for g in result['issues']}
    for node in all_nodes(tree):
        if isinstance(node, lua.LocalAssign):
            for target, value in zip(node.targets, node.values):
                if isinstance(target, lua.Name) and 'timer' in target.id.lower() and isinstance(value, lua.Table):
                    result['timer_tables'].append(dict(name=target.id, source_line=line(node),
                        values=list(flatten_timer_table(static_value(value, {}, {}) or {}))))
                    issue(result, node, 'timer lookup table requires authored lookup index/phase/duration input: ' + target.id)
        name = method_name(node)
        is_dbm_timer = timer_vars and isinstance(node, lua.Invoke) and expression(node.source) in timer_vars and timer_vars[expression(node.source)][2] == 'timer'
        if name in ('Bar', 'CDBar', 'CastBar') or (is_dbm_timer and name in ('Start', 'TLStart', 'SetTimeline')):
            result['static_timers'].append(dict(source_file=result['source_file'], source_line=line(node), expression=expression(node)))
            if line(node) not in handled:
                issue(result, node, 'timer requires unresolved event/phase/timeline input: ' + expression(node))
    if not any(rule['action'] == 'timer' for rule in result['rules']):
        issue(result, tree, 'no schedulable engage/cast timers; native timeline input or unsupported trigger required')


def all_nodes(node):
    yield node
    for key, value in vars(node).items():
        if key.startswith('_') or key == 'comments':
            continue
        if isinstance(value, lua.Node):
            yield from all_nodes(value)
        elif isinstance(value, list):
            for child in value:
                if isinstance(child, lua.Node):
                    yield from all_nodes(child)


def line(node):
    token = getattr(node, '_first_token', None)
    return token.line if token else 0


def expression(node):
    return ast.to_lua_source(node).strip()


def make_result(file, addon, version):
    return dict(encounter_id=0, map_id=0, npcs=[], addon=addon, version=version,
                source_file=file, rules=[], issues=[], static_timers=[], timer_tables=[], dynamic_timeline=None)


def issue(result, node, reason):
    result['issues'].append(dict(addon=result['addon'], source_file=result['source_file'],
                                 source_line=line(node), reason=reason))


def add_rule(result, node, context, **values):
    rule = dict.fromkeys(RULE_FIELDS)
    rule.update(addon=result['addon'], version=result['version'], encounter_id=result['encounter_id'],
                source_file=result['source_file'], source_line=line(node), **context)
    rule.update(values)
    result['rules'].append(rule)


def spell_value(node, context):
    number = static_number(node)
    if number is not None and number > 0:
        return int(number)
    if expression(node) in ('args.spellId', 'spellId'):
        return context.get('trigger_spell_id')
    return None


def branch_context(test, context, result):
    """Only understood identity/health/stage predicates are eligible to run."""
    if isinstance(test, (lua.LessThanOp, lua.LessOrEqThanOp)) and method_name(test.left) == 'GetHealth':
        threshold = static_number(test.right)
        if threshold is not None:
            return dict(context, trigger='health', health_pct=threshold)
    if isinstance(test, lua.EqToOp):
        value = static_number(test.right)
        if method_name(test.left) == 'GetStage' and value is not None:
            return dict(context, required_phase=value)
        if method_name(test.left) == 'MobId' and value in result['npcs']:
            return context.copy()
        if expression(test.left) in ('spellId', 'args.spellId') and value is not None:
            registered = context.get('trigger_spell_id')
            if registered is None or registered == int(value):
                return dict(context, trigger_spell_id=int(value))
            return None
    return None


def disjoint_dispatch(test, next_test):
    return (isinstance(test, lua.EqToOp) and isinstance(next_test, lua.EqToOp)
            and expression(test.left) == expression(next_test.left)
            and static_number(test.right) is not None
            and static_number(next_test.right) is not None
            and static_number(test.right) != static_number(next_test.right))


def apply_branch_bindings(parent, child, block):
    locals = {expression(target) for node in all_nodes(block) if isinstance(node, lua.LocalAssign)
              for target in node.targets}
    for name in assigned_names(block) - locals:
        if name in parent:
            parent[name] = child.get(name)


def extract_actions(block, result, context, timer_vars=None, bindings=None):
    bindings = dict(bindings or {})
    for node in block.body:
        if isinstance(node, (lua.LocalAssign, lua.Assign)):
            bind_assignment(node, bindings, context)
            continue
        if isinstance(node, (lua.If, lua.ElseIf)):
            choice = retail_condition(node.test)
            chosen = node.body if choice is True else node.orelse if choice is False else None
            if choice is not None:
                if isinstance(chosen, lua.ElseIf):
                    chosen = lua.Block(body=[chosen])
                if isinstance(chosen, lua.Block):
                    child, stopped = extract_actions(chosen, result, context, timer_vars, bindings)
                    apply_branch_bindings(bindings, child, chosen)
                    if stopped:
                        return bindings, True
            else:
                if any(isinstance(n, lua.Return) for n in all_nodes(node)):
                    issue(result, node, 'conditional early return requires unresolved guard: ' + expression(node.test))
                    return bindings, True
                branch = branch_context(node.test, context, result)
                if branch is not None:
                    extract_actions(node.body, result, branch, timer_vars, bindings)
                else:
                    relevant = any(method_name(n) in ('CDBar', 'Bar', 'SetStage', 'Message', 'Start', 'Show') for n in all_nodes(node))
                    if relevant:
                        issue(result, node, 'unsupported conditional: ' + expression(node.test))
                if isinstance(node.orelse, lua.ElseIf) and disjoint_dispatch(node.test, node.orelse.test):
                    extract_actions(lua.Block(body=[node.orelse]), result, context, timer_vars, bindings)
                elif node.orelse:
                    issue(result, node, 'conditional else branch requires preceding guard to be false')
                for name in assigned_names(lua.Block(body=[node])):
                    if name in bindings:
                        bindings[name] = None
            continue
        if isinstance(node, lua.Do):
            child, stopped = extract_actions(node.body, result, context, timer_vars, bindings)
            apply_branch_bindings(bindings, child, node.body)
            if stopped:
                return bindings, True
            continue
        if isinstance(node, lua.Return):
            return bindings, True
        name = method_name(node)
        if not name:
            continue
        args = node.args
        if name in ('CDBar', 'Bar', 'CastBar') and len(args) >= 2:
            spell = spell_value(args[0], context)
            delay = static_value(args[1], bindings, context)
            if spell is None or type(delay) not in (int, float) or delay <= 0:
                issue(result, node, 'nonliteral timer: ' + expression(node))
                continue
            add_rule(result, node, context, action='timer', spell_id=spell,
                     delay_ms=round(delay * 1000), event_kind=name)
        elif name == 'SetStage' and args:
            phase = static_number(args[0])
            if phase is None:
                issue(result, node, 'nonliteral phase: ' + expression(node))
            else:
                add_rule(result, node, context, action='phase', phase=phase)
        elif name == 'Message' and len(args) >= 2:
            spell, color = spell_value(args[0], context), literal(args[1])
            if spell and color in COLORS:
                key = expression(args[2]) if len(args) > 2 else 'spellName'
                add_rule(result, node, context, action='warning', spell_id=spell,
                         severity=COLORS[color], warning_key=key)
            else:
                issue(result, node, 'non-spell/unsupported warning: ' + expression(node))
        elif timer_vars and name in ('Start', 'Show'):
            variable = expression(node.source)
            definition = timer_vars.get(variable)
            if definition is None:
                continue
            spell, default, action, severity, key = definition
            if action == 'timer':
                if spell is None:
                    issue(result, node, 'DBM timer has no statically known spellId: ' + expression(node))
                    continue
                delay = static_value(args[0], bindings, context) if args else default
                if type(delay) not in (int, float) or delay <= 0:
                    issue(result, node, 'nonliteral DBM timer: ' + expression(node))
                    continue
                add_rule(result, node, context, action='timer', spell_id=spell,
                         delay_ms=round(delay * 1000), event_kind='DBM')
            else:
                add_rule(result, node, context, action='warning', spell_id=spell,
                         severity=severity, warning_key=key)
    return bindings, False


def assigned_names(block):
    return {expression(target) for node in all_nodes(block)
            if isinstance(node, lua.Assign) and not isinstance(node, lua.LocalAssign)
            for target in node.targets if isinstance(target, lua.Name)}


def mutated_roots(block):
    names = set()
    for node in all_nodes(block):
        if isinstance(node, lua.Assign) and not isinstance(node, lua.LocalAssign):
            for target in node.targets:
                root = target
                while isinstance(root, lua.Index):
                    root = root.value
                if isinstance(root, lua.Name):
                    names.add(root.id)
    return names


def select_methods(block, bindings=None, mutable=None):
    """Capture lexical constants at each method declaration, not end of file."""
    bindings = dict(bindings or {})
    mutable = mutated_roots(block) if mutable is None else mutable
    for node in block.body:
        if isinstance(node, lua.LocalAssign):
            bind_assignment(node, bindings, {})
            for name in mutable.intersection(bindings):
                bindings[name] = None
        elif isinstance(node, lua.Method):
            yield node, bindings.copy()
        elif isinstance(node, lua.Do):
            yield from select_methods(node.body, bindings, mutable)
        elif isinstance(node, lua.If):
            choice = retail_condition(node.test)
            if choice is True:
                yield from select_methods(node.body, bindings, mutable)
            elif choice is False and isinstance(node.orelse, lua.Block):
                yield from select_methods(node.orelse, bindings, mutable)


def known_statements(block, module_exists=False):
    """Only selected Retail/enUS branches can register handlers or locale data."""
    for node in block.body:
        if isinstance(node, (lua.If, lua.ElseIf)):
            if module_exists and expression(node.test) == 'not mod':
                continue
            choice = retail_condition(node.test)
            if choice is True:
                yield from known_statements(node.body, module_exists)
                if any(isinstance(n, lua.Return) for n in all_nodes(node.body)):
                    return
            elif choice is False:
                if isinstance(node.orelse, lua.Block):
                    yield from known_statements(node.orelse, module_exists)
                    if any(isinstance(n, lua.Return) for n in all_nodes(node.orelse)):
                        return
                elif isinstance(node.orelse, lua.ElseIf):
                    yield from known_statements(lua.Block(body=[node.orelse]), module_exists)
            elif any(isinstance(n, lua.Return) for n in all_nodes(node)):
                return
        elif isinstance(node, lua.Do):
            yield from known_statements(node.body, module_exists)
        elif isinstance(node, lua.Return):
            return
        else:
            yield node


def extract_bigwigs(source, file, addon, version):
    result = make_result(file, addon, version)
    tree = ast.parse(source)
    for node in all_nodes(tree):
        name = method_name(node)
        if name == 'NewBoss' and len(node.args) >= 2:
            result['map_id'] = static_number(node.args[1]) or 0
        elif name == 'RegisterEnableMob':
            result['npcs'] = [int(literal(a)) for a in node.args if isinstance(literal(a), (int, float))]
        elif name == 'SetEncounterID' and node.args:
            result['encounter_id'] = static_number(node.args[0]) or 0
    # Lua replaces a method when a selected Retail branch declares it again.
    methods = {method.name.id: (method, bindings)
               for method, bindings in select_methods(tree.body)}.values()
    handlers = {}
    registrations = [node for method, _ in methods if method.name.id == 'OnBossEnable'
                     for node in known_statements(method.body)]
    for node in registrations:
        if method_name(node) == 'Log' and len(node.args) >= 3:
            event, handler = literal(node.args[0]), literal(node.args[1])
            if event in ('SPELL_CAST_START', 'SPELL_CAST_SUCCESS') and handler:
                for spell in node.args[2:]:
                    value = static_number(spell)
                    if value and value > 0:
                        handlers.setdefault(handler, []).append((event.lower().removeprefix('spell_'), int(value)))
    for method, bindings in methods:
        name = method.name.id
        # Current Retail BossPrototype:Engage dispatches only OnEncounterStart.
        if name == 'OnEncounterStart':
            extract_actions(method.body, result, dict(trigger='engage'), bindings=bindings)
        elif name == 'UNIT_HEALTH':
            extract_actions(method.body, result, dict(trigger='health'), bindings=bindings)
        elif name == 'UNIT_SPELLCAST_SUCCEEDED':
            extract_actions(method.body, result, dict(trigger='cast_success'), bindings=bindings)
        for trigger, spell in handlers.get(name, []):
            extract_actions(method.body, result, dict(trigger=trigger, trigger_spell_id=spell), bindings=bindings)
    audit_timer_calls(tree, result)
    return result


def extract_dbm(source, file, addon, version):
    result = make_result(file, addon, version)
    tree = ast.parse(source)
    variables = {}
    for node in all_nodes(tree):
        name = method_name(node)
        if name == 'SetEncounterID' and node.args:
            result['encounter_id'] = static_number(node.args[0]) or 0
        elif name == 'SetZone' and node.args:
            result['map_id'] = static_number(node.args[0]) or 0
        elif name == 'SetCreatureID':
            result['npcs'] = [int(literal(a)) for a in node.args if isinstance(literal(a), (int, float))]
        if not isinstance(node, lua.LocalAssign) or not node.values:
            continue
        call = node.values[0]
        ctor = method_name(call) or ''
        if not ctor.startswith('New') or not isinstance(node.targets[0], lua.Name):
            continue
        if 'Timer' in ctor and call.args:
            # DBM Timer.lua:1192: custom NewTimer's second arg is a name;
            # its explicit spellId is arg14. Typed NewCD/Next timers use arg2.
            spell_index = 13 if ctor == 'NewTimer' else 1
            delay = static_number(call.args[0])
            spell = static_number(call.args[spell_index]) if len(call.args) > spell_index else None
            if spell and spell > 0:
                variables[node.targets[0].id] = (int(spell), delay, 'timer', None, None)
            else:
                variables[node.targets[0].id] = (None, delay, 'timer', None, None)
                issue(result, call, 'timer constructor lacks statically known spellId: ' + expression(call))
        elif 'Announce' in ctor or 'SpecialWarning' in ctor:
            spell = static_number(call.args[0]) if call.args else None
            if spell and spell > 0:
                severity = 2 if 'SpecialWarning' in ctor else 0
                variables[node.targets[0].id] = (int(spell), None, 'warning', severity, ctor)
    for method, bindings in select_methods(tree.body):
        name = method.name.id
        if name in ('OnCombatStart', 'OnLimitedCombatStart'):
            extract_actions(method.body, result, dict(trigger='engage'), variables, bindings)
        elif name in ('SPELL_CAST_START', 'SPELL_CAST_SUCCESS'):
            trigger = name.lower().removeprefix('spell_')
            extract_actions(method.body, result, dict(trigger=trigger), variables, bindings)
    audit_timer_calls(tree, result, variables)
    return result


def table_value(node):
    """Literal Lua tables only; names/calls remain absent rather than executing."""
    if not isinstance(node, lua.Table):
        return literal(node)
    values = {}
    index = 1
    for field in node.fields:
        key = field.key.id if isinstance(field.key, lua.Name) else literal(field.key)
        if key is None:
            key, index = index, index + 1
        values[key] = (table_value(field.value), line(field))
    return values


def field(table, key, default=None):
    return table.get(key, (default, 0))[0] if isinstance(table, dict) else default


def extract_exboss(source, file, addon, version):
    tree = ast.parse(source)
    root = next((n.values[0] for n in tree.body.body if isinstance(n, lua.LocalAssign)
                 and n.targets[0].id == 'EncounterData'), None)
    if root is None:
        return []
    data = table_value(root)
    encounters = []
    for map_id, (map_data, _) in field(data, 'maps', {}).items():
        for encounter_id, (boss, _) in field(map_data, 'bosses', {}).items():
            result = make_result(file, addon, version)
            result.update(map_id=map_id, encounter_id=encounter_id)
            for event_id, (event, source_line) in field(boss, 'events', {}).items():
                spell = field(event, 'spellID')
                if not isinstance(spell, int) or spell <= 0:
                    continue
                context = dict(trigger='cast_start' if field(event, 'castType') == 'begincast' else 'cast_success', trigger_spell_id=spell)
                rule = dict.fromkeys(RULE_FIELDS)
                rule.update(addon=addon, version=version, encounter_id=encounter_id,
                            source_file=file, source_line=source_line, **context,
                            action='warning', spell_id=spell, severity=field(event, 'severity', 0),
                            warning_key=field(event, 'textLabel') or field(event, 'eventName'))
                result['rules'].append(rule)
                initial, series = field(event, 'firstSeenSec'), field(event, 'cdSeriesSec')
                if isinstance(initial, (int, float)) and initial > 0:
                    timer = dict(rule, trigger='engage', action='timer', delay_ms=round(initial*1000))
                    result['rules'].append(timer)
                if initial is None:
                    result['issues'].append(dict(addon=addon, source_file=file, source_line=source_line,
                        reason=f'event {event_id}: firstSeenSec absent; no authored initial schedule'))
                if series is None:
                    result['issues'].append(dict(addon=addon, source_file=file, source_line=source_line,
                        reason=f'event {event_id}: cdSeriesSec absent; native timeline input required'))
                if series:
                    result['issues'].append(dict(addon=addon, source_file=file, source_line=source_line,
                                                 reason=f'event {event_id}: unsupported observed timer series'))
            encounters.append(result)
    return encounters


def locale_literals(source, prefix):
    """Read literal locale assignments; calls and computed strings stay unresolved."""
    values = {}
    tree = ast.parse(source)
    module_exists = any(method_name(node) in ('NewBoss', 'NewMod') for node in all_nodes(tree))
    for node in known_statements(tree.body, module_exists):
        if isinstance(node, lua.Assign):
            for target, value in zip(node.targets, node.values):
                key = expression(target)
                if key.startswith('L.'):
                    text = literal(value)
                    if isinstance(text, str):
                        values[prefix + key[1:]] = text
                    elif isinstance(value, lua.Table):
                        for name, (text, _) in table_value(value).items():
                            if isinstance(text, str):
                                values[prefix + key[1:] + '.' + str(name)] = text
        call = node.values[0] if isinstance(node, lua.LocalAssign) and node.values else node
        if method_name(call) == 'SetDefaultLocale' and call.args and isinstance(call.args[0], lua.Table):
            for name, (text, _) in table_value(call.args[0]).items():
                if isinstance(text, str):
                    values['L.' + str(name)] = text
    return values


def render_locale_expression(node, spell_name, locales):
    text = literal(node)
    if isinstance(text, (str, int, float)):
        return text
    key = expression(node)
    if key in ('spellName', 'args.spellName'):
        return spell_name
    if key in locales:
        return locales[key]
    if method_name(node) == 'format':
        template = render_locale_expression(node.source, spell_name, locales)
        args = [render_locale_expression(arg, spell_name, locales) for arg in node.args]
        if isinstance(template, str) and all(arg is not None for arg in args):
            try:
                return template % tuple(args)
            except (TypeError, ValueError):
                return None
    return None


def render_warnings(result, spell_names, locales):
    accepted = []
    for rule in result['rules']:
        if rule['action'] != 'warning':
            accepted.append(rule)
            continue
        name = spell_names.get(rule['spell_id'])
        key = rule['warning_key']
        text = None
        if name and key and key.startswith('New'):
            category = 'AUTO_SPEC_WARN_TEXTS' if 'SpecialWarning' in key else 'AUTO_ANNOUNCE_TEXTS'
            suffix = key.removeprefix('NewSpecialWarning').removeprefix('New').removesuffix('Announce').lower()
            template = locales.get('DBM.' + category + '.' + suffix)
            if template and '%%' not in template:
                try:
                    text = template % name
                except (TypeError, ValueError):
                    text = None
        elif name and key and result['addon'] == 'EXBossData':
            # EncounterData labels are plain data, never parse them as Lua.
            if key.isascii() and key.strip():
                text = key
        elif name and key:
            try:
                node = ast.parse('local value = ' + key).body.body[0].values[0]
                text = render_locale_expression(node, name, locales)
            except Exception as error:
                result['issues'].append(dict(addon=result['addon'], source_file=rule['source_file'],
                    source_line=rule['source_line'], reason='warning expression parse failed: ' + str(error)))
        if isinstance(text, str) and text:
            rule['warning_text'] = text
            accepted.append(rule)
        else:
            result['issues'].append(dict(addon=result['addon'], source_file=rule['source_file'],
                source_line=rule['source_line'], reason='unresolved enUS locale warning or dynamic format arguments: ' + str(key)))
    result['rules'] = accepted


def validate_spells(db, encounters):
    builds = dict(db.execute("SELECT table_name,build FROM db2_build WHERE table_name IN ('spell','spell_name')"))
    if builds != {'spell': BUILD, 'spell_name': BUILD}:
        raise ValueError(f'Spell/SpellName must both be imported from {BUILD}: {builds}')
    valid = {row[0] for row in db.execute('SELECT s.ID FROM spell s JOIN spell_name n ON n.ID=s.ID')}
    invalid = sorted({r[k] for e in encounters for r in e['rules'] for k in ('spell_id', 'trigger_spell_id') if r.get(k) and r[k] not in valid})
    if invalid:
        raise ValueError('Unknown Spell/SpellName IDs: ' + ', '.join(map(str, invalid)))


def schedule_authorities(encounters):
    sources = {}
    for encounter in encounters:
        if any(rule['action'] == 'timer' for rule in encounter['rules']):
            sources.setdefault(encounter['encounter_id'], set()).add(encounter['addon'])
    ambiguous = {encounter: sorted(addons) for encounter, addons in sources.items() if len(addons) > 1}
    if ambiguous:
        raise ValueError(f'Conflicting schedule authorities require explicit selection: {ambiguous}')
    return {encounter: next(iter(addons)) for encounter, addons in sources.items()}


def write_host_bindings(out, encounters, authorities):
    for encounter in encounters:
        if authorities.get(encounter['encounter_id']) != encounter['addon']:
            continue
        for npc in encounter['npcs']:
            rows = out.execute('SELECT DISTINCT e.MapID,e.Bit FROM dungeon_encounter e '
                               'WHERE e.ID=? AND e.MapID=? AND EXISTS(SELECT 1 FROM tdb_creature c WHERE c.id=? AND c.map=e.MapID)',
                               (encounter['encounter_id'], encounter['map_id'], npc)).fetchall()
            if len(rows) == 1:
                map_id, bit = rows[0]
                out.execute('INSERT OR IGNORE INTO bossmod_binding VALUES(?,?,?,?)',
                            (map_id, npc, encounter['encounter_id'], bit))


def write_copy(source, target, encounters):
    source, target = Path(source).resolve(), Path(target).resolve()
    if source == target or target.exists():
        raise ValueError('Output must be a new, separate database copy')
    with closing(sqlite3.connect(source.as_uri()+'?mode=ro', uri=True)) as src:
        validate_spells(src, encounters)
        authorities = schedule_authorities(encounters)
        with closing(sqlite3.connect(target)) as out:
            src.backup(out)
            out.executescript(SCHEMA)
            out.execute('INSERT INTO bossmod_import_meta VALUES(?,?)', (BUILD, 'luaparser==4.0.0'))
            out.executemany('INSERT INTO bossmod_authority VALUES(?,?)', authorities.items())
            write_host_bindings(out, encounters, authorities)
            for encounter in encounters:
                for npc in encounter['npcs'] or [0]:
                    out.execute('INSERT OR IGNORE INTO bossmod_encounter VALUES(?,?,?,?,?,?)',
                                tuple(encounter[k] for k in ('addon', 'version', 'encounter_id', 'map_id')) + (npc, encounter['source_file']))
                for kind in ('static_timers', 'timer_tables', 'dynamic_timeline'):
                    values = encounter.get(kind)
                    if values:
                        out.execute('INSERT INTO bossmod_static_source VALUES(?,?,?,?,?)',
                            (encounter['addon'], encounter['encounter_id'], encounter['source_file'], kind, json.dumps(values, ensure_ascii=False)))
                for rule in encounter['rules']:
                    out.execute('INSERT INTO bossmod_rule('+','.join(RULE_FIELDS)+') VALUES('+','.join('?' for _ in RULE_FIELDS)+')',
                                tuple(rule.get(k) for k in RULE_FIELDS))
                for gap in encounter['issues']:
                    out.execute('INSERT INTO bossmod_import_issue VALUES(?,?,?,?)', tuple(gap[k] for k in ('addon', 'source_file', 'source_line', 'reason')))
            out.commit()


def source_version(root):
    head = root/'.git/HEAD'
    if head.exists():
        value = head.read_text().strip()
        if value.startswith('ref: '):
            ref = root/'.git'/value[5:]
            if ref.exists():
                return ref.read_text().strip()
            for row in (root/'.git/packed-refs').read_text().splitlines():
                if row.endswith(' '+value[5:]):
                    return row.split()[0]
        return value
    toc = next(root.glob('*.toc'), None)
    version = next((row.partition(':')[2].strip() for row in toc.read_text().splitlines() if row.startswith('## Version:')), 'unknown') if toc else 'unknown'
    digest = hashlib.sha256()
    for path in sorted(root.rglob('*.lua')):
        digest.update(str(path.relative_to(root)).encode())
        digest.update(path.read_bytes())
    return version + ':' + digest.hexdigest()


def import_sources(db, roots):
    encounters, gaps = [], []
    with closing(sqlite3.connect(Path(db).resolve().as_uri()+'?mode=ro', uri=True)) as conn:
        validate_spells(conn, [])
        spell_names = dict(conn.execute('SELECT s.ID,n.Name_lang FROM spell s JOIN spell_name n ON n.ID=s.ID'))
        valid = set(spell_names)
        common = locale_literals((Path(roots['BigWigs'])/'Locales/enUS_common.lua').read_text(), 'CL') if 'BigWigs' in roots else {}
        for addon, root in roots.items():
            root = Path(root)
            version = source_version(root)
            locales = dict(common)
            if addon == 'DBM':
                locales.update(locale_literals((root/'DBM-Core/localization.en.lua').read_text(), 'DBM'))
            dynamic = {}
            if addon == 'EXBossData':
                dynamic = extract_exboss_timeline((root/'FixedTimelineBosses.lua').read_text())
            paths = [root/'EncounterData.lua'] if addon == 'EXBossData' else sorted(root.rglob('*.lua'))
            for path in paths:
                file = str(path.relative_to(root))
                source = path.read_text(encoding='utf-8-sig')
                if addon != 'EXBossData' and not ('SetEncounterID' in source and ('NewBoss' in source or 'NewMod' in source)):
                    continue
                try:
                    if addon == 'EXBossData':
                        results = extract_exboss(source, file, addon, version)
                    else:
                        extractor = extract_dbm if addon == 'DBM' else extract_bigwigs
                        results = [extractor(source, file, addon, version)]
                except Exception as error:
                    gaps.append(dict(addon=addon, source_file=file, source_line=0, reason=f'parse failure: {error}'))
                    continue
                for result in results:
                    if addon == 'EXBossData':
                        result['dynamic_timeline'] = dynamic.get(result['encounter_id'])
                        if result['dynamic_timeline']:
                            result['issues'].append(dict(addon=addon, source_file='FixedTimelineBosses.lua', source_line=0,
                                reason='durationRules/aiStateMachine classify native timeline input; not initial/repeat cooldowns'))
                    accepted = []
                    for rule in result['rules']:
                        invalid = [rule[k] for k in ('spell_id', 'trigger_spell_id') if rule.get(k) and rule[k] not in valid]
                        if invalid:
                            result['issues'].append(dict(addon=addon, source_file=file, source_line=rule['source_line'], reason='absent pinned Spell/SpellName: '+str(invalid)))
                        else:
                            accepted.append(rule)
                    result['rules'] = accepted
                    render_warnings(result, spell_names, dict(locales, **locale_literals(source, 'L')))
                    if result['encounter_id']:
                        encounters.append(result)
    if encounters and gaps:
        encounters[0]['issues'].extend(gaps)
    return encounters


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--dbm', type=Path, required=True)
    parser.add_argument('--bigwigs', type=Path, required=True)
    parser.add_argument('--littlewigs', type=Path, required=True)
    parser.add_argument('--exboss-data', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    roots = dict(DBM=args.dbm, BigWigs=args.bigwigs, LittleWigs=args.littlewigs, EXBossData=args.exboss_data)
    encounters = import_sources(args.db, roots)
    write_copy(args.db, args.output, encounters)
    coverage = {}
    for addon in roots:
        rows = [e for e in encounters if e['addon'] == addon]
        coverage[addon] = dict(encounters=len({e['encounter_id'] for e in rows}),
                               timed_encounters=len({e['encounter_id'] for e in rows if any(r['action']=='timer' for r in e['rules'])}),
                               rules=sum(len(e['rules']) for e in rows), issues=sum(len(e['issues']) for e in rows),
                               audited_timer_calls=sum(len(e['static_timers']) for e in rows),
                               timer_lookup_tables=sum(len(e['timer_tables']) for e in rows),
                               native_timeline_encounters=sum(e['dynamic_timeline'] is not None for e in rows))
    args.report.write_text(json.dumps(dict(build=BUILD, coverage=coverage, encounters=encounters), ensure_ascii=False, indent=2)+'\n')
    print(json.dumps(coverage))

if __name__ == '__main__':
    main()
