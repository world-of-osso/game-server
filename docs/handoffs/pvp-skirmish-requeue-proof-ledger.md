# Skirmish Queue Again — CPU proof ledger

[Contract](../specs/pvp-progression.md). Policy `SKIRMISH_REQUEUE_LEAVE_THEN_QUEUE_SAME_MODE`; source gap `SKIRMISH_REQUEUE_TRANSITION_UNQUALIFIED`. Evidence: canonical engine `data/diagnostics/pvp-skirmish-requeue-2026-10-19/`; Retail source receipts in sibling `pvp-skirmish-requeue-source-2026-10-19/`.

## Proof ledger

|Revision / command receipt|Scope|Result|Validity|
|---|---|---|---|
|fb2c09653 / server-red-command.json, server-red.stdout.log, server-red.stderr.log|`skirmish_requeue_ --include-ignored --test-threads2`, protocol76b415e|9 genuine failures,0 setup failures;36.00s|Before production. Every fixture reached its asserted Ended/Waiting/InProgress precondition; the request then received no response|
|09589fe42 / server-green-command.json, server-green.stdout.log, server-green.stderr.log|Same9 cases|9PASS/0FAIL,61.44s; no compiler warnings|Production768afeef1. 09589fe42 corrects one pre-ACK assertion the RED never reached: begin_transfer removes InMap, so the player is in no copy before ACK|
|659277ee7 / server-retained-command.json, server-retained.stdout.log, server-retained.stderr.log|Filters `skirmish_requeue_ arena solo_shuffle lfg_pvp_join deserter::port::`, no `--include-ignored`|131PASS/0FAIL/1ignored,284.71s; no compiler warnings|Enabled9 run as ordinary tests; the ignored case is the pre-existing explicit collision microbenchmark. Covers the extracted arena queue path and WorldPortAck completion for arena/Shuffle/LFG/port controls|

Lifecycle cases run the real 2v2/3v3 channel match to Ended, then: stale identity refused in place; one request per player plus a same-frame duplicate yields one return and one refusal; before ACK there is no queue and no copy; each ACK queues the played mode at the captured origin; the last ACK forms a fresh unaccepted invitation of the same mode. A level drop after return makes the ACK produce the ordinary queue error with the player left at origin and nothing queued. Rated2v2, Solo Shuffle, Warsong (Ended), unaccepted, Waiting and InProgress requests are refused without transfer, membership or queue change.

CPU-only: no UDP, rendered client, live server or deployment evidence. Lead final integration/check/fmt remain pending; no duplicate broad gates.
