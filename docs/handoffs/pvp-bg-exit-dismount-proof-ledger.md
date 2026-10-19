# Ordinary BG exit dismount

## Scope

TC a352b1fa Battleground.cpp833–834 RemovePlayerAtLeave removes MOUNTED78. Successful ordinary BG exit (Waiting/InProgress/Ended) removes a mount acquired inside the match through existing remove_auras/on_removed cleanup, including Mounted display and capability speed; unrelated aura preserved. Failed combat leave, pending/declined invites and non-BG transfer retain mounts. Arena, mount availability/remount restrictions, prior-mount restoration, pets, rewards/new penalties/flags/wire/live excluded.

Source SHA receipt: engine `data/diagnostics/pvp-bg-exit-dismount-2026-10-19/source-receipt.json`.

## Evidence

- Base `398ada1308f73596b9d360a09fe178d0c93168b7`, protocolb3a34fe.
- Initial ignored REDs retained separately: b9b77/b5d2 gave four genuine early-exit mount failures plus two controls, but completed cases lacked object-use channel/producer. 48229 attempted extending minimal deserter harness with NpcInteractionPlugin and failed required faction-resource setup. Lead explicitly directed reverting that addition and using proven full native source-match harness only for completed cases. 07e122 then produced five genuine mount failures, but Random's existing ten-player fixture intentionally remained Waiting below Arathi8/team start minimum.
- Corrected ignored/pushed `89277d62bae409f76af34d9dc6aca2dabbeb64fa`: **6 genuine mount assertion failures / 2 controls PASS**, native `bg_exit_mount_ -- --include-ignored --nocapture`. Concrete/Random Waiting/InProgress/actual node-victory Ended all return through native Leave/ACK but retain mount78. No setup failure. Waiting/InProgress tests remain on minimal harness; completed tests use existing full helper, with new test-only20-player Random fixture variant (old10-player minimum-control untouched).
- Source32235 callback mounts129/display17697 with riding75 speed86457 after entry; unrelated71041 held. Proof is native application/channel/objective integration, not physical cast/UDP/rendered.
- Production shares existing mount-aura removal between arrival and successful non-arena leave; leave calls only after return initiation succeeds. No duplicate type filter/new tracking state. GREEN pending; Lead owns broad/check/fmt gate.

Full argv/logs (including all separately qualified prerequisite runs) and final ledger: engine `data/diagnostics/pvp-bg-exit-dismount-2026-10-19/`.
