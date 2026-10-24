# Generic health aura states — independent gate

## Authorized scope

Branch `aura-state-health` starts at exact master `461aa81024c2b1d444729fc0ba9961b213d51e7e`. Lead assigned only the seven TC health-threshold states beside existing Druid periodic-heal15. No Ret-local predicate, new spell metadata, other computed states, cached flags or timers. `aura_spell_restrictions.rs` and its Some(false)-only rejection rule remain untouched.

## Pinned source

TC a352b1fa actually sets the health-state siblings in `Unit::Update`, `Unit.cpp481–487`. `ModifyHealth8616–8655` and `SetHealth10039–10067` mutate the backing current health. `Unit.h804–809` uses strict `<`/`>` against uint64 `CountPctFromMaxHealth`; percentage results truncate. `SharedDefines.h2832–2861` supplies IDs/names. Native integer health counts are read from current `Health` each query; no cached aura-state component is introduced.

| ID / TC state | Predicate | Proof status |
|---|---|---|
|2 WOUNDED_20_PERCENT|current < count20%max|genuine core RED; native full-health24275 RED/below20controlPASS|
|6 WOUNDED_25_PERCENT|current < count25%max|genuine core RED|
|13 WOUNDED_35_PERCENT|current < count35%max|genuine core RED|
|21 WOUND_HEALTH_20_80|current < count20%max or current > count80%max|genuine core RED|
|23 HEALTHY_75_PERCENT|current > count75%max|genuine core RED|
|24 WOUND_HEALTH_35_80|current < count35%max or current > count80%max|genuine core RED|
|25 WOUNDED_50_PERCENT|current < count50%max|genuine core RED|

Tests include equality and neighboring values at every threshold, non-round maximums101/119, all other unknown state IDs and missing Health. Existing unknown-state Swiftmend control previously used state2; intentional new support requires using still-uncomputed state4/Frozen instead. Its contract remains unknown≠false. Existing state15 behaviors remain separate retained controls.

## Strict RED

Revision `fbdc5ee0d889f3db0c0fb78e71acaded10e9d5b4`:

- Core `health_aura_state`:8genuine missing-state failures/2unknown-or-missing-Health controlsPASS;0.00stest,11m43cold compile, zero warnings.
- Native `health_aura_state_native`: real31146 full-health ordinary24275 incorrectly returnsOk instead of TargetAuraState (genuine RED). Real31146 with current health prepared below20% accepts ordinary24275 and receives actual damage (controlPASS).1RED/1PASS32.34s;2m42compile; inherited unused simulator navigation import warning.

The native lower-threshold fixture changes only `Health.current` to19% of the real template's unchanged max; no spell, power, book, range, hit-chance or damage override. It is a health-state admission fixture, not a DPS measurement. Private snapshot SHA256 `52fa4d0ebec755f421b5678f0e83aa690a1d2422f33eae5208ff398848d4756c`; no canonical database writes.

## Implementation and pending proof

`auras/aura_state.rs` computes only these seven IDs, retaining exact15 logic and all otherNone results. Health reads are source-equivalent integer counts; thresholds truncate before strict comparisons. The actual cast pipeline consumes these values without a Ret exception. Existing Swiftmend unknown-state oracle uses4 after2 becomes computed; source15/exemption behavior remains unchanged. Remove the inherited unused simulator navigation alias rather than suppressing its warning.

Targeted GREEN and requested `cargo check -p server -p server-simulation --tests -j2` (once) are pending after implementation commit. Parent owns broad/final gate. Native Ret Hammer consumer stays separate until this shared feature lands; no whole-rotation/DPS claim.

## Artifacts

Engine `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/aura-state-health-2026-10-24/` contains `health-aura-state-source-v1.json` (full TC source paths/hashes), `health-aura-state-core-red-v1.log`, and `health-aura-state-native-red-v1.log`. Logs/proofs retain exact revision and scope; later source changes invalidate only intersecting evidence.
