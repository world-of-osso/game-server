"""Behavioral fixtures for the source graph; run with Python unittest."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('graph', Path(__file__).with_name('server-crate-graph.py'))
graph = importlib.util.module_from_spec(spec)
spec.loader.exec_module(graph)


class SourceGraphTests(unittest.TestCase):
    def test_literals_nested_comments_and_grouped_paths(self):
        code = graph.mask_literals('''
// crate::comment::bad
/* crate::bad /* nested */ crate::bad */
const S: &str = r###"crate::bad"###;
const C: char = '}';
use crate::{alpha::{A, B}, beta::C};
fn f<'a>() { crate::gamma::run(); }
''')
        self.assertEqual([name for name, _ in graph.crate_references(code)],
                         ['alpha', 'beta', 'gamma'])

    def test_module_ownership_cfg_cycles_and_distinct_commit_churn(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'crates/server/src'
            source.mkdir(parents=True)
            fixtures = {
                'main.rs': 'mod alpha; mod beta; mod gamma;\n',
                'alpha.rs': '''use crate::beta::B;
#[cfg(test)]
#[path = "alpha_tests.rs"]
mod tests;
#[path = "alpha_loader.rs"]
mod loader;
''',
                'alpha_loader.rs': 'use crate::beta::B;\n',
                'alpha_tests.rs': 'use crate::gamma::C;\n',
                'beta.rs': 'use crate::alpha::A;\n',
                'gamma.rs': 'pub struct C;\n',
            }
            for name, text in fixtures.items():
                (source / name).write_text(text)
            def git(*args):
                return subprocess.check_output(['git', '-C', str(root), *args], text=True)
            git('init', '-q')
            git('add', '.')
            git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.test',
                'commit', '-qm', 'Fixture')
            result = graph.analyze(root, '2000-01-01', '2030-01-01')
            self.assertEqual(result['unresolved'], [])
            self.assertEqual(result['graphs']['production']['edges']['alpha'], ['beta'])
            self.assertEqual(result['graphs']['including_tests']['edges']['alpha'], ['beta', 'gamma'])
            self.assertIn(['alpha', 'beta'], result['graphs']['production']['scc'])
            self.assertEqual(result['modules']['alpha']['files'], 3)
            self.assertEqual(result['modules']['alpha']['commits'], 1)
            self.assertEqual(result['modules']['alpha']['test_file_lines'], 1)
            witness = next(edge for edge in result['witnesses'] if edge['to'] == 'gamma')
            self.assertEqual((witness['path'], witness['line'], witness['test_only']),
                             ('crates/server/src/alpha_tests.rs', 1, True))


if __name__ == '__main__':
    unittest.main()
