"""External JSON contract for the standalone pinned-TC child-edge extractor."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SOURCE = '''enum WarriorSpells {
    SPELL_WARRIOR_SHIELD_CHARGE_EFFECT = 385954,
};
class spell_warr_shield_charge : public SpellScript {
    void HandleDummy() {
        GetCaster()->CastSpell(GetHitUnit(), SPELL_WARRIOR_SHIELD_CHARGE_EFFECT, true);
        // GetCaster()->CastSpell(GetHitUnit(), 999999, true);
        auto message = "CastSpell(target, 888888, true)";
    }
};
class literal_cast : public SpellScript {
    void Hit() { GetCaster()->CastSpell(GetHitUnit(), 5176, true); }
};
class dynamic_cast_script : public SpellScript {
    void Hit() { GetCaster()->CastSpell(GetHitUnit(), GetEffectInfo(EFFECT_0).TriggerSpell, true); }
};
'''


class TcSpellChildrenTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.cpp = self.root / 'spell_warrior.cpp'
        self.cpp.write_text(SOURCE)
        self.manifest = self.root / 'manifest.json'
        self.manifest.write_text(json.dumps({'spell_warrior.cpp': {
            'revision': 'a352b1fa', 'sha256': hashlib.sha256(self.cpp.read_bytes()).hexdigest(),
        }}))
        self.index = self.root / 'index.json'
        rows = {385952: 'spell_warr_shield_charge', 10: 'literal_cast', 11: 'dynamic_cast_script'}
        self.index.write_text(json.dumps({str(root): [{'file': 'spell_warrior.cpp',
            'class': name, 'base': 'SpellScript'}] for root, name in rows.items()}))

    def extract(self, name='edges.json'):
        output = self.root / name
        result = subprocess.run([sys.executable, str(ROOT / 'scripts/extract_tc_spell_children.py'),
            '--source-dir', str(self.root), '--index', str(self.index),
            '--manifest', str(self.manifest), '--output', str(output)],
            capture_output=True, text=True)
        return result, output

    def test_constant_enum_and_literal_calls_produce_cited_child_edges(self):
        result, output = self.extract()
        self.assertEqual(result.returncode, 0, result.stderr)
        data = json.loads(output.read_text())
        pairs = {(row['parent_spell_id'], row['child_spell_id']) for row in data['edges']}
        self.assertEqual(pairs, {(385952, 385954), (10, 5176)})
        edge = next(row for row in data['edges'] if row['parent_spell_id'] == 385952)
        self.assertEqual(edge['file'], 'spell_warrior.cpp')
        self.assertEqual(edge['cast_line'], 6)
        self.assertEqual(edge['script_class'], 'spell_warr_shield_charge')
        self.assertEqual(data['schema_version'], 1)
        self.assertEqual(data['source_revision'], 'a352b1fa')

    def test_dynamic_call_is_recorded_without_an_invented_child(self):
        result, output = self.extract()
        self.assertEqual(result.returncode, 0, result.stderr)
        data = json.loads(output.read_text())
        self.assertFalse(any(row['parent_spell_id'] == 11 for row in data['edges']))
        dynamic = [row for row in data['unresolved_calls'] if row['parent_spell_id'] == 11]
        self.assertEqual(len(dynamic), 1)
        self.assertEqual(dynamic[0]['expression'], 'GetEffectInfo(EFFECT_0).TriggerSpell')
        self.assertEqual(dynamic[0]['reason'], 'dynamic_spell_expression')

    def test_source_hash_mismatch_is_an_explicit_failure_without_output(self):
        self.cpp.write_text(SOURCE + '\n// changed input\n')
        result, output = self.extract()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('source hash mismatch', result.stderr)
        self.assertFalse(output.exists())

    def test_repeated_extraction_is_byte_identical(self):
        first, first_output = self.extract('first.json')
        second, second_output = self.extract('second.json')
        self.assertEqual(first.returncode, 0, first.stderr)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(first_output.read_bytes(), second_output.read_bytes())


if __name__ == '__main__':
    unittest.main()
