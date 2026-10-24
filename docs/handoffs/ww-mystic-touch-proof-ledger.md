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

## Open qualification

Runtime RED first. Producer only after observed failure at the real native hit boundary. First-impact versus later-hit multiplier ordering and unrelated same-Monk item/proc damage still require exact native controls; the unrelated-owner case does not claim those. No producer or DPS parity claim follows from staging this packet.
