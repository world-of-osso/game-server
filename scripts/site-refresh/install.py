#!/usr/bin/env python3
"""Install local refresh assets and user units; never enable or run the timer."""
import argparse
from pathlib import Path
import shutil
import subprocess


def install(exporter):
    source = Path(__file__).resolve().parent
    home = Path.home()
    library = home / ".local/lib/game-server-site-refresh"
    units = home / ".config/systemd/user"
    config = home / ".config/game-server-site-refresh"
    for directory in (library, units, config):
        directory.mkdir(parents=True, exist_ok=True)
    for name in ("refresh.py",):
        shutil.copy2(source / name, library / name)
    shutil.copy2(exporter, library / "game-server-site-export")
    for name in ("game-server-site-refresh.service", "game-server-site-refresh.timer"):
        shutil.copy2(source / name, units / name)
    optouts = config / "opt-outs.json"
    if not optouts.exists():
        with optouts.open("x") as output:
            output.write("[]\n")
        optouts.chmod(0o600)
    subprocess.run(["systemctl", "--user", "daemon-reload"], check=True)
    print(f"Installed {library} and {units}; timer NOT enabled or started")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exporter", type=Path, required=True)
    install(parser.parse_args().exporter)
