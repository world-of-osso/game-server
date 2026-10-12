# Class completeness continuation 6

Verified: 2026-10-12. [Contract](specs/class-completeness.md), [prior batches](class-completeness2.md), [implementation](wiki/systems/spell-casting.md#level80-class-completeness).

## Native form admission — batch28

`spell_cast::shapeshift::check_cast` consumes native `SpellShapeshift` required/excluded masks as64-bit values. Exclusions precede allowed-form matches. Unmatched forms use `SpellShapeshiftForm.Flags`: Stance1 distinguishes stances from shifted forms; CanOnlyCastShapeshiftSpells0x400 restricts unmatched spells. Attr0NotShapeshifted0x10000 and Attr2AllowCasterForm0x80000 retain their distinct source meaning. Recipient aura275 bypasses this check only when its native family/effect mask matches; MiscValue is not interpreted as a spell ID. Missing form rows log and permit exactly as the pinned source, without invented flags. Existing triggered casts remain exempt. Appended wire failures distinguish forbidden and missing-required forms. Retained Retail UI files do not supply the relevant GlobalStrings, so native UI displays explicit source failure names; localized/form-specific strings remain a presentation gap.

Primary reference: TrinityCore a352b1fa `SpellInfo.cpp::CheckShapeshift` (retained copy lines2115–2166), `Spell.cpp::CheckCast` aura275 exception, `DBCEnums.h::SpellShapeshiftFormFlags`. Native inputs are DB2 12.1.0.69933/imported read-only world.db, not invented IDs or coefficients.

Proof: serverbaacfcda genuineRED3→0928c318GREEN3; protocolcc75a86; enginef9f310b5a. Server45cb8f3f integrates origin/mastere3a7d52d, resolving the destination conflict as `ground_destination` before `relocate_destination`. Expanded db430013 controls10/10 GREEN: real Shred damage and Regrowth heal; missing/forbidden/allowed forms; family/mask/recipient exclusions; native expiry/removal; high-word form35; Stance, AllowCasterForm and CanOnlyCast flags. Earlier private-field compile failure and omitted native Moonfire164812 fixture child were not feature REDs.

Census at db430013/sharedcc75a86: all7827 cases, exit0,590.95sec. **2981working /4664partial /182broken**, versus3000/4645/182 at e40eb857/shared80bec38. Thirteen memberships become working after275 implementation. Thirty-two previously working unshifted probe casts now correctly expose required-form prerequisites (Druid, Rogue and two Devourer roots); they remain conditional partials, not unsupported/broken spells or silently granted forms. Net−19 working is not a claim of whole-ability regression or completion. Detailed transitions: `data/class-completeness6-2026-10-12/transitions-batch28.json`.

Revalidated authentic missing-store inputs:37 IDs/170 memberships still have no canonical imported SpellName rows. Four Mage fixed-position IDs retain their explicit missing-source boundary; no positions invented. Representative13-class/26-PNG both-skin gate remains closed from prior receipts, not newly rerun. Full class/talent goal remains open; new native process proof and integration checks remain separate gates.

## Lead gate fixture diagnoses

Lead's three failures reproduced together before changes in `lead-gate-red.log`.

- Fists113656 now deals actual cone117418 damage. Diagnostic records19×261 damage and target health0; a subsequent Blackout Kick correctly rejects that dead target. Cooldown-sequence fixture now starts with100000HP and asserts both actual damage and a surviving target before its follow-up. No target validation or damage behavior weakened; Fists tick-count/whole-ability certification remains separate.
- Native Arcane137021s4 is60 points with−3 RealPointsPerLevel, SpellLevel10 and MaxLevel30. Pinned `SpellEffectInfo::CalcValue` clamps before subtracting spell/base level. The former level80 expectation800mana/s omitted that term; native bonus at30/80 is0. Test now covers10→60%,20→30%,30/80→0%, with actual in/out-of-combat regeneration. No invented regen coefficient or production change.
- Native Goblin Gumbo42753 and toy388088 use destination87. A request without a placement click correctly fails NoTarget. Type22 spellcasters now have the shared runtime; the obsolete unsupported-type expectation is replaced by no-click rejection and actual186472 object creation after a clicked destination and completed native cast. Missing381317 remains rejected with valid input, no silent success.

Server3e8ba9a3 repairs these inputs/expectations; bf1ead38 observes the native Gumbo cast completion. Targeted Monk/Arcane2/2 GREEN and toy1/1 GREEN. Broad lead-requested gate is tracked independently; no landable claim before it passes. Durable logs and census artifacts: slot-local `data/class-completeness6-2026-10-12/`. These test-only repairs do not invalidate batch28 runtime behavior evidence.
