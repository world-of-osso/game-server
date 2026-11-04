# Matched Holy Priest and Restoration Shaman pulls

The 75-second native replays use each WCL actor's CombatantInfo talents and gear, inferred opening auras and their observed removals, the recorded hostile intake, and first-to-last-damage enemy intervals. Their race remains unmatched. The per-recipient intake gate passes in every seed up to, but excluding, the first fatal hit. Deaths are outcomes, not intake-gate failures.

- Holy Priest: `BCX8v3VPd6pnyzM4` fight 1 pull 1, report start 2026-11-01 UTC.
- Restoration Shaman: `mHcDYRVyprndwjFC` fight 2 pull 2, report start 2026-10-31 UTC.

The v3 proof uses integration source `cfe4c34d0b`, which combines the landed changes below with the tested Guardian Elune's Favored and Holy Light's Resurgence patches. It is not a paired estimate of their effects: new proc rolls can alter the random stream. Input, binary and snapshot hashes, logs, per-hit health, casts, healing and gate reports are in `game-engine/data/diagnostics/healer-priest-shaman-2026-11-03/` (`*-v3-*`, `*-v3-metadata.json`).

| Seed | Priest first native death | Shaman first native death |
|---|---|---|
| 1 | Healer, 53.95 s | None |
| 2 | Healer, 54.25 s | Affliction DPS, 52.30 s |
| 3 | Healer, 53.95 s | Guardian tank, 35.05 s |
| 4 | None | None |
| 5 | None | None |

## Fixes and remaining gaps

- The critical hit reader, opening-aura removal and refresh handling, replay-only release opt-out, follower form exit, Affliction Siphon Life, and Holy Light's Resurgence fixes are on master: `b815230603`, `8e9328e3fe`, `57ace82579`, `f5eea9fd69`, `2efd813365`, `264e3ab6f9`, and `18244b7761`. Guardian Elune's Favored `2700b6c87e` is behavior-tested and included in v3; confirm landing separately.
- **Priest first HP gap:** seed 1 first differs by more than 10 percentage points at 14.216s, when native is 94.82% and WCL 79.54%; native absorbs 5.18% of the hit and WCL absorbs none. Native first falls more than 10 points below WCL at 22.219s: 77.92% versus 91.50%. WCL Soulcoil Barrier 1307578 absorbs 5.21% on that hit; native has no remaining barrier on that hit. Earlier HP already differs. The per-spell receipt, absorb and cast ledger is `v3-death-attribution.json`; native and WCL amounts retain their respective HP units, and Priest healer/self-heal lists overlap.
- **Priest timing gaps:** native Soulcoiler Ritual Vessel 1291894 starts at 0.10s versus WCL 17.669s; native Divine Hymn 64843 starts at 27.75s versus WCL 50.846s. WCL uses selected Desperate Prayer 19236 at 28.606s; the native Holy APL has no action for it. Do not infer general cooldown timing from this pull. WCL Holy Nova 281265 receipts follow 132157 casts, which this native APL does not select. Rune heals 1263002 and 1287665 have native item handling but are absent on this recipient in seed 1.
- **Desperate Prayer policy gap:** among 71 casts by seven Holy Priests in current-build +18 Altar reports `f3d81arQWb4cqvZL/15`, `QhL7CHrwabAFxBdv/58`, `Vfzd1c3gaWRhNAL8/5`, `jacKzYMZnLgC2W3Q/2`, `y3m8WgrY2Ttpxj7K/1`, `xhN4jtgKmTQP3aVB/16` and `qNR6B2twCLXQrK1m/27` (2026-10-21 through 2026-11-01), 54 have a valid preceding HP observation within 500ms. Their HP p25/median/p75 is 68.35%/85.55%/100%; actual HP damage in the preceding three seconds is 16.89%/29.66%/46.26%, with seven taking none. Cast-event HP alone is unreliable: 48 of 71 report more than 100% while HP and maximum HP straddle the defensive's max-health change. This sample does not establish a tight threshold. `priest-dp-analysis-precast.json` records event ages and sources.
- **Shaman target gaps:** seed 2's Affliction actor 2 first differs by more than 10 points at 14.862s: native 81.82%, WCL 92.63%. WCL Soul Leech 108366 absorbs part of that hit; the native replay has no 108366 generation. Siphon Life 453000 is now implemented and appears in v3. Fel Synergy 389372 and its accumulating Soul Leech/Demon Skin shield producer remain unimplemented: WCL shows the shield removed at 16.863s and restored with Fel Synergy at 17.247s despite intervening Agony ticks. The source does not establish whether batching is general or Demon Skin-specific. DB2 describes separate owner/pet percentages; do not guess the missing timing or implement recharge in isolation from shield state.
- **Guardian policy gap:** the selected 22842 spell is implemented, but the native Guardian APL does not cast it. Eight current-build +18 Altar reports provide 377 casts, with logged HP p25/median/p75 79.49%/94.44%/100%; cast HP can reflect Fount's max-health change and WCL does not expose the remaining charges. The sample does not establish a health/charge threshold. See [Guardian replay](104.md#frenzied-regeneration-policy-gap) for report IDs.

**Evidence labels:** WCL observations are limited to the dated reports above; policy thresholds remain unqualified. The detailed Priest attribution is `priest-v3-attribution.md`, and individual WCL event streams, matched CombatantInfo and statistical samples are beside it in the diagnostics directory.
