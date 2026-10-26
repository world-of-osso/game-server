"""Exercise the real Cargo/rustfmt discovery boundary used by module registration."""
import runpy
import subprocess
import tempfile
import unittest
from pathlib import Path

FORMATTER = Path(__file__).resolve().parents[1] / "format-spell-scripts.py"


class SpellScriptFormatTests(unittest.TestCase):
    def test_macro_registered_leaf_and_nested_file_are_checked_and_formatted(self):
        format_workspace = runpy.run_path(str(FORMATTER))["format_workspace"]
        with tempfile.TemporaryDirectory(prefix="spell-script-format-") as directory:
            root = Path(directory)
            source = root / "crates/server-spell-scripts/src"
            (source / "modules/hidden").mkdir(parents=True)
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["crates/server-spell-scripts"]\n'
                '[workspace.package]\nedition = "2024"\n'
            )
            (source.parent / "Cargo.toml").write_text(
                '[package]\nname = "formatter-fixture"\nversion = "0.1.0"\n'
                'edition.workspace = true\n'
            )
            (source / "lib.rs").write_text("mod modules;\n")
            (source / "modules.rs").write_text(
                'macro_rules! declare_modules {\n'
                '    ($name:ident, $path:literal) => {\n'
                '        #[path = $path] mod $name;\n'
                '    };\n}\n'
                'declare_modules!(hidden, "modules/hidden.rs");\n'
            )
            leaf = source / "modules/hidden.rs"
            nested = source / "modules/hidden/child.rs"
            good_leaf = (
                '#[path = "hidden/child.rs"]\nmod child;\n'
                'pub fn value() -> u32 {\n    7\n}\n'
            )
            leaf.write_text(good_leaf)
            nested.write_text("pub const VALUE: u32 = 9;\n")
            prepared = subprocess.run(["cargo", "fmt", "--all"], cwd=root)
            self.assertEqual(prepared.returncode, 0)

            bad_leaf = '#[path = "hidden/child.rs"]\nmod child;\npub fn value( )->u32{7}\n'
            bad_nested = "pub const VALUE :u32=9;\n"
            leaf.write_text(bad_leaf)
            nested.write_text(bad_nested)
            self.assertEqual(format_workspace(root, check=True), 1)
            self.assertEqual(leaf.read_text(), bad_leaf)
            self.assertEqual(nested.read_text(), bad_nested)

            self.assertEqual(format_workspace(root, check=False), 0)
            self.assertEqual(leaf.read_text(), good_leaf)
            self.assertEqual(nested.read_text(), "pub const VALUE: u32 = 9;\n")
            self.assertEqual(format_workspace(root, check=True), 0)


if __name__ == "__main__":
    unittest.main()
