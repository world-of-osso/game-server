# Ordinary BG Preparation44521 — CPU proof ledger

[Contract](../specs/pvp-progression.md#ordinary-battleground-preparation). Base2a1e1202a (landed bp736 clock/gates), protocol7f9bd7c. Evidence: canonical engine `data/diagnostics/pvp-bg-preparation-aura-2026-10-19/`; source/census receipts in sibling `pvp-bg-preparation-aura-source-2026-10-19/`.

## Approved scope

Actual Waiting ACK grants authored44521 once; natural start removes it; successful BG exit clears its BG-preparation role beside existing mount cleanup. Use normal aura application/removal; never the arena resource/cooldown reset path. Source: TC a352b1fa Battleground.cpp1054–1056/444 and the explicit bounded project exit contract.

Generic world interrupts are excluded: `AURA_INTERRUPT_LEAVE_ENTER_WORLD_SEMANTICS_UNQUALIFIED`. Read-only70448 census found28,057rows/28,050spell IDs with either raw flag; this is not evidence to strip all those auras on zoning, logout or persistence boundaries. No generic transfer implementation or new44521 effect handler.

## Proof ledger

|Revision / command receipt|Scope|Result|Validity|
|---|---|---|---|
|36bfc2ea0 / red-command.json, red.stdout.log, red.stderr.log|`bg_preparation_aura_ --include-ignored --test-threads2`|6 genuine failures (ACK2/start2/exit2),7 controls PASS;13cases/11.98s|Observed before production edits; no setup failures|

Native qualification applies the complete authored aura through AuraApplication: Fireball133 Mana cost20 at base2000 and SinisterStrike1752 Energy cost45 become zero; normal removal restores exact values. Native charge-plan affordability also changes with empty pools. This proves computed cost/admission, not physical cast execution.

Controls cover arena32727/no44521, pending decline, late InProgress arrival, refused combat exit, unrelated aura identity, unentered/other-copy actors and stable aura identity across repeated updates. Source baseline aura application in start/exit fixtures isolates those cleanup failures from the absent producer.

The RED command exceeded Pyrun's synchronous budget during a cold dependency build (19m24s). Existing detached job01a157bd-0d39-77eb-9f75-1be3ecc9d866 was observed to completion, never rerun for logs. No live runtime or UDP/render/deployment evidence.

Production consumer/spec committed after genuine RED. Targeted GREEN and Lead final integration/check/fmt remain pending; tests stay ignored until GREEN.
