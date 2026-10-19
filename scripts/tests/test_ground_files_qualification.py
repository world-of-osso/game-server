"""Before-only ground metadata observations; no extraction or decoder replacement."""

from contextlib import redirect_stderr, redirect_stdout
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ground_files


def observe(action):
    stdout, stderr = io.StringIO(), io.StringIO()
    error = None
    with redirect_stdout(stdout), redirect_stderr(stderr):
        try:
            action()
        except (Exception, SystemExit) as caught:
            error = {
                "type": f"{type(caught).__module__}.{type(caught).__qualname__}",
                "message": str(caught),
            }
    return {"stdout": stdout.getvalue(), "stderr": stderr.getvalue(), "error": error}


def observe_walk(case):
    files = ground_files.GroundFiles(Path(case["cache"]))

    def walk():
        for map_id, wdt in case["maps"]:
            files.start_map(map_id)
            if wdt:
                files.add_map(wdt)
            else:
                files.missing.append((map_id, ground_files.NO_WDT))

    result = observe(walk)
    result["present"] = list(files.present)
    result["missing"] = files.missing
    return result


def observe_main(case, missing):
    arguments = [
        sys.executable, ground_files.__file__, "--world-db", case["world_db"],
        "--cache", case["cache"], *case["args"],
    ]
    if missing:
        arguments.append("--missing")
    result = subprocess.run(arguments, capture_output=True, text=True, check=False)
    return {"stdout": result.stdout, "stderr": result.stderr, "exit_code": result.returncode}


@unittest.skipUnless(os.environ.get("WOW_GROUND_INPUTS"), "explicit qualification manifest required")
class GroundFilesQualificationTest(unittest.TestCase):
    def test_capture_before_only(self):
        manifest = json.loads(Path(os.environ["WOW_GROUND_INPUTS"]).read_text())
        output = Path(os.environ["WOW_GROUND_OUTPUT"])
        output.mkdir()  # Existing receipts must never be overwritten.
        outcomes = {}
        for case in manifest["cases"]:
            result = {
                "name": case["name"], "kind": case["kind"],
                "walk": observe_walk(case),
                "main": observe_main(case, False),
                "main_missing": observe_main(case, True),
            }
            outcomes[case["name"]] = result
            (output / f'{case["name"]}.json').write_text(
                json.dumps(result, indent=2) + "\n"
            )
        self.assertEqual(len(outcomes), len(manifest["cases"]))
        self.assertTrue(any(case["kind"] == "real" for case in manifest["cases"]))
        self.assertEqual(
            outcomes["complete_ordered_closure"]["walk"]["present"],
            ["terrain/1.wdt", "terrain/10.adt", "terrain/11.adt", "models/20.wmo", "models/30.wmo"],
        )
        self.assertEqual(
            outcomes["complete_ordered_closure"]["main"]["stdout"],
            "terrain/1.wdt\nterrain/10.adt\nterrain/11.adt\nmodels/20.wmo\nmodels/30.wmo\n",
        )
        self.assertIsNone(outcomes["gfid_length_4"]["walk"]["error"])
        self.assertEqual(outcomes["gfid_length_4"]["walk"]["missing"], [])
        self.assertEqual(outcomes["gfid_length_8"]["walk"]["missing"], [(10, "models/31.wmo")])
        self.assertEqual(
            outcomes["global_first_wmo"]["walk"]["present"],
            ["terrain/1.wdt", "models/20.wmo", "models/30.wmo"],
        )
        self.assertEqual(outcomes["duplicate_maid_first_empty"]["walk"]["present"], ["terrain/1.wdt"])
        (output / "outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
        print(f"Captured {len(outcomes)} independent walk/main/missing outcomes")


if __name__ == "__main__":
    unittest.main()
