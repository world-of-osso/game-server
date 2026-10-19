# Ordinary BG completion: active permanent-pet combat stop

## Scope

TC a352b1fa Battleground.cpp773 CombatStopWithPets(true). After an accepted/current-copy owner's existing health/primary reset and combat stop, stop its currently summoned permanent pet through existing hunter_pet::summoned_pet/PetStable.summoned and combat_stop. Preserve same entity, health/resource; no heal/despawn. Unentered/departed/other-copy owners' pets unchanged. Exclude temporary guardians/all-controlled completeness, arena changes, pet resurrection/reset/rewards, new ownership flags/lifecycles/wire/live.

Source SHA receipt: engine `data/diagnostics/pvp-bg-completion-pets-2026-10-19/source-receipt.json`.

## Evidence

- Base `46d5aba82e8692f56f00218b412d4f92db2904a8`, protocol frozenb3a34fe.
- Ignored/pushed initial RED `f37b57b8c`: two genuine Warlock combat failures; two fixture/oracle failures (native tame despawns its original spawn through death/hiding rather than deleting its respawn entity; unentered/other-copy owners lacked faction input). Preserved `red-prerequisites-*`, not represented as complete behavioral RED.
- Corrected ignored/pushed `1081d3947cc401446212648dd3770fa8905da95b`: **3 genuine CombatStatus(true)-vs-false failures / 2 controls PASS**, native `bg_end_pets_ -- --include-ignored --nocapture`. Native source13481 Tame Wolf299 and688 Summon Imp416 producers install actual pet ownership/storage, normal start_attack plus direct damage refresh pet combat through actual source-node terminal victory. Separate unrostered targets ensure accepted-player cleanup cannot accidentally stop the pet. Living hunter/warlock and actual dead Warlock owner cases fail at pet combat. Native departed and other-copy/unentered controls PASS.
- Proof is native effect-producer/real damage/map-objective/channel integration, not physical spell-cast, pet-AI policy, UDP or rendering.
- Production `d13c98f99` reuses existing pet lookup and combat stop only (three lines); master current before GREEN. Tests enabled in `eed785593288fdddb5d9d186356e2f5cb760b518`.
- GREEN on `eed785593`, protocolb3: **15 PASS / 0 FAIL / 0 ignored**, one native `maps::tests::battleground_nodes::end_cleanup:: -- --nocapture` invocation (five new +ten retained). Hunter/Warlock active pets with living/dead owners stop combat/autoattack before result and retain same entity, injured health and primary resource; later updates remain stopped. Native-departed and other-copy/unentered owners' pets remain fighting. Existing owner health/primary reset, dead cleanup, draw and no-transfer/replay cases pass.
- Exact retained four-player arena channel lifecycle control pending capacity. Changed Rust formatted, manual changed-line readability clear, whitespace diff clear. Later docs-only commits do not invalidate Rust proof. Lead owns broad/check/fmt acceptance.

Full commands/logs/final ledger: engine `data/diagnostics/pvp-bg-completion-pets-2026-10-19/`.
