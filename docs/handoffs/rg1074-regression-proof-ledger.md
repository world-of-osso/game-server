# rg1074 native resource/periodic regressions

Parent gate at661a89dec (`gate-rg1074-srv.log`, integrated header1c2973180) passed check/fmt but both listed boundaries also failed in isolation. Parent2026-10-24 requests fixes on a tip merged with masterdc3f30ca7. Exact masterdc3f30ca700b12c6cb977dc6c6b9a436676b582a merged as810a2b1e6, preserving both sides of four additive documentation conflicts; no Rust conflicts.

## Mana UDP fixture

Retained actual gate RED: `networking::tests::regen::client_sees_a_damaged_players_health_and_mana_regenerate_to_exactly_full` panics at `powers.rs666`, `native Mana power row`, inside the real UDP process/tick boundary. The minimal networking server initialized an empty PowerTypeTable even though its player already has base_mana1000. The shared native regeneration source now requires the real Mana row.

101459804 loads the complete real PowerTypeTable through the normal read-only native DB loader in that fixture, before registering the same tick systems. No synthetic power row, guessed regen amount, production fallback or panic suppression. The existing test still must observe replicated health/Mana progressing and exact final fullness. Exact original UDP regression passes at395ad19d1 in8.52s; native Mana progress/replication and exact health fullness remain asserted.

## Echo triggered refresh provenance

Retained actual gate RED: `holy_echo_of_light_native90_reheal_preserves_unpaid_pool_and_tick_phase` gets0 heals instead of1 before its original tick deadline (`echo_of_light.rs382`). Its oracle is correct.

Pinned TC a352b1fa:
- `SpellDefines.h291`: `TRIGGERED_DONT_RESET_PERIODIC_TIMER=0x00020000`.
- `SpellDefines.h293`: boolean-triggered `TRIGGERED_FULL_MASK=0x0007FFFF`, including that flag.
- `Spell.cpp3236`: ordinary reset only when native stack count<2 AND that flag is absent.
- `SpellAuras.cpp976–991`: native Pandemic suppresses reset separately.

9bb's generic refresh correction checked stacking/Pandemic but omitted per-cast FULL_MASK provenance. That incorrectly reset Echo's real triggered child, and the changed triggered Languish oracle encoded the same error. Preserve ordinary Barbed reset and unchanged native per-tick/carry amounts; do not revert to tooltip totals or add an Echo exception.

The fix carries `cast_dont_reset_periodic_timer` only on existing per-cast SpellInfo snapshots (`serde(skip)`), set by both scripted triggered loading and boolean creature FULL_MASK admission. Generic refresh uses it beside stacking/Pandemic. Normal DB definitions/default snapshots remain false. Languish's original-phase assertion is restored and exercises both existing scripted-trigger and boolean creature-trigger paths; unflagged Renew remains independent. Existing native ordinary Barbed tests retain their source-backed reset contract.

## Proof status and limits

395ad19d1 targeted verification **13PASS** on one compiler-JSON-selected `server-3270869ea8fa4b0c` ELF: exact UDP1/8.52s, exact Echo1/14.44s, generic rolling2/10.33s (scripted and bool FULL_MASK paths in the first case), ordinary Barbed/dummy20 controls9/15.92s. Compilation43m04s after the master/split merge, all compiler JSON messages empty; no warnings hidden. Source/ELF SHA256 `13134f09638cb47c4a8ddab107368aa29a663c7798376674f3c969e52849ea0e`; read-only private world SHA256 `0297adfd466e85ce812f7821aa3788823adcf06867a9bd6999207bad3a75d83a` unchanged before/after.

Diagnostics retain `rg1074-fix-395ad19d1-{compile,udp-regen,echo-refresh,generic-rolling,barbed-dummy20}.log`, executable receipt and summary JSON plus command/environment/revision entries in `proof-ledger.jsonl`. Docs-only follow-ups do not invalidate that code scope. No rerun of the broad parent gate here. Parent owns final integration. Other gate failures outside these two are not claimed fixed.

Howl actual-summon→Cry is separately qualified atc7c33ace7: native3GREEN17.84s (clean12m52s), qualified ELF SHA256c41d6c344b34b51a6f4e05bb8d14fc57cd00be2a2b085532cf2dd38f35c4f428, frozen private world0297. Its root/KC/cycle0-KC geometry/admission boundary remains open, with no teleport or disabled LOS. Cry result preserved before master merge; no claim that pre-merge proof automatically covers later source changes.
