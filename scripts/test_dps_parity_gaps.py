import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import dps_parity_gaps as gaps


class GapAliasTests(unittest.TestCase):
    def test_native_children_are_compared_with_reference_parents(self):
        for spec, parent, child in [(252, 47541, 47632), (253, 16827, 49966), (255, 16827, 49966),
                                    (258, 199484, 199486), (258, 447445, 447448),
                                    (262, 51505, 285452), (262, 157375, 269005),
                                    (64, 116, 228597), (64, 30455, 228598),
                                    (64, 199786, 228600), (254, 257044, 257045),
                                    (254, 260243, 260247)]:
            with self.subTest(spec=spec, parent=parent), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                native = {"combat": {"dps": 1000, "actions": [
                    {"spell_id": child, "name": "child", "share_pct": 20},
                ]}}
                reference = {"dps": 1000, "actions": [
                    {"spell_id": parent, "name": "parent", "share_pct": 25},
                ]}
                (root / f"{spec}.native.1.json").write_text(json.dumps(native))
                ref = root / "reference.json"
                ref.write_text(json.dumps(reference))
                with patch.object(gaps, "reference_for", return_value=ref):
                    result = gaps.compare(str(spec), root, 1)
                self.assertEqual(result["rows"][0]["native"], 200)
                self.assertEqual(result["rows"][0]["gap"], 50)
                self.assertEqual(result["extra"], [])

    def test_other_specs_keep_their_native_spell_ids(self):
        actions = [{"spell_id": 49966, "name": "smack", "share_pct": 20}]
        self.assertIn(49966, gaps.per_spell(actions, 1000, gaps.NATIVE_ALIASES.get(254)))


if __name__ == "__main__":
    unittest.main()
