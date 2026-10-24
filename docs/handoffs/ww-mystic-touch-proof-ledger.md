# Mystic Touch native producer — RED packet

Verified2026-10-23; no runtime RED or producer implementation claimed. Parent approved named `MONK_MYSTIC_TOUCH_SIMC_PRODUCER`, pinned SimC76f8118 monk/sc_monk.cpp438–455. Native learned passive8647 is distinct from produced enemy debuff113746. Require the actual damage actor's learned8647; no Monk-class or nearby-party fallback. Native113746 owns Aura87/5%/physical school1, enemy target6. No damage coefficient tuning or flat multiplier is introduced.

## Source and existing differential

Native8647 has passive Attr0=64, SkillLineAbility39570/SkillLine829/ClassMask512/AcquireMethod2/requiredlevel49, Dummy4/Trigger0. The aligned native TP→TP→FoF receipt has the learned passive but target multiplier1 versus comparator1.05. This observation is not yet this packet's asserted native RED. [Source and FoF alignment](ww-momentum-boost-source-ledger.md).

## Staged ignored controls

- Native level49 learned book, actual positive surviving-target Tiger Palm impact →113746 expected.
- Native level49 learned book, actual positive surviving-target white hit →113746 expected.
- Native level48 unlearned book, actual TP damage →no113746.
- Unrelated native owner damage while learned Monk is present but not attacking →no113746.

The new unengaged level helper separates native initialization from the old helper's owner attack; existing at_level callers keep their prior engagement and three-frame behavior. Tests grant no caster resources or target auras and use actual hit/health changes before producer assertions. All new source-view-dependent cases stay ignored. They will be compiled with the next already-authorized precombat build, never an extra WW-only compile.

## Pinned source timing — inference, not runtime qualification

`sc_monk.cpp438–455` calls `trigger_mystic_touch` before base impact, but `action.cpp1855–1914` snapshots the target and calculates direct amount before scheduling impact;4366–4367 snapshots `target_da_multiplier`. `action.hpp945–946` and `attack.cpp150–156` include target vulnerability in that multiplier; `player.cpp5885–5886` supplies Mystic Touch's native school-specific value. `player.cpp7067–7068` instead preactivates the debuff when the SimC override is enabled.

Inference: with the override disabled, the first applying direct hit uses its earlier snapshot; later hits can receive the new debuff. An oracle first-hit product1.05 may reflect the override, not producer timing. This does not authorize a prepull aura or flat5% multiplier. Exact native first/later-impact numerics remain unqualified.

## Open qualification

Runtime RED first. Producer only after observed failure at the real native hit boundary. First-impact versus later-hit multiplier ordering and unrelated same-Monk item/proc damage still require exact native controls; the unrelated-owner case does not claim those. No producer or DPS parity claim follows from staging this packet.
