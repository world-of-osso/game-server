# Shared TDB spell_proc values / TC dispatch

## Scope / base

Main authorized the full pinned18-column row, not only the initial illustrative list: SpellId,SchoolMask,SpellFamilyName,SpellFamilyMask0–3,ProcFlags,ProcFlags2,SpellTypeMask,SpellPhaseMask,HitMask,AttributesMask,DisableEffectsMask,ProcsPerMinute,Chance,Cooldown,Charges. Use the existing importer/table/provenance; no parallel parser or presence-only fallback after replacement. Base7701fc1b3 contains import87b0d684c/provenance682b31e1c, gatingft1083; rebase onto master when landed. Main publishes canonical only after code lands; this work mutates private copies only.

Audit: engine data/diagnostics/arms-parity-2026-10-23/tdb-spell-proc-source-audit.json SHA4c7f2019aedc23baab7432fc3b639573ab854da23b433ecb393a9def77f28704;1291effective positive-ID rows,1283base plus7ordered updates, eight pinned source hashes. TC a352b1fa SpellMgr.cpp1497–1653 loads explicit values/defaults before generated entries;524–593 filters event/school/family/type/phase/hit. SpellAuras.cpp1981–2010 applies weapon PPM, DB2 RPPM precedence, chance mods and level reduction. Explicit values must override DB2-derived masks, not union with them.

## Strict independent RED (before Ret dependencies)

Native BodyAndSoul64129 has Aura42→65081,100% native chance, help-spell flag0x4000, and all DB2 effect masks0. Audited TDB row: family6,mask0=1,mask2=0x80000,type4(no-damage/heal),phase2,HitMask3. Native Shield17 family mask0=1 is permitted. Native FlashHeal2061 mask0=0x800/mask3=0x40000000 and healing type must not trigger65081. Real Player casts and200 native frames observe whether the actual speed aura appeared, including before expiry; no injected spell rows/chance or fallback casts.

Two ignored test cases added; runtime RED pending. Existing native friendly-heal proc36488→36483 and generated FranticMomentum391875 are unrelated candidate controls; exact executed controls will be recorded. No production value import/dispatch implementation yet.

## Required later Ret RED

Do not import item_sets/Ret input policy piecemeal: main ownsrt1086/d38586b3e. After it lands merge Ret master+770, then add exactc26921958/cce395359 tests/fixture. NativeGarrick209057/class2/spec70/4setpieces, passive383344/no preexisting383346, single Blade184575 or Judgment20271 cast,200realframes, no grants/overrides/AA. Audited383344 family10/mask3=0x40000000/type1/phase2 must admit Blade and reject Judgment; peer observed7319wrong Judgment proc atc269. Source private DB52fa4d0ebec755f421b5678f0e83aa690a1d2422f33eae5208ff398848d4756c has no TDB table: import only a separate copy and record hashes. This pair is second RED, not waived by independent GREEN.

## Completion gate

Strict behavioral/importer RED→GREEN; all18 persisted values and TC zero defaults round-trip; native event/family/type/phase/hit/attribute/effect-disable/chance/PPM/recovery/charges behavior and retained unrelated controls; exact Ret pair after dependency landing. Preserve proof revision/binary/data identity, no setup/zero-test RED claims, no canonical write or DPS claim. Main owns broad integration/publication. Frozen expected-lengtha940 and persistent-RED2bc stay independent.
