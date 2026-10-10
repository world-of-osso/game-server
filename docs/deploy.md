# Server deployment

Verified: 2026-10-09 (deployment script and landed commits; no new live check).

The shared test realm runs on **sakuin**, as the NixOS system `game-server` unit on **UDP 5000**, with state under **`/var/lib/game-server`**. Desktop `osso-5000` instructions describe the retired test-realm location, not this service. Private desktop/local development remains separate; see [remote builds](remote-builds.md).

## Deployment contract

Run the repository's `./deploy.sh` from its checkout. The script ships binaries/data; the sakuin repository's `ops/nixos/game-server.nix` owns the unit, environment and firewall. This documentation reconciliation does not authorize running deployment or restarting services.

- `0dcaa23` selects sakuin and `/var/lib/game-server`, uploads `game-server` and `game-server-admin` into `bin/`, and places `world.db` and economy configuration under `data/`.
- `6bfd7b6` replaces host Cargo compilation with `../game-engine/scripts/desktop-server-build.py --root "$PWD" --release`: Debian bookworm/glibc 2.36 output avoids the observed Arch `GLIBC_2.43 not found` failure on sakuin's glibc 2.42.
- Ground inputs come from `scripts/ground_files.py` into `ground/`; LOS bake output goes into `ground/los/`. The script requires `los/eye_heights.tsv`, assigns ownership to `game-server:game-server`, restarts the system unit and prints its status. It does not upload account/character redb state.

A landed script is not new deployment-health proof. Retain existing runtime receipts separately; no build, upload or service operation was performed for this docs update.

## Sources

- [deploy.sh](../deploy.sh) — exact build/upload/service sequence (`0dcaa23`, `6bfd7b6`).
- [Ground assets](wiki/systems/ground-assets.md) — extraction and geometry proof boundaries.
- [Development commands](wiki/reference/dev-commands.md) — development versus shared-realm workflows.
