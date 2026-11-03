# Preservation: recorded Altar pull

The replay uses WCL `pPFDbkfQK7MACtmY` fight 41 pull 1, actor 221, first 75 seconds. Its party specs match the log: Brewmaster 268, Preservation 1468, Arcane 62 and two Elemental 262. Equipment and talent selections are native profiles, not exact copies of the logged characters. NPC lifetimes use first-to-last damage windows and are **INFERRED_WCL**. WCL deferred absorption receipts are removed from their source hit before the recorded Stagger and Time Dilation ticks are replayed. The specific attacker of pooled deferred damage is not observable and is represented by an engaged enemy.

Five native seeds passed the predeclared intake gate: each party member's normalized effective damage plus absorbs was within 10% of WCL in each 15-second window; no recorded events were dropped because a target was dead. The gate proves that the normalized input was delivered; it does not independently prove WCL's reported max-HP normalization or match all combat behavior. No party member died. Tank post-hit HP mean absolute difference across paired WCL observations was 19.55, 18.57, 19.10, 17.62 and 15.35 percentage points for seeds 1–5. HP difference is an outcome, not an intake gate.

| Measure | Native mean (95% seed CI) | WCL actor 221 |
|---|---:|---:|
| Effective healing per second | 78,777.74 (77,582.93–79,972.56) | 102,684.08 |
| Consumed absorbs per second | 7,534.85 (7,289.73–7,779.98) | 33,549.61 |
| Overheal | 43.16% (40.43–45.88%) | 55.80% |
| Dream Breath 355941 share | 21.74% (19.65–23.84%) | 4.75% |
| Reversion 355916 share | 13.00% (10.82–15.18%) | 6.42% |
| Time Dilation 357170 share | 0% | 7.98% |

Shares divide effective healing plus consumed absorbs by the same total. Overheal excludes absorbs and is overheal divided by effective plus overheal. WCL counts its actor's heal and absorbed events; native includes the healer and its owned summons. The two raw HPS values use different original max-HP scales and cannot be compared directly. The 95% intervals describe seed variation, not WCL population uncertainty. The per-seed gate, reports, binary/input hashes and original event extracts are under `/home/osso/.worktrees/healer-wcl-preservation-evidence/` and `/home/osso/.worktrees/healer-multi-enemy-evidence-v2/`.
