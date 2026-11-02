#!/usr/bin/env python3
"""Inventory script-triggered spells without executing their runtime callbacks.

The manifest is line-oriented so independent modules can union-merge declarations.
A dynamic call must name its source expression and explicitly enumerate its possible
children (or identify an externally supplied native spell). It is never inferred
from source shape alone.
"""
import argparse
from collections import Counter, defaultdict
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
MODULES = ROOT / 'crates/server-spell-scripts/src/modules'
MANIFEST = ROOT / 'crates/server-spell-scripts/src/child_spells.in'
# These APIs either load a child by ID or launch an already-loaded SpellInfo.
# The argument index is zero based. Every dynamic expression is reported.
APIS = {
    'cast_triggered': 2,
    'try_cast_triggered': 2,
    'cast_triggered_with_overrides': 2,
    'cast_triggered_with_effect0_amount': 2,
    'cast_triggered_at_destination': 2,
    'cast_costed_trigger_with_context': 2,
    'load_triggered_spell': 1,
    'launch_triggered_spell': 2,
    'launch_triggered_spell_with_primary': 2,
    'creature_cast': 2,
}


def mask_comments_and_strings(source):
    """Keep offsets and newlines while masking Rust comments and string literals."""
    result = list(source)
    i = 0
    while i < len(source):
        end = None
        if source.startswith('//', i):
            end = source.find('\n', i)
            if end < 0:
                end = len(source)
        elif source.startswith('/*', i):
            depth = 1
            end = i + 2
            while end < len(source) and depth:
                if source.startswith('/*', end):
                    depth += 1
                    end += 2
                elif source.startswith('*/', end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
            if depth:
                raise ValueError('unterminated Rust block comment')
        else:
            raw = re.match(r'(?:b)?r(#{0,255})"', source[i:])
            if raw:
                terminator = '"' + raw.group(1)
                closing = source.find(terminator, i + raw.end())
                if closing < 0:
                    raise ValueError('unterminated Rust raw string')
                end = closing + len(terminator)
            elif source[i] == "'":
                char = re.match(r"'(?:\\\\.|[^'\\\\\n])'", source[i:])
                if char:
                    end = i + char.end()
            elif source[i] == '"':
                end = i + 1
                while end < len(source):
                    if source[end] == '\\':
                        end += 2
                    elif source[end] == '"':
                        end += 1
                        break
                    else:
                        end += 1
                else:
                    raise ValueError('unterminated Rust string')
        if end is None:
            i += 1
            continue
        for j in range(i, end):
            if source[j] != '\n':
                result[j] = ' '
        i = end
    return ''.join(result)


def split_arguments(source, begin):
    """Split a call's top-level arguments, preserving nested Rust delimiters."""
    stack = []
    args = []
    start = begin
    pairs = {')': '(', ']': '[', '}': '{'}
    for i in range(begin, len(source)):
        c = source[i]
        if c in '([{':
            stack.append(c)
        elif c in ')]}':
            if c == ')' and not stack:
                args.append(source[start:i].strip())
                return args
            if not stack or stack.pop() != pairs[c]:
                raise ValueError(f'unbalanced Rust call near offset {i}')
        elif c == ',' and not stack:
            args.append(source[start:i].strip())
            start = i + 1
    raise ValueError(f'unterminated Rust call at offset {begin}')


def split_fields(source, begin):
    """Read the top-level fields of a Rust struct literal."""
    stack = []
    fields = []
    start = begin
    pairs = {')': '(', ']': '[', '}': '{'}
    for i in range(begin, len(source)):
        c = source[i]
        if c in '([{':
            stack.append(c)
        elif c in ')]}':
            if c == '}' and not stack:
                fields.append(source[start:i].strip())
                return fields
            if not stack or stack.pop() != pairs[c]:
                raise ValueError(f'unbalanced Rust struct near offset {i}')
        elif c == ',' and not stack:
            fields.append(source[start:i].strip())
            start = i + 1
    raise ValueError(f'unterminated Rust struct at offset {begin}')


def literal(expr):
    match = re.fullmatch(r'(\d[\d_]*)(?:u32)?', expr.strip())
    return int(match.group(1).replace('_', '')) if match else None


def scan():
    """Yield (path, function, API, argument, optional literal/const id)."""
    files = sorted(MODULES.rglob('*.rs'))
    sources = {p: mask_comments_and_strings(p.read_text()) for p in files}
    local_constants = {}
    for path, source in sources.items():
        local_constants[path] = {
            name: int(value.replace('_', ''))
            for name, value in re.findall(
                r'\bconst\s+([A-Za-z_][A-Za-z_0-9]*)\s*:\s*u32\s*=\s*(\d[\d_]*)\s*;', source
            )
        }
    calls = re.compile(r'\b(' + '|'.join(sorted(APIS, key=len, reverse=True)) + r')\s*\(')
    applications = re.compile(r'\bAuraApplication\s*\{')
    functions = re.compile(r'\bfn\s+([A-Za-z_][A-Za-z_0-9]*)\s*(?:<[^;{}]*>)?\s*\(')
    for path, source in sources.items():
        names = list(functions.finditer(source))
        for match in calls.finditer(source):
            previous = source[:match.start()].rstrip()
            if previous.endswith('fn'):
                continue
            argv = split_arguments(source, match.end())
            index = APIS[match.group(1)]
            if len(argv) <= index:
                raise ValueError(f'{path}: malformed call to {match.group(1)}')
            expr = ' '.join(argv[index].split())
            value = literal(expr)
            if value is None:
                value = local_constants[path].get(expr)
            function = next((n.group(1) for n in reversed(names) if n.start() < match.start()), '<module>')
            yield (str(path.relative_to(MODULES)), function, match.group(1), expr, value)
        for match in applications.finditer(source):
            fields = split_fields(source, match.end())
            spell_fields = [field for field in fields if re.match(r'^spell_id(?:\s*:|\s*$)', field)]
            if len(spell_fields) != 1:
                raise ValueError(f'{path}: AuraApplication missing explicit spell_id')
            field = spell_fields[0]
            expr = ' '.join(field.split(':', 1)[1].split()) if ':' in field else 'spell_id'
            value = literal(expr)
            if value is None:
                value = local_constants[path].get(expr)
            function = next((n.group(1) for n in reversed(names) if n.start() < match.start()), '<module>')
            yield (str(path.relative_to(MODULES)), function, 'AuraApplication', expr, value)


def inventory():
    constants = defaultdict(set)
    dynamic = Counter()
    for path, function, api, expr, value in scan():
        if value is None:
            dynamic[(path, function, api, expr)] += 1
        else:
            constants[path].add(value)
    return constants, dynamic


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--inventory', action='store_true')
    parser.add_argument('--generate', action='store_true')
    args = parser.parse_args()
    constants, dynamic = inventory()
    if args.inventory or args.generate:
        for path in sorted(constants):
            for child in sorted(constants[path]):
                print(f'C\t{path}\t{child}')
        for (path, function, api, expr), count in sorted(dynamic.items()):
            print(f'D\t{path}\t{function}\t{api}\t{expr}\t{count}\t?')
        print(f'{sum(map(len, constants.values()))} literal children; {len(dynamic)} unresolved sites', file=sys.stderr)
        return 0
    parser.error('use --inventory or --generate until the declaration audit is installed')


if __name__ == '__main__':
    sys.exit(main())
