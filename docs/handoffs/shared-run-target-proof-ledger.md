# Shared APL run/target proof ledger

Scope: engine-free `server::simc_apl`; no native rotation/DPS qualification.

## Isolated packet

- `42ed29079`: five run/control regressions.
- `e8940cbb7`: explicit `Action.target`, candidate API scaffold and six target regressions; existing native fixture Action constructors updated.
- `e7fdc4dd1`: non-returning runs, min/max target selection and final `target_ids(&Action)` signature.

Do not merge the whole Arms branch for this slice: it also contains independent enabled native REDs and an opt-in schedule RED. Lead accepted these three commits for its `ar1024` integration gate; no full-suite result is claimed here.

## Behavioral coverage

|Feature|Source|Proof|
|---|---|---|
|Satisfied run chooses child; exhausted run never resumes parent|SimC76f8118 `action.cpp4742–4763`|Two genuine REDs→GREEN|
|False run/call return, unknown run list rejection|`action.cpp4648–4689,4742–4763`|Control GREEN and genuine unknown-list RED→GREEN|
|Explicit candidate adapter; unknown target facts reject|`action.cpp5362–5453`; existing strict-fact contract|Two genuine REDs→GREEN|
|Min/max native identity and native-order ties|`action.cpp5411–5449`|Min identity/tie assertions genuineRED→GREEN; max also exercisedGREEN|
|Target choice precedes if; no second-best retry|`action.cpp2435–2468,2619`|Concrete lowest-wound60% target fails health<20 despite10% second target; caller Slam selected|
|Run child reads chosen target; exhausted nested run restores call context|Same run/target boundaries|Genuine inherited-target RED→GREEN; context-restoration control retained|

## Commands and results

All commands used engine `scripts/agent/agent-run arms-parity`, server `scripts/build-host.py --build-host native --test -j 2 -p server --lib simc_apl:: -- --nocapture --test-threads=1`, `TMPDIR=/var/tmp/osso` and admission through `capacity.sh`.

|Revision/scope|Admission|Result|Validity|
|---|---|---|---|
|`a446f0634`, original run tests|`arms-parity`|14PASS/4 assertion RED;1987.53s cold compile/run|Historical missing-run evidence; later API changes require next proof|
|`2088713a6`, target scaffold (library scope unchanged from `e8940cbb7`)|Lead-authorized `lead_arms_14775`|16PASS/8 assertion RED;165.98s; no compiler errors|Direct missing run/target decisions, not a compile-failure substitute|
|`e7fdc4dd1`, implemented run/target code|Same lead-only grant|24/24GREEN;30.51s compile;0warnings/0ignored; exit0|Current relevant Rust scope; later documentation-only updates do not invalidate it|

Raw logs/argv/receipts: engine `data/diagnostics/arms-parity-2026-10-23/apl-{run-red,target-red,run-target-green}.{log,proof.json}`. No broad suite/check/readability gate was substituted for targeted RED/GREEN; lead owns integration verification.

## Remaining boundaries

Native providers must supply action-specific eligible target order and read `Action.target`; dispatch must hit that native identity without accidentally retargeting the actor's primary victim. Native target/debuff/dot/add/boss facts, `spell_targets`, `active_dot`, line cooldowns and the common `retarget_auto_attack` action are separate integration work. The unsupported candidate default is an error, never an actor-wide or first-target fallback.
