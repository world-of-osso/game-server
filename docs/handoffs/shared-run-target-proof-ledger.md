# Shared APL run/target proof ledger

Scope: engine-free `server::simc_apl`; no native rotation/DPS qualification.

## Corrected isolated packet

- `42ed29079`: five run/control regressions.
- `e8940cbb7`: explicit `Action.target`, candidate API scaffold and target regressions; existing native fixture Action constructors updated.
- `arms-apl-gate-implementation`: replacement implementation and corrected oracle, assembled from verified `ad720fc3c` plus proof-only documentation. Resolve this local gate ref for the third cherry-pick; never use old `e7fdc4dd1`.

Do not merge the whole Arms branch: independent enabled native REDs and opt-in schedule REDs are not this packet. Lead reports original ar1024 pulled before landing; corrected packet still requires its independent integration gate after ww1025.

## Binary authority and rejected oracle

Pinned SimC76f8118 with the source profile, deterministic seed and explicit custom APL:

```text
actions=auto_attack
actions+=/run_action_list,name=probe_child,target_if=min:target.health.pct
actions.probe_child=mortal_strike,if=target.health.pct<80
initial_resource=rage=100
```

Actual debug selects adds after0.109s, but the first child Mortal Strike is60.056s on main `Fluffy_Pillow`, not6s when adds fall below80%. `player.cpp2467–2476` changes active list pointers, not child targets. At15s the main is95% and the add50%; the replacement test rejects an inherited add cast in exactly that state. Custom probe is not a scoring reference and changes no pinned DPS references.

Old24/24GREEN at `e7fdc4dd1` contained an incorrect inherited-child oracle and is not qualification of that behavior. Correction deletes shared target inheritance state rather than special-casing a list or spec.

## Behavioral coverage

|Feature|Source|Proof|
|---|---|---|
|Satisfied run chooses child; exhausted run never resumes parent|`action.cpp4742–4763`|Original genuine RED→GREEN; retained GREEN|
|False run/call return; unknown run list rejects|`action.cpp4648–4689,4742–4763`|Original RED/control GREEN; retained GREEN|
|Explicit candidate adapter; unknown target facts reject|`action.cpp5362–5453`; strict-fact contract|Original genuine RED→GREEN; retained GREEN|
|Min/max identity and native-order ties; select before if, never retry second best|`action.cpp2435–2468,2619,5411–5449`|Concrete native identities/health/wound values; retained GREEN|
|Run selector does not change child targets or caller target|`player.cpp2467–2476`; actual binary probe|23PASS/1 genuine assertion RED at2d6a66259 →24/24GREEN atad720fc3c|

## Proof ledger

Command: engine `scripts/agent/agent-run arms-parity python3 <server>/scripts/build-host.py --build-host native --test -j 2 -p server --lib simc_apl:: -- --nocapture --test-threads=1`, `TMPDIR=/var/tmp/osso`, admission `lead_arms_14775` (lead grant only for this shared slice).

|Revision/scope|Result|Validity|
|---|---|---|
|`a446f0634`, original run tests|14PASS/4 assertion RED|Historical missing-run evidence|
|`2088713a6`, target scaffold|16PASS/8 assertion RED; no compiler errors|Historical missing run/target decisions|
|`e7fdc4dd1`, original implementation|24/24GREEN|Inherited-child oracle invalidated by actual binary; not accepted for landing|
|`2d6a66259`, corrected source-backed child oracle|23PASS/1 assertion RED;507.75s warm-dependency rebuild/run|Actual wrong result: child Mortal Strike selected on add42 instead of no action|
|`ad720fc3c`, target inheritance deleted|24/24GREEN;28.61s compile;0compiler warnings/0ignored; exit0|Current relevant Rust scope; subsequent documentation-only updates preserve proof|

Receipts: engine `data/diagnostics/arms-parity-2026-10-23/71.run-context-{audit.*,conclusion.json}`, `apl-run-context-{red,green}.{log,proof.json}` and earlier `apl-{run-red,target-red,run-target-green}.{log,proof.json}`. No broad suite/check/readability result substituted for targeted RED/GREEN; lead owns integration verification.

## Remaining boundaries

Native providers supply action-specific eligible target order and honor each returned action's own `Action.target` without changing primary/autoattack victim. Native target/debuff/dot/add/boss facts, `spell_targets`, `active_dot`, line cooldowns and `retarget_auto_attack` remain separate integration work. Missing candidate adapters error; never use actor-wide or first-target fallbacks.
