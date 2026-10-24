# Native WDP enabler producer

Approved2026-10-24 policy: `SIMC_WW_WDP_ENABLER_PRODUCER`. Scope: actual RSK107428 or FoF113656 root landing while both native cooldowns remain>0 applies native196742 via generic aura path. Selected WDP owner's source buff eligibility only. Duration comes from CURRENT DB2 (1000ms); no152175 special case or native CasterAuraSpell gate change.

## Authority / absence

Pinned TC a352b1fa spell_monk.cpp has no Whirling Dragon Punch/152175/196742 binding. CURRENT native152175 requires CasterAuraSpell196742;196742 effect0 aura226 has no TriggerSpell. TDB spell_proc has no152175/196742 rows. Existingf2ST/3T receipts have no196742 uptime or152175casts. Raw read-only data/receipt packet: engine `ww-parity-2026-10-23/wdp-native-enabler-source-packet.json`.

SimC76f8118 sc_monk.cpp1138 (RSK execute),1826 (FoF execute),4552–4572 (WDP buff) supplies missing producer/overlapping-CD eligibility. Buff creation at6416–6417 is conditional on selected WDP talent. `SIMC_WW_WDP_GRACE_DURATION_DIFF`: SimC's manually measured1500ms grace plus minimum remaining cooldown differs from DB2 DurationIndex36/1000ms. DB2 wins: apply its generic1000ms aura only, no copied1500 or extended CD-derived timer.

## RED/GREEN scope

Ignored `literal_ww_native_selected_wdp_earns_its_native_cooldown_window` runs the actual selected269 literal fixture, requires real native RSK/FoF CastSuccess and overlapping native cooldowns, then asserts earned owned196742 duration1000. No aura/cast/power/cooldown grants. Atd174 zero-warning -j2 build555.29s; initial runtime fails17.92s before producer assertion because merged native TTD now requires explicit FixtureTarget lifetime/role. This is setup failure, not RED. Fixture now installs the same declared lifetimes as the measurement caller before decisions; no cast/aura/power/CD changes. Genuine runtime RED still required before producer implementation. Damage152175/children/recipients are outside this enabler-only change; numerical band remains open. Fresh CURRENT-backup EACH300s ST/3T required after qualification.
