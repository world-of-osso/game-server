# Native periodic amounts and rolling refresh

## Authority and source

2026-10-24 parent instruction: DB2/TC override tooltip arithmetic; reproduce native per-tick and unpaid-carry failures, fix generic computation, audit every tooltip normalization in `refresh.rs`. No SimC coefficient matching or Barbed-specific replacement.

Pinned TC a352b1fa:
- `SpellAuraEffects.cpp832–843`: fresh effect amount plus same-caster/spell/effect unpaid old amount divided across new total ticks; no fresh-total normalization.
- `SpellAuraEffects.cpp932–947`: duration/period, plus native extra-initial payment, determines total; effect amount remains per tick.
- `Spell.cpp3236–3244`: ordinary nonstacking recasts reset periodic timers.
- `SpellAuras.cpp976–991`: native Attr13 `PERIODIC_REFRESH_EXTENDS_DURATION` suppresses reset. Stacking also preserves time-to-tick.
- `spell_hunter.cpp585`:217200 participates in Latent Poison; no fixed-five damage script.

Native70448 selected253 inputs:217200 effect0 AP coefficient0.856999993/period2000ms; actual selected duration14000ms and extra-initial flag produce8 payments. Dummy effect1=5 and tooltip `$s1*$s2` do not override those executable inputs. Pinned SimC76f8118 independently uses full effect amount but is not authority for this fix; its refresh budget differs from TC.

## Frozen inputs and proof

Diagnostics root: `game-engine/data/diagnostics/dps-parity-fix-2026-10-21/`. Read-only private current-runtime class-closure world SHA256 `cd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6`; source canonical digest rechecked unchanged before experiment. Existing native253 profile/traits, genuine Dwarf Player, ordinary legal casts/GCD, native charge count and seed70448. No assigned stats/resources/timers, forced crit, fake trait selection, coefficient edits or canonical DB writes.

| Revision | Scope | Result |
|---|---|---|
|d9ad8a360|Complete one-cast and legal-overlap native lifetime diagnostics|2PASS23.19s; first total16997.574, two-cast33995.150. This disproved loss of the existing fixed-five budget, not correctness of that budget.|
|198697bd9|Same diagnostics plus DB2 per-tick and TC refresh behavioral assertions|2PASS/2FAIL21.52s; actual fresh2124.6968 vs source3399.5146, refresh8 observed payments vs one paid old plus8 new. Genuine gameplay RED; compile53.53s clean.|
|9bb480581|Generic production fix, same native4|4GREEN27.34s; compile12m17s. Fresh8×3399.5146=27196.1172; legal refresh first3399.5146 plus8×6374.0898=54392.2334. Actual refreshed payments1.55,3.55,…15.55 reach expiry once.|

Qualified9bb ELF SHA256 `3715bbd0feb2b6ecc1c3932ba90edf70d80b2df3e8f6e462611706595afca971`; native world digest unchanged. Normalized predecessor577 is preserved separately with SHA256 `8a38b28ef072e99e127231a9c5914a6e3717d8ba32fe6c7a031fe9d46ece4b40` for matched measurement, not inferred from an artifact filename.

Each phase has separate `barbed-budget-<phase>-<revision>-{compile.log,native.log,executable.json,receipts/}` plus exact command/environment entries in `proof-ledger.jsonl`. Receipt files preserve source factors, actual native damage inputs, legal cast times and full aura timelines through actual expiry. These are mechanics tests, not whole-DPS certification.

## Fix and normalization audit

`refresh.rs` contained exactly one tooltip-derived normalization: `normalize_barbed_damage`, selected only for217200. Deleted function and spell-ID branch; fresh DB2 effect amounts remain untouched. Other named branches accumulate Ignore Pain using its sourced health cap, or carry sourced Rip/Rake/Shield Block/SoTR durations; none divides periodic damage by tooltip counts. Those contracts remain intact.

Generic refresh prepares native time-to-tick from stacking/Pandemic inputs, otherwise uses the new application schedule. `refresh_from` installs that prepared effect state. One shared rolling pass carries only unpaid old amounts over the new schedule. No new public API, formula, estimator or Barbed executor. Existing fractional stored amounts/payment rounding remain unchanged.

Legacy dummy20 and Languish assertions were corrected to the sourced contract after real Player RED; they are controls, not the original RED. Their focused server-lib gate, Echo/335467/Pandemic controls and broad integration remain parent-owned pending proof. Earlier batch20 fixed-total inference/numeric-dummy closure is withdrawn; dummy5 gets no invented replacement action.
