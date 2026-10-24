# Shared use_items

Goal2026-10-24: select native equipped on-use items in pinned slot order, preserving native admission/cooldowns and original Action.options. Ret Equipment from landed master5d39f490a is integrated.

Pinned SimC76f8118 `engine/player/player.cpp10221–10480`: use_items proxies prefer trinket1,trinket2, then remaining slots; execute scans ready+target-ready and executes FIRST eligible proxy then breaks. It is not an immediate all-items batch. `slots=` restricts/reorders proxies; default check_existing1 excludes slots/named items explicitly covered by use_item (precombat named/effect uses excluded only under source conditions). No invented simultaneous use or shared-CD bypass.

Reported gap: Prepared records item:<slot> and handles only singular use_item; use_items silently selects nothing. Added ignored native fixture with declared real61034/193701 in the existing269profile BEFORE native spawn/equipment. Both actual native use_item readiness controls must pass, then use_items must select/execute exactly the first slot's native spell. No target/item metadata mock, resource grant, canonical write or class-specific execution shim. RED pending; no use_items implementation yet. Exact broader options/source exclusions must be explicit before claiming their qualification.
