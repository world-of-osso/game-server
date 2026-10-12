# Retail 12.1 arenas

## Contract
- Skirmish2v2/3v3 queues form distinct, level-eligible rosters; invitations use match identity and existing accept/decline/entry/origin-return transport.
- Map selection uses pinned BattlemasterList12.1.0.69933, MapDB2, TDB1210 safe-locs/weights and verified local CASC WDTs. Never manufacture absent starts, spawns or policies.
- Reserved arena copies admit only their roster. Preparation starts after worldport readiness, lasts60s (TCa352b1fa Arena.cpp/Battleground.h), and prevents enemy attacks. Authored gates open at start; Shadow Sight enables at90s where authored (arena_scripts_generic.cpp).
- Opposing teams are hostile regardless of race. Elimination completes once one team has no living participants; TC gold-first ordering applies to simultaneous elimination. Retain results/scoreboard until leave, return to each participant's captured origin.
- Rated2v2/3v3 is conditional on a sourced formula; pinned ArenaTeam.cpp is accepted reference, not certified Blizzard Retail policy. No queue-only wins, payouts or fabricated season caps. Persist actual played-match rating only.
- Solo Shuffle requires a complete reset/preparation and role/round policy as well as six-player DB2 rows. Historical Blizzard previews establish six rounds, healer/DPS permutations and first-kill outcome; later order/tank-role changes prevent claiming the preview is the complete 12.1 policy. Dampening requires sourced current timing/ramp policy; historical values or spell existence alone cannot establish it. See the client wiki source-policy audit.

## Data installation
Lead runs scripts/import-arenas.py against the intended worldDB with --wdt-dir pointing at the local-CASC WDT receipts and --promote for authored gate/orb objects. Promotion touches only arena object spawns and their templates. Source CSV: scripts/reference/arena-12.1.0.69933/BattlemasterList.csv. Runtime refuses startup without arena_map. Canonical imports remain lead-owned.

## Current proof boundary
Twelve targeted server cases cover skirmish pairing, source level20 floor, reserved-copy admission, source door/orb loading, preparation/gates, real same-race enemy attacks, TC arithmetic/window policy, and full four- and six-player skirmish/rated HostClient entry→32727 aura→90s physical34709 orb→HP-primed real kills→scored outcome/full resurrection→every exact origin return. Full-channel cases enter with a dead participant and prove source battlefield-port full resurrection before preparation. A real-channel alive-forfeit case proves departures end the match and return origins without fabricating deaths. Rated results persist personal rating/MMR only after the played outcome; old ratings with absent sourced MMR remain untrusted. Both-skin UI tests cover actions, enemy frames, rating and measured score display; rendered/UDP live proof remains pending. Shuffle role/round policy and dampening schedule remain source gaps. Full ledger: /home/osso/.worktrees/handoff-arena-coverage.md.
