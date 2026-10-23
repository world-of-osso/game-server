#!/usr/bin/env python3
"""Read-only, stdlib-only Rust source/churn census; no Cargo invocation.

Follow main.rs mod declarations (including #[path] and inline modules), not a
filename prefix heuristic. Edges include imports AND expression/type crate::
paths. Comments/literals are masked; grouped use crate::{a, b::{c}} is handled.
This is a lexical potential-dependency graph, NOT Rust name/cfg resolution:
features/platform alternatives are unioned; direct #[cfg(test)] subtrees are
excluded only from the production view. Macro expansion and super/self aliases
are not resolved. JSON retains file/line witnesses for every edge.
"""
import argparse
import collections
import json
from pathlib import Path
import re
import subprocess


MOD = re.compile(r'((?:#\s*\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*([;{])')
TEST = re.compile(r'#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]')


def mask_literals(source):
    """Preserve character offsets/newlines, including nested block comments."""
    out = list(source)
    i = 0
    while i < len(source):
        end = None
        if source.startswith('//', i):
            end = source.find('\n', i)
            if end < 0:
                end = len(source)
        elif source.startswith('/*', i):
            depth, end = 1, i + 2
            while depth and end < len(source):
                if source.startswith('/*', end):
                    depth, end = depth + 1, end + 2
                elif source.startswith('*/', end):
                    depth, end = depth - 1, end + 2
                else:
                    end += 1
        else:
            raw = re.match(r'(?:br|cr|r)(#{0,255})"', source[i:]) if source[i] in 'bcr' else None
            if raw:
                close = '"' + raw[1]
                pos = source.find(close, i + raw.end())
                if pos < 0:
                    raise ValueError('Unterminated raw string')
                end = pos + len(close)
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
            elif source[i] == "'":
                char = re.match(r"'(?:\\(?:u\{[^}]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'", source[i:])
                if char:
                    end = i + char.end()
        if end is None:
            i += 1
        else:
            for j in range(i, end):
                if out[j] != '\n':
                    out[j] = ' '
            i = end
    return ''.join(out)


def closing_brace(code, start):
    depth = 1
    for pos in range(start + 1, len(code)):
        depth += (code[pos] == '{') - (code[pos] == '}')
        if depth == 0:
            return pos + 1
    raise ValueError('Unbalanced braces')


def test_ranges(code):
    ranges = []
    for attr in TEST.finditer(code):
        end = attr.end()
        # Ignore delimiters inside following attributes/argument lists.
        square = paren = 0
        while end < len(code):
            char = code[end]
            square += (char == '[') - (char == ']')
            paren += (char == '(') - (char == ')')
            if square == paren == 0 and char in '{;':
                end = closing_brace(code, end) if char == '{' else end + 1
                break
            end += 1
        ranges.append((attr.start(), end))
    return ranges


def crate_references(code):
    for match in re.finditer(r'\bcrate\s*::\s*(\w+|\{)', code):
        if match[1] != '{':
            yield match[1], match.start()
            continue
        start = match.end()
        end = closing_brace(code, start - 1) - 1
        depth = 0
        first = True
        for token in re.finditer(r'\w+|[{},]', code[start:end]):
            value = token[0]
            if depth == 0 and first and re.match(r'\w', value):
                yield value, match.start()
                first = False
            if value == '{':
                depth += 1
            elif value == '}':
                depth -= 1
            elif value == ',' and depth == 0:
                first = True


def scc(graph):
    """Tarjan SCCs, including singleton leaves."""
    index, low, stack, active, result = {}, {}, [], set(), []

    def visit(node):
        index[node] = low[node] = len(index)
        stack.append(node)
        active.add(node)
        for target in sorted(graph[node]):
            if target not in index:
                visit(target)
                low[node] = min(low[node], low[target])
            elif target in active:
                low[node] = min(low[node], index[target])
        if low[node] == index[node]:
            component = []
            while True:
                target = stack.pop()
                active.remove(target)
                component.append(target)
                if target == node:
                    break
            result.append(sorted(component))

    for node in sorted(graph):
        if node not in index:
            visit(node)
    return sorted(result, key=lambda component: (-len(component), component))


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True)


def analyze(root, since, until):
    entry = root / 'crates/server/src/main.rs'
    names = {match[2] for match in MOD.finditer(mask_literals(entry.read_text()))}
    records, edges, unresolved = [], [], []

    def walk(path, owner, inherited_test=False, ancestors=()):
        path = path.resolve()
        if path in ancestors:
            raise ValueError(f'Recursive module path: {path}')
        source = path.read_text()
        code = mask_literals(source)
        ranges = test_ranges(code)
        is_test = lambda offset: inherited_test or any(a <= offset < b for a, b in ranges)
        relative = str(path.relative_to(root))
        records.append({'path': relative, 'module': owner, 'test_only': inherited_test,
                        'lines': len(source.splitlines()),
                        'pub_crate': len(re.findall(r'pub\s*\(\s*crate\s*\)', code))})
        for target, offset in crate_references(code):
            if target in names and target != owner:
                edges.append({'from': owner, 'to': target, 'test_only': is_test(offset),
                              'path': relative, 'line': code.count('\n', 0, offset) + 1})
        if owner == '@main':
            for match in re.finditer(r'\b(\w+)\s*::', code):
                if match[1] in names:
                    edges.append({'from': owner, 'to': match[1], 'test_only': is_test(match.start()),
                                  'path': relative, 'line': code.count('\n', 0, match.start()) + 1})

        def descend(text, offset, base, custom_base):
            cursor = 0
            while match := MOD.search(text, cursor):
                attrs, name, delimiter = match.groups()
                absolute = offset + match.start()
                child_owner = name if owner == '@main' else owner
                if delimiter == '{':
                    end = closing_brace(text, match.end() - 1)
                    descend(text[match.end():end - 1], offset + match.end(), base / name, base / name)
                    cursor = end
                    continue
                original_attrs = source[absolute:absolute + len(attrs)]
                custom = re.search(r'#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]', original_attrs)
                candidates = [custom_base / custom[1]] if custom else [base / (name + '.rs'), base / name / 'mod.rs']
                found = [candidate for candidate in candidates if candidate.is_file()]
                if len(found) != 1:
                    unresolved.append({'path': relative, 'module': name, 'candidates': [str(p) for p in candidates]})
                else:
                    walk(found[0], child_owner, is_test(absolute), ancestors + (path,))
                cursor = match.end()

        base = path.parent if path.name in ('main.rs', 'mod.rs') else path.with_suffix('')
        descend(code, 0, base, path.parent)

    walk(entry, '@main')
    modules = sorted(names | {'@main'})
    graphs = {}
    for mode in ('production', 'including_tests'):
        graph = {name: set() for name in modules}
        for edge in edges:
            if mode == 'including_tests' or not edge['test_only']:
                graph[edge['from']].add(edge['to'])
        graphs[mode] = {'edges': {name: sorted(graph[name]) for name in modules}, 'scc': scc(graph)}
    # Default git log traversal counts ordinary reachable commits once, not merge
    # commits. --no-renames keeps name-only records unambiguous; churn is current
    # module ownership, so deleted/historical unmapped paths are not attributed.
    owners = collections.defaultdict(set)
    for record in records:
        owners[record['path']].add(record['module'])
    churn = {name: set() for name in modules}
    revision = git(root, 'rev-parse', 'HEAD').strip()
    log = git(root, 'log', revision, '--no-merges', '--no-renames', '--since=' + since,
              '--until=' + until, '--format=COMMIT:%H', '--name-only', '--', 'crates/server/src')
    commit = None
    for line in log.splitlines():
        if line.startswith('COMMIT:'):
            commit = line[7:]
        elif line in owners:
            for owner in owners[line]:
                churn[owner].add(commit)
    return {'revision': revision, 'since': since, 'until': until,
            'modules': {name: {'commits': len(churn[name]),
                              'files': sum(r['module'] == name for r in records),
                              'lines': sum(r['lines'] for r in records if r['module'] == name),
                              'test_file_lines': sum(r['lines'] for r in records if r['module'] == name and r['test_only']),
                              'pub_crate': sum(r['pub_crate'] for r in records if r['module'] == name)} for name in modules},
            'graphs': graphs, 'files': records, 'witnesses': edges, 'unresolved': unresolved}


def render_markdown(result):
    lines = ['# Server module graph and churn snapshot', '',
             f"Source `{result['revision']}`; UTC interval `{result['since']}` through `{result['until']}`.", '',
             'Generated by `scripts/server-crate-graph.py --markdown`; do not edit tables manually.', '',
             'Edges point from consumer to dependency. P = production potential edges; +T = additional test-inclusive edges.',
             'Churn counts distinct non-merge commits touching currently owned files; file LOC includes comments/blank lines.', '',
             '## Cycles', '']
    for mode, graph in result['graphs'].items():
        cycles = [component for component in graph['scc'] if len(component) > 1]
        lines += [f"### {mode}: {sum(map(len, graph['edges'].values()))} edges", '']
        for component in cycles:
            lines += [f"{len(component)} modules: " + ', '.join(f'`{name}`' for name in component), '']
        lines += ['All other SCCs are singletons (self-edges omitted).', '']
    lines += ['## Full adjacency and churn', '',
              '| Module | 14d commits | LOC (test-only files) | pub(crate) occurrences | P dependencies | +T dependencies |',
              '|---|---:|---:|---:|---|---|']
    for name, stats in result['modules'].items():
        production = result['graphs']['production']['edges'][name]
        extra = sorted(set(result['graphs']['including_tests']['edges'][name]) - set(production))
        lines.append(f"| {name} | {stats['commits']} | {stats['lines']} ({stats['test_file_lines']}) | "
                     f"{stats['pub_crate']} | {', '.join(production) or '—'} | {', '.join(extra) or '—'} |")
    return '\n'.join(lines) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--since', default='14 days ago')
    parser.add_argument('--until', default='now')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--markdown', type=Path)
    args = parser.parse_args()
    result = analyze(args.root.resolve(), args.since, args.until)
    if result['unresolved']:
        raise SystemExit(json.dumps(result['unresolved'], indent=2))
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    if args.markdown:
        args.markdown.write_text(render_markdown(result))
    print(f"{result['revision']}: {len(result['modules']) - 1} top-level modules; "
          f"{len(result['files'])} file visits; output {args.output}")


if __name__ == '__main__':
    main()
