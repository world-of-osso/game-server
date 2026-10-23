# SimC spell oracle

Single-spell formula receipts from public SimC `76f8118cf3f7f41dad20708e8328e1f4ab2deebc`. Clean room: no decompiled client code. Short probes do **not** certify native DPS parity.

## Run

```text
python3 scripts/dps_parity/simc_spell_oracle.py --profile Arms --action auto_attack --action mortal_strike --output /var/tmp/osso/mortal_strike.json
python3 -m unittest discover -s scripts/dps_parity -p 'test_simc_spell_oracle.py'
python3 scripts/dps_parity/simc_spell_oracle_batch.py --spec 70
```

Repeated actions retain APL priority: enabling cooldowns first, spell next, resource generators last. `--targets` defaults to1, `--time` to30s. Names resolve within pinned MID2; explicit paths permit the labelled MID1 Devastation reference. No automatic tier substitution. Original action lists are stripped. Simulation options come from the profile's recorded single-target reference proof, including `optimal_raid=0` and every other non-default flag. `--reference-proof PATH` selects an exact proof; missing/conflicting matches fail rather than enable default raid buffs. Only targets, duration, iterations/threads, deterministic logging and output destinations are probe controls; fixed-time/vary-length, seed, target level, fight style and all other flags remain inherited. The proof records inherited options, explicit overrides and source-proof hash. `agent-run simc-oracle` uses `TMPDIR=/var/tmp/osso`; adjacent `.simc`, `.txt`, `.console.txt` and `.proof.json` retain exact argv, revision and hashes.

Corpus: `game-engine/data/diagnostics/simc-spell-oracle-2026-10-23/index.md`. Top-five selection combines equal name/ID pet channels and records original shares/actors/reference hashes. Frost251's receipt-v5 native-legality exclusion remains in force.

## Compare one native cast

Match actual **formula spell ID**, actor/pet, target count, direct/tick kind and buff state. Report-parent IDs can differ: Fists113656 prints child117418; Fire Breath357208 prints357209. `selection.observed_formula_actions` preserves actual IDs. Compare factors left to right:

| SimC printed factor | Native equivalent |
|---|---|
| `a_power`/`ap`, `s_power`/`sp` | Snapshot `native.attack_power` (or `native.ranged_attack_power`), `native.spell_power`; use pet stats for pet damage. |
| `a_mod`, `s_mod`, `base`, `weapon`, `w_mult`, `w_slot_mod`, `bonus_da` | Effective per-cast coefficients, base, rolled/normalized weapon term and bonus. **Absent from current aggregate native receipts**; owners must expose cast-boundary factors. `main_hand.min/max` are only a range. |
| `mult`, `da_mul`/`ta_mul`, `ply_mul`, `vs_mul`, `per_mul`, `tick_mult`, `rta_mult` | Native direct/tick, actor, persistent/school and rolling multipliers. Not currently printed by aggregate receipts. Compare source IDs/effect indices against native auras. |
| `crit`, `tgt_crit`, `crit_bonus` | `native.crit_pct / 100`; target crit and per-cast bonus need additional native evidence. Total crit multiplier is `1 + crit_bonus`. |
| `versatility` | `1 + native.versatility_pct / 100`, using equally timed snapshots. |
| `tgt_*_mul`, `target_armor` | Target vulnerability/mitigation and effective armor at impact; not caster `native.armor`. Aggregate native receipts lack these factors. |
| `armor_mitigation_fraction` | Explicit `reduced by X% from armor` divided by100. Compare native armor-only reduction, not total mitigation. Null means unprinted, not zero. |
| Final `actual_amount` | Native `CombatLogEvent.amount` for one matched impact. `actions[].average_hit` mixes results/states and cannot identify formula factors. |

No native instrumentation is added here. Existing field names come from `dps_parity/report.rs`; parity owners expose missing cast factors. Report the first differing factor, not a guessed channel-wide scaling correction. SimC70391/native70448 build differences remain separate evidence.

## Interpretation

`events` preserve raw lines, timestamps, factors and identities. Formula records may be individual AoE impacts/channel children, **not independent casts**. Result records include pre/post calculation stages: never sum them; early `mitigated_amount=0` is not final damage. Summary arrays are distinct observed values, not means.

Passive products itemise printed percentage sources, deduplicated by source/effect/value. They are attribution aids, **not extra multipliers applied atop `mult`**. `per_buff_stack`, `per_consumed_buff_stack`, `buff_present` and `mastery_scaled` remain conditional. Actor-wide modifiers carry `scope=actor` and no invented action ID. Unprinted factors are null; unsupported grammar stays in `unparsed_relevant_lines`; ambiguous result IDs stay null with candidates. Spell0 requires action/actor disambiguation. All observed children/procs remain, but co-occurrence does not prove causality.

`NOT_TRIGGERED_BY_MINIMAL_DRIVERS` means no matching formula observed in30s, not zero damage. Startup modifiers may still print. Failures retain actual errors/proof; no values are fabricated.

## Sources

- [CLI](../../../scripts/dps_parity/simc_spell_oracle.py), [batch driver map](../../../scripts/dps_parity/simc_spell_oracle_batch.py), [recorded tests](../../../scripts/dps_parity/test_simc_spell_oracle.py).
- [Native receipt fields](../../../crates/server/src/dungeon_simulator/dps_parity/report.rs).
- [Parity specification](../../specs/dps-parity.md).

## See Also

- [[simc-apl]] — native rotation binding is separate.
- [[character-stats]] — native stat production.
