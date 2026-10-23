# Restoration emergency direct rescue

## Scope

Project-authored Restoration105 decision policy, not Retail APL or new spell mechanics. An emergency patient must receive ready direct Regrowth before routine delayed HoTs when native instant rescue is unavailable. Existing ready Swiftmend and Nature's Swiftness retain priority. No Holy, shared ranking, simulator, capture/replay, benchmark, spell metadata, resource or database changes.

Evidence: `game-engine/data/diagnostics/healer-ai-2026-10-21/policy-diagnosis/`.

## Diagnosis and native proof

Historical Azure1bf739d7a seeds4/10 show actual tank deaths with nearby active healer at99.88%/100%mana. This motivated a native decision counterexample, not a causal performance conclusion. Historical Holy failures are not current route evidence: a later af3aa374c Henry Stockade run clears all3 bosses with0 deaths. Read-only details: `initial-readonly.md`.

| Revision / boundary | Result | Evidence |
|---|---|---|
|2b4d8ca55 native32 critical tank after ordinary NS consumption and Swiftmend cooldown | Genuine RED:1FAIL6.92s, actual health784→868 with no direct8936Heal; all prerequisites pass |`emergency-native-red.log` |
|2e58adb3a production priority | Emergency direct rescue GREEN; existing NS/low-mana Swiftmend/stable Rejuvenation/Soul-Regrowth controls GREEN,5PASS. New Swiftmend control alone fails |`emergency-five-green-swiftmend-prerequisite-failure.log` |
|9307a0440 new Swiftmend control prerequisite |1PASS21.91s after ordinary Rejuvenation and explicit state15 assertion; previous five proofs remain valid against unchanged production |`swift-control-green.log` |
|9307a0440 scoped helper check |EXIT0,1m39s,no warnings |`emergency-check.log` |

The first new instant-rescue control incorrectly used Lifebloom. Native computed state15 only includes Regrowth/Rejuvenation/Wild Growth family masks; replacing its new setup with ordinary Rejuvenation corrects the prerequisite, not production behavior or an existing expectation. No existing test expectation changed.

## Root and correction

Shared urgency ranking prefers shorter cast time before effective healing, and estimates full-duration HoT output. Restoration's routine maintenance accepted the leading owned candidate even in emergencies. The narrow specialization correction calls ready emergency Regrowth after native instant-rescue choices and before routine upkeep; shared ranking and all native casts remain unchanged.

## Remaining gates and limits

- Master8290f38e0 integrated cleanly at5b9ff82e110ecd62f82c4f3220f5c56a47d2e1ec. All6 targeted native tests PASS11.95s; helper checkEXIT0/10.38s/no warnings; format PASS. Receipts: `postmaster-native-green.log`, `postmaster-check.log`, `postmaster-format.log`. Manual changed-Rust readability: no violations.
- Four S/T source hashes match before/after; frozen SQLite remains SHA2566b8b5e96b2a1f0caac05df74abce559516cacae9279d85e669d97edfd0dd7f2c. `postmaster-inputs.json` records scope: helper native roots alias checkout, not an independent-copy attestation. Lead integration gate remains separate.
- Agent14737's GREEN healer JSON/MD benchmark is the required effectiveness measurement; not yet available here. No survival, HPS, mana-efficiency, route-reliability or overall-quality improvement claimed.
- Holy decision diagnosis continues separately; no Holy production changes in this slice.
