# Retail 12.1 arenas

## Contract
- Skirmish2v2/3v3 queues form distinct, level-eligible rosters; invitations use match identity and existing accept/decline/entry/origin-return transport.
- Map selection uses pinned BattlemasterList12.1.0.69933, MapDB2, TDB1210 safe-locs/weights and verified local CASC WDTs. Never manufacture absent starts, spawns or policies.
- Reserved arena copies admit only their roster. Preparation starts after worldport readiness, lasts60s (TCa352b1fa Arena.cpp/Battleground.h), and prevents enemy attacks. Authored gates open at start; Shadow Sight enables at90s where authored (arena_scripts_generic.cpp).
- Opposing teams are hostile regardless of race. Elimination completes once one team has no living participants; TC gold-first ordering applies to simultaneous elimination. Retain results/scoreboard until leave, return to each participant's captured origin.
- Rated2v2/3v3 is conditional on a sourced formula; pinned ArenaTeam.cpp is accepted reference, not certified Blizzard Retail policy. No queue-only wins, payouts or fabricated season caps. Persist actual played-match rating only.
- Solo Shuffle requires authentic role/round policy as well as six-player DB2 rows. Dampening requires sourced runtime policy; spell existence alone cannot supply timing.

## Data installation
Lead runs scripts/import-arenas.py against the intended worldDB with --wdt-dir pointing at the local-CASC WDT receipts. Source CSV: scripts/reference/arena-12.1.0.69933/BattlemasterList.csv. Runtime refuses startup without arena_map. Canonical imports remain lead-owned.

## Current proof boundary
Skirmish queue pairing and readiness/preparation/elimination tested behaviorally. Reserved-copy admission regression reproduced; entry integration underway. Gates, enemy combat, authored orbs, native frames, rated/Shuffle and both-skin live proof are not yet complete. Full ledger: /home/osso/.worktrees/handoff-arena-coverage.md.
