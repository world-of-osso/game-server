# Shared native APL buff facts — 2026-10-24

## Scope and revisions

Branch `apl-buff-duration-react`, worktree `/home/osso/.worktrees/game-server-apl-buff-facts`, based on requested `1466a4ca7`. `origin/master` had advanced when takeover started; the explicit requested base was retained.

Only shared work from `d492492f2`, `9177b4558`, `dd6d8bd25`, `eb2955e36` was ported. No Evoker implementation, source-order/min-max work, pushes or agents. Existing native Evoker spells are execution-time fixture controls only.

- `d8fc98804`: initial real-ID regressions and contract.
- `e42d224cd`: earned refresh/expiry/reacquisition regression, before implementation.
- `1f5e8c7a6`: duration/named-time delegation, reaction policy and cfg-only aura hooks.
- `d5b83f217`: correct fixture's stack-gain-versus-overcap distinction; production implementation unchanged.

## Behavior and proof

| Boundary | Exact behavior | Proof at d5b83f217 |
|---|---|---|
| `buff.tigers_fury.duration` | Real5217 native base/fresh duration, unchanged while remains declines and after natural expiry; existing fresh-duration helper, no duration-minus-remains reconstruction | Native RED → GREEN |
| `buff.clearcasting.react` | Real Omen16864 white-hit proc produces135700; absence0, newly acquired stacks initially unreacted, stack count reacts after source cap, natural expiry0 | Native RED → GREEN |
| Gained stacks / overcap / reacquisition | Earn actual native cap, newly gained stack waits independently, fully reacted capped refresh retains deadlines immediately, expiration then new instance waits again | Native RED → corrected fixture GREEN |
| `action.<alias>.execute_time` | Existing native helper shared with bare contextual execute_time; real rated Living Flame361469 and Disintegrate356995 cast/channel state | Native RED → GREEN; both controls pass |
| Native stack loss | Existing spend-stack and set-self-stack boundaries notify reaction state; removal deletes instance deadlines | Hooks inspected; no separate stack-consumption regression in this slice |

Pinned local SimC `/home/osso/.worktrees/simc-76f8118`: `buff/buff.cpp355–360` reads buff_duration;485–503/2004–2015 returns reacted stack count;2697–2703 assigns deadlines only to newly gained stacks. `player/player.cpp1017–1020,1225–1226,7763–7766` supplies mean/stddev/offset/exponential-nu/cap; `util/rng.hpp387–455` defines truncated normal plus exponential sampling. Explicit reference fields are `.25/.05/.1/.15/1.4` seconds. Configuration must precede tracked acquisition; no implicit profile settings, constant-delay substitute or `up` alias. Active unconfigured/unobserved buffs fail explicitly; genuine absence is zero.

## Commands and results

Every Cargo invocation followed ADD from `CAPACITY_ID=aplbuff timeout 60 /home/osso/.worktrees/logs/capacity.sh`; HOLD was retried at approximately60s intervals. All ran from the requested worktree through:

```text
/home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run aplbuff env TMPDIR=/var/tmp/osso CARGO_TARGET_DIR=/home/osso/.worktrees/game-server-dev-parity/target ...
```

Parent released that target exclusively after14781 died; its source checkout was not modified. New worktree `data` links to canonical server data; no data files are committed.

Native reference control environment:

```text
DPS_PARITY_OUTPUT=/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/dev-evoker-parity-2026-10-22/current-canonical-inputs/20261024T014957Z
DPS_PARITY_WORLD_DB=<DPS_PARITY_OUTPUT>/world.db
```

Input SHA256: profile `d9806d7638db585b00205d5e4a84b8ed5c7757caef74d8accb6baf4c43520e32`; selected traits `0d6c7b9d5db03cf8d34e996e2c0a776212a44b8e346de69ed20debff0b6ac835`. Existing input receipt records the read-only native database snapshot and source provenance.

1. RED at `e42d224cd`: `cargo test -p server-simulation native_apl_shared_ -j2 -- --ignored --nocapture --test-threads=1`. Four genuine unknown buff-property failures after native premises,16.53s; execute-time case initially lacked reference inputs. Log `/var/tmp/osso/aplbuff-red.log`.
2. Corrected execute-time setup, same revision: same command with filter `native_apl_shared_named_execute_time` and reference environment. Genuine unknown action-property execute_time after actual rated cast premise,19.29s. Log `/var/tmp/osso/aplbuff-execute-red.log`.
3. Initial GREEN attempt at `1f5e8c7a6`: four PASS/one FAIL27.24s. The purported overcap refresh actually gained stack2: reacted1 versus stacks2 is correct. Fixture now proves native cap before testing overcap and explicitly checks new-stack delay. Log `/var/tmp/osso/aplbuff-green.log`.
4. Final GREEN at `d5b83f217`: original five-test filter plus reference environment, **5 PASS/0 FAIL/0 ignored**,22.44s; compile15.31s, no warnings. Log `/var/tmp/osso/aplbuff-green-d5b83f217.log`.
5. Requested check, executed **once** at `d5b83f217`: `cargo check -p server -p server-simulation -p server-spell-tests --tests -j2`, **exit0**,1m45s, no warnings. Log `/var/tmp/osso/aplbuff-check.log`.

Changed Rust files formatted with `rustfmt --edition 2024 --config skip_children=true`; manual changed-line readability audit found no blocking issue. Cargo only reordered two existing lockfile package entries; that unrelated normalization was restored. Later documentation-only changes do not invalidate these proofs.

## Remaining gate

Lead owns full integration gate. These receipts qualify shared fact boundaries, not complete Feral/Evoker literal rotations, numerical DPS parity, statistical RNG-stream equivalence or every reaction stack-loss path.

See [contract](../specs/simc-apl.md#native-buff-reaction-duration-and-named-execution-time) and [shared evaluator wiki](../wiki/systems/simc-apl.md#native-buff-duration-and-reaction--2026-10-24).
