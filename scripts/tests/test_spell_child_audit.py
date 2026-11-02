"""Behavioral tests for the source-backed spell-child inventory."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('spell_child_audit', ROOT / 'scripts/spell_child_audit.py')
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class SpellChildInventoryTests(unittest.TestCase):
    def test_literal_const_dynamic_and_aura_children(self):
        with tempfile.TemporaryDirectory() as directory:
            original = audit.MODULES
            try:
                audit.MODULES = Path(directory)
                (audit.MODULES / 'example.rs').write_text('''
const CHILD: u32 = 12_345;
fn run(world: &mut World, target: Entity, child: u32) {
    // cast_triggered(world, target, 99999, None, 0);
    let example = "cast_triggered(world, target, 88888, None, 0)";
    cast_triggered(world, target, CHILD, Some(target), 1);
    try_cast_triggered(world, target, choose(child, other), Some(target), 1);
    let application = AuraApplication { spell_id: 98765, target, caster: target };
    launch_triggered_spell(world, target, &prepared, Some(target), 1);
}
''')
                declared, unresolved = audit.inventory()
                self.assertEqual(declared['example.rs'], {12345, 98765})
                self.assertEqual(unresolved[('example.rs', 'run', 'try_cast_triggered', 'choose(child, other)')], 1)
                self.assertEqual(unresolved[('example.rs', 'run', 'launch_triggered_spell', '&prepared')], 1)
            finally:
                audit.MODULES = original

    def test_comments_nested_and_rust_strings_are_masked(self):
        text = 'a /* one /* two */ three */ r#"cast_triggered(x)"# \'(\' b\n'
        masked = audit.mask_comments_and_strings(text)
        self.assertEqual(len(masked), len(text))
        self.assertEqual(masked.count('\n'), text.count('\n'))
        self.assertNotIn('cast_triggered', masked)
        self.assertNotIn('(', masked)

    def test_manifest_requires_explicit_dynamic_children_and_detects_stale_sites(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / 'children.in'
            site = ('example.rs', 'run', 'try_cast_triggered', 'child')
            constants = {'example.rs': {12345}}
            sites = {site: 1}
            manifest.write_text('C\texample.rs\t12345\n')
            self.assertTrue(any('dynamic child' in error for error in audit.check_manifest(manifest, constants, sites)))
            manifest.write_text('C\texample.rs\t12345\nD\texample.rs\trun\ttry_cast_triggered\tchild\t1\t67890,67891\n')
            self.assertEqual(audit.check_manifest(manifest, constants, sites), [])
            self.assertTrue(any('stale' in error for error in audit.check_manifest(manifest, constants, {})))
            manifest.write_text('D\texample.rs\trun\ttry_cast_triggered\tchild\t1\t?\n')
            with self.assertRaisesRegex(ValueError, 'explicit child ids'):
                audit.read_manifest(manifest)

    def test_unbalanced_call_is_an_error(self):
        with self.assertRaisesRegex(ValueError, 'unterminated'):
            audit.split_arguments('world, actor, something(', 0)


if __name__ == '__main__':
    unittest.main()
