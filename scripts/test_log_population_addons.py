import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('addons', Path(__file__).with_name('log_population_addons.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class AddonTest(unittest.TestCase):
    def test_traits_and_factory_keep_actual_source_lines(self):
        with tempfile.TemporaryDirectory() as directory:
            traits = Path(directory) / 'TrashMobTraits.lua'
            traits.write_text('rows = {\n {\n npcID = 123,\n coPresenceNPCIDs = { [456] = true },\n'
                              ' spellIDs = { 10, 11 },\n bossEncounter = true,\n },\n }\n')
            factory = Path(directory) / 'Factory.lua'
            factory.write_text('row = {\n ["sourceNPCIDs"] = { 123, 456 },\n'
                               ' ["sourceType"] = "pack",\n ["spellID"] = 10,\n },\n')
            runs = [{'creatures': {'guid': {'npc_id': 123}}}]
            module.annotate(runs, module.read_traits(traits), module.read_factory(factory))
            unit = runs[0]['creatures']['guid']
            self.assertEqual(unit['addon_traits'][0]['coPresenceNPCIDs'], [456])
            self.assertEqual(unit['addon_traits'][0]['spellIDs'], [10, 11])
            self.assertEqual(unit['addon_traits'][0]['line'], 3)
            self.assertEqual(unit['addon_factory_associations'][0],
                             {'source': str(factory), 'line': 2, 'source_type': 'pack', 'spell_id': 10})


if __name__ == '__main__':
    unittest.main()
