"""Private placement safety through the public CLI, without starting a server."""
import os
from pathlib import Path
import subprocess
import sys
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "place-private-character.py"


class PrivatePlacementSafety(unittest.TestCase):
    def invoke(self, socket, account):
        environment = {**os.environ, "GAME_SERVER_ADMIN_SOCKET": socket}
        return subprocess.run(
            [sys.executable, str(SCRIPT), "cooldown-assist", account, "Terrainproof",
             "--admin", "/does-not-exist/never-invoke-admin"],
            env=environment, capture_output=True, text=True, check=False,
        )

    def test_shared_realm_is_refused_before_invoking_admin(self):
        result = self.invoke("/tmp/game-server-admin-5000.sock", "fb_proof")
        self.assertEqual(result.returncode, 2)
        self.assertIn("never 5000", result.stderr)

    def test_implicit_socket_is_refused_before_invoking_admin(self):
        result = self.invoke("", "fb_proof")
        self.assertEqual(result.returncode, 2)
        self.assertIn("explicitly select a private port", result.stderr)

    def test_non_fixture_account_is_refused_before_invoking_admin(self):
        result = self.invoke("/tmp/game-server-admin-5881.sock", "ordinary_player")
        self.assertEqual(result.returncode, 2)
        self.assertIn("fb_* account", result.stderr)


if __name__ == "__main__":
    unittest.main()
