#!/usr/bin/env python3
"""Extract cited constant CastSpell edges from pinned TC class-script bindings.

This is a bounded source-data extractor, not a C++ interpreter. Dynamic spell
expressions remain explicit unresolved calls. See docs/reference/tc-spell-child-edges.md.
"""
import argparse
from functools import lru_cache
import hashlib
import json
from pathlib import Path
import re
import sys

NONCODE = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'', re.S)
NUMBER = re.compile(r'(?:0[xX][0-9a-fA-F]+|[0-9]+)(?:[uUlL]*)\Z')
COORDINATE = re.compile(r'[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)[fF]?\Z')
CALL = re.compile(r'\bCast(?:Custom)?Spell\s*\(')


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def blank_cpp_noncode(source):
    return NONCODE.sub(lambda match: ''.join('\n' if char == '\n' else ' ' for char in match[0]), source)


def closing_delimiter(source, start, opening, closing):
    depth = 0
    for index in range(start, len(source)):
        if source[index] == opening:
            depth += 1
        elif source[index] == closing:
            depth -= 1
            if depth == 0:
                return index
    raise ValueError(f'unclosed {opening!r} at source offset {start}')


def split_arguments(source):
    starts = [0]
    depth = 0
    for index, char in enumerate(source):
        if char in '([{':
            depth += 1
        elif char in ')]}':
            depth -= 1
        elif char == ',' and depth == 0:
            starts.append(index + 1)
    ends = [start - 1 for start in starts[1:]] + [len(source)]
    return [source[start:end].strip() for start, end in zip(starts, ends)]


def literal_integer(expression):
    expression = expression.strip()
    if not NUMBER.fullmatch(expression):
        return None
    digits = expression.rstrip('uUlL')
    return int(digits, 16 if digits.lower().startswith('0x') else 10)


def constant_values(source):
    values = {}
    enum_headers = re.finditer(r'\benum(?:\s+class)?\s+\w*(?:\s*:\s*[^\{]+)?\s*\{', source)
    bodies = []
    for match in enum_headers:
        start = source.index('{', match.start())
        end = closing_delimiter(source, start, '{', '}')
        bodies.append(source[start + 1:end])
    declarations = re.finditer(r'\b(?:constexpr|const)\s+(?:std::)?(?:u?int\d*(?:_t)?|auto)\s+(\w+)\s*=\s*([^;]+);', source)
    pairs = [match.groups() for match in declarations]
    for body in bodies:
        pairs.extend(re.findall(r'\b(\w+)\s*=\s*([^,}]+)', body))
    for name, expression in pairs:
        value = literal_integer(expression)
        if value is None:
            continue
        if name in values and values[name] != value:
            raise ValueError(f'ambiguous constant {name}: {values[name]} versus {value}')
        values[name] = value
    return values


def spell_argument(arguments):
    if len(arguments) < 2:
        return None
    if len(arguments) >= 4 and all(COORDINATE.fullmatch(value) for value in arguments[:3]):
        return arguments[3]
    return arguments[1]


def class_calls(source, code, name):
    declarations = list(re.finditer(r'\b(?:class|struct)\s+' + re.escape(name) + r'\b', code))
    if len(declarations) != 1:
        raise ValueError(f'class {name}: expected one definition, found {len(declarations)}')
    start = code.index('{', declarations[0].end())
    end = closing_delimiter(code, start, '{', '}')
    calls = []
    for match in CALL.finditer(code, start, end):
        opening = code.index('(', match.start())
        closing = closing_delimiter(code, opening, '(', ')')
        arguments = split_arguments(code[opening + 1:closing])
        expression = spell_argument(arguments)
        calls.append({'cast_line': source.count('\n', 0, match.start()) + 1,
                      'expression': expression, 'argument_count': len(arguments)})
    return calls


def resolve_child(expression, constants):
    if expression is None:
        return None, 'unsupported_cast_overload'
    value = literal_integer(expression)
    if value is None and re.fullmatch(r'\w+(?:::\w+)*', expression):
        value = constants.get(expression.split('::')[-1])
    if value is None:
        return None, 'dynamic_spell_expression'
    if value <= 0:
        return None, 'nonpositive_child_id'
    return value, None


def read_child_edges(index, manifest, source_dir, revision):
    @lru_cache(maxsize=None)
    def read_source(filename):
        expected = manifest.get(filename)
        if expected is None:
            raise ValueError(f'{filename}: no pinned manifest entry')
        if expected['revision'] != revision:
            raise ValueError(f'{filename}: source revision differs from {revision}')
        path = source_dir / filename
        if sha256(path) != expected['sha256']:
            raise ValueError(f'{filename}: source hash mismatch')
        text = path.read_text()
        code = blank_cpp_noncode(text)
        return text, code, constant_values(code)

    @lru_cache(maxsize=None)
    def read_calls(filename, name):
        text, code, constants = read_source(filename)
        return class_calls(text, code, name), constants

    edges, unresolved = [], []
    bindings = sorted({(int(root), Path(row['file']).name, row['class'])
                       for root, rows in index.items() for row in rows})
    for root, filename, name in bindings:
        calls, constants = read_calls(filename, name)
        for call in calls:
            record = dict(parent_spell_id=root, file=filename, script_class=name, **call)
            child, reason = resolve_child(call['expression'], constants)
            if reason:
                unresolved.append(dict(record, reason=reason))
            else:
                edges.append(dict(record, child_spell_id=child, kind='tc_constant_cast'))
    filenames = sorted({filename for _, filename, _ in bindings})
    return dict(schema_version=1, source_revision=revision,
                source_files=[dict(file=name, **manifest[name]) for name in filenames],
                edges=edges, unresolved_calls=unresolved,
                coverage=dict(script_bindings=len(bindings), source_files=len(filenames),
                              constant_edges=len(edges), unresolved_calls=len(unresolved)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('source-dir', 'index', 'manifest', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--revision', default='a352b1fa')
    args = parser.parse_args()
    try:
        if args.output.exists():
            raise ValueError(f'refusing existing output: {args.output}')
        data = read_child_edges(json.loads(args.index.read_text()), json.loads(args.manifest.read_text()),
                       args.source_dir, args.revision)
        data['index_sha256'] = sha256(args.index)
        data['manifest_sha256'] = sha256(args.manifest)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(data, indent=2, sort_keys=True) + '\n')
    except (OSError, ValueError, KeyError) as error:
        print(f'TC child extraction failed: {error}', file=sys.stderr)
        return 1
    print(json.dumps(data['coverage'], sort_keys=True))
    return 0


if __name__ == '__main__':
    sys.exit(main())
