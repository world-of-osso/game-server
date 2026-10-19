# Ordinary BG completion health and primary power

## Scope

TC a352b1fa Battleground.cpp772–773 calls ResetAllPowers before CombatStopWithPets. Player.cpp1804–1827 fills health and primary Mana/Energy, zeros primary Rage/Runic/Lunar, leaves unlisted primary types and secondary pools unchanged. Reuse existing arena reset_resources, after dead-player resurrection and before combat stop/result credit. Only accepted actors still in current copy; unentered, departed and other-copy actors unchanged. Do not repeat reset on later Ended updates. No arena behavior change, new timer/flags/payout/wire/live.

Pinned hashes: engine `data/diagnostics/pvp-bg-completion-resources-2026-10-19/source-receipt.json`.

## Evidence

- Base `56106c9151327ace2009a78984aac34478b50bcd` (40252+).
- Ignored RED `7b8055dd3b2c4d3645187c48e78fe6ee1540e4ab`, pushed first. Initial invocation could not compile unrelated guild.online_zone against old frozenprotocolf9; preserved `prerequisite-red.*`, NOT behavioral RED. Lead approved refreshing owned frozen protocol to `b3a34fe35c764cc5e77086d85c19605bfad6448a`, current master carrying guild field; no protocol source edits.
- Actual native RED on same server `7b8055dd3`, protocolb3: `bg_end_resources_ -- --include-ignored --nocapture`: **6 genuine health40-vs100 failures / 1 native-departure control PASS**. Six primary cases (Mana/Energy/Rage/Runic/Lunar/Focus) reach real source-node terminal victory; winner/loser actors, current-copy/no-transfer and primary/secondary/replay assertions follow health reset. No fixture failure.
- Production promotes only existing reset_resources visibility and invokes it before combat_stop; no duplicated resource math. Prior cleanup living-unhealed expectations are deliberately superseded by approved full-health/primary-Mana completion behavior; fixture declares Mana primary explicitly.
- Production `8159b2062`, master merged as `c639f7e79`, tests enabled in `45572ca3c9789d863cf604111116f1171610f821` before proof.
- GREEN on server `45572ca3c`, protocolb3: **10 PASS / 0 FAIL / 0 ignored**, one native `maps::tests::battleground_nodes::end_cleanup:: -- --nocapture` invocation (seven new +three retained). Real source-node terminal objective: full health, primary Mana/Energy100, Rage/Runic/Lunar0, Focus40 unchanged, secondary HolyPower2 unchanged, non-primary Mana40 unchanged; both assigned teams, dead full resurrection, unentered/other-copy/native-departed exclusions, no transfer and no repeated refill on later Ended updates. Retained real timeout draw/current-copy/death cleanup passes.
- Exact retained arena primary-reset control pending capacity. Changed Rust formatted, manual changed-line readability clear, whitespace diff clear. Later docs-only commits do not invalidate Rust proof. Lead owns broad/check/fmt acceptance. CPU-only native objective/channel proof; no rendered/deployed claim.

Full argv/logs/protocol receipt/final ledger: engine `data/diagnostics/pvp-bg-completion-resources-2026-10-19/`.
