# Feral shared Program surfaces

Parent priority2026-10-24: after target-ready/use_items native GREEN, reproduce reporter36fc81e6882f8ac10832a1f964bf6d062f430634's four exact engine-free cases, then implement shared Boolean target_if, floor(), pool_resource for_next and max_energy without class-specific row rewrites. Do not duplicate bt1082/45bae2933's min/max operators.

Imported unchanged `crates/server/src/feral_apl_reporter.rs` and `simc_apl/feral_reporter_tests.rs`. Harness imports the actual production Program directly, with no Bevy/database/native admission mock. Build `rustc --edition=2024 --test crates/server/src/feral_apl_reporter.rs -o <out>`; runtime filter `feral_reporter_`.

Exact cases: Boolean target_if chooses first refreshable candidate; floor sets remaining Convoke uses2; pool_for_next waits rather than dispatching its control row; max_energy refuses25-Energy Bite below extra-energy cap. Source `feral-literal-apl@121a`, engine `feral-parity-2026-10-23/literal-source/inventory.proof.json`/pinned SimC76f8118 Feral APL. Local source SHA manifest `ww-parity-2026-10-23/feral-shared-reporter/source-receipt.json`. Current RED pending. Engine-free proof does not qualify native action/fact adapters, resources or300s Feral parity.
