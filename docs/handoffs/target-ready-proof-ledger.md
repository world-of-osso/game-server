# Adapter-declared target readiness

Goal2026-10-24: generic per-action/per-candidate adapter veto before shared literal selection; no rewritten rows or native cast-admission changes.

Reporter15155 case: b584a6b2d `dungeon_simulator::dps_parity::ret_rotation::readiness_tests::dps_parity_ret_literal_native_idle_target_does_not_select_rebuke`. SimC76f8118 `sc_paladin.cpp1956–1962` requires the actual target casting debuff for Rebuke target_ready. Reporter's native log selects rebuke on idle31146 after actual KnownSpells/idle/AA controls (47.98s/exit101), engine `ret-parity-2026-10-23/ret-literal-rebuke-ready-red-v1.log`.

Imported exact b584 Ret module, pinned43-row source and four fixture files test-only to reproduce that unchanged test/function name. No300s Ret caller wiring or full Ret adapter qualification. Master5d39f490a/Ret Equipment merged asde8d2d609 with lifecycle/GCD/poll/composite/self-cast contexts and incoming native knowledge/resource probes retained; context aliases retain native pool classification too. No target-ready behavior implementation yet; current integrated native RED pending.
