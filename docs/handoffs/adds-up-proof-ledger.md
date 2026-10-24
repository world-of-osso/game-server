# Native add-event up predicate

## Scope

Main requested a small RED-first follow-up to217f38d8f/d07db700c (bt1082). Both source revisions lack `up` in fixture_add_value; they do not cover this operand. Branch arms-adds-up descends217 through a29841a3f, but only this scoped test/behavior/docs delta is offered for integration. Proc runtime implementation remains parked; no proc source/data changes belong to this slice.

Pinned SimC76f8118 raid_event.cpp3068–3077 returns0 for an absent event's ordinary properties.3097–3108 returns1 when any matching event is up, otherwise0.2440–2443 reads the event's is_up state (asserted against its duration event). Native FixtureAdds already records actual active-wave expiration, so no new event model, enemy-count approximation, movement alias, or dungeon timing estimate is needed.

## Required proof

Five focused NativeFacts tests: absent0; configured/waiting0 despite exists/count; active and respawned waves1; exact expiration/after expiration/retired0; existing six properties and unknown-property rejection unchanged. Concrete native World/Time/FixtureAdds state reaches the public fact reader; no native spell catalog or class-specific fixture is needed for this state-reader boundary. Existing lifecycle integration remains separately identified in the217 ledger.

Test-only RED commit pending actual execution. Build/setup failures and zero tests are not RED. Record exact revision/profile/command/log/binary and retain a1PASS control alongside expected4FAIL before implementation. No activity behavior has changed yet.
