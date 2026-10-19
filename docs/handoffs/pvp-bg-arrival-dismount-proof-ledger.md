# Ordinary battleground arrival dismount

## Scope

Successful WorldPortAck into an accepted/current ordinary battleground copy removes MOUNTED78 through normal remove_auras/mount on_removed callbacks. Mount aura, Mounted display and capability speed aura leave; unrelated aura remains. Queue, pending/declined invitation, refused entry and ordinary non-BG transfer retain mount. No early removal during successful port initiation. Arena unchanged; exclude remount/zone restrictions, pet policy, flight restoration, new flags/wire/rewards/penalties/live.

TC a352b1fa Battleground.cpp1038 AddPlayer RemoveAurasByType(MOUNTED); BattleGroundHandler.cpp407–408 calls AddPlayer only after WorldPortAck. Pinned cached hashes: engine `data/diagnostics/pvp-bg-arrival-dismount-2026-10-19/source-receipt.json`.

## Evidence

- Base `fe734dc507a6291439a04c47598faba408fdaf11`, after lr684 landing and branch cleanup.
- Ignored RED `f0c3527214e2dab2debf833548c8637c4f084555`, pushed first. Native `bg_arrival_mount_ -- --include-ignored --nocapture`: **2 genuine failures / 4 controls PASS**. Concrete/Random real accepted+ACK actors retain MOUNTED78. Native source32235 mounts129/display17697 at riding75 and creates speed86457 through normal application callbacks (not a physical cast proof); unrelated71041 preserved. Queue/no-match, actual invitation decline, actual refused entry and Stockade trigger/ACK controls retain mount. No prerequisite failures.
- Production `04aa1541f` reuses read_player_battleground current-copy/accepted-membership authority at successful transfer arrival and removes mount-type auras normally. No new tracking state. Master already current at integration; tests enabled in `f11674f2acca85414462f14294a307bd539ea86d`.
- GREEN on `f11674f2a`: **68 PASS / 0 FAIL / 0 ignored**, one native invocation of `maps::tests::battleground_nodes::deserter:: -- --nocapture` (six new +62 retained). Concrete/Random port initiation retains source mount/speed/display; actual ACK removes all three through native cleanup and preserves same unrelated71041 aura instance. All four mount-retention controls and retained arena/death/combat/admission cases PASS.
- Changed Rust formatted; manual changed-line readability clear; whitespace diff clear. Later docs-only commits do not invalidate Rust proof. Lead broad/check/fmt gate/landing pending.

Full commands/logs/final ledger under engine `data/diagnostics/pvp-bg-arrival-dismount-2026-10-19/`. CPU-only native-channel/aura-callback proof; no rendered/deployed claim. Lead owns broad/check/fmt acceptance.
