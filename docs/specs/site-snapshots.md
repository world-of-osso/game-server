# site-data — static JSON contract v1 (2026-10-11)

Active goal: offline redb + world.db exporter and private fixtures; no listener, production access or publishing. Slot `/home/osso/.worktrees/game-server-s1`, branch `site-data`. Engine data/caches read-only except requested fixture output.

## Delivery layout / wire rules

All paths relative to snapshot root: `index.json`, `characters/<decimal-id>.json`, `guilds/<decimal-id>.json`, `rankings/mythic-plus.json`, `rankings/raids.json`, `icons/<FDID>.png|webp`. IDs encoded as strings for JS u64 safety; reference-table IDs remain numbers. UTF-8, snake_case, schema_version=1. Every document has `schema_version`, `realm` (slug), `as_of` (caller-supplied Unix seconds), `gaps` (string array). Null means unavailable, never zero; empty arrays mean no observed records, check gaps before treating as definitive absence. Dates Unix seconds, elapsed/timer milliseconds. Icons null when local web-ready asset unavailable. No remote URLs or account/private fields. Output deterministic for identical inputs + as_of. Publish whole snapshot atomically outside exporter (lead owns publishing).

`index.json`: realm_name, characters array of `{id,name,race:{id,name},class:{id,name},spec:{id,name}|null,level,faction|null,guild:{id,name}|null,item_level|null,search_keys:[lowercased name, guild name if present],url:"characters/<id>.json"}`, guilds array `{id,name,url}`. Search client-side, case-insensitive; keys are literal strings not fuzzy scores.

`characters/<id>.json`: same identity fields plus `equipped_items:[{slot,item_id,name,quality,item_level|null,icon_fdid|null,icon_url|null,appearance:{display_info_id|null,hidden},transmog:null|{appearance_id,display_info_id|null,hidden}}]`, `talents:{spec_id,selected_loadout_id,loadouts:[{id,name,selected,purchases:[{node_id,entry_id,ranks}]}]}|null`, `achievement_points:number|null`, `recent_achievements:[{id,name,points,earned_at|null,icon_fdid|null,icon_url|null}]`, `mythic_plus_best:[]`, `raid_progression:[]`, `pvp:null|{honor,honor_max,conquest,conquest_max,brackets:[{bracket,rating,season_wins,season_losses,weekly_wins,weekly_losses}]}`, `render_url:null`. Character contribution achievements only, not account aggregates.

`guilds/<id>.json`: id,name,roster (same summaries), raid_progression array. Do not export MOTD/info/officer notes. Guild progression only from recorded guild-at-kill attribution; current membership is not historical guild credit.

## Ranking records (design may build empty states now)

`rankings/mythic-plus.json`: `policy:"highest_timed_key_then_clear_time"`, `runs:[{id,season_id,dungeon:{id,name},level,elapsed_ms,timer_ms,in_time,completed_at,party:[{character_id,name,class_id,spec_id,role}],rank}]`. Authored timer required, in_time = elapsed_ms <= timer_ms. Per-season/dungeon rows timed first, level descending, elapsed_ms ascending, ID ascending stable tie-break; `rank` ordinal within that season/dungeon. UI level filter exact, not a score. Character best one observed timed run per dungeon/season by same comparator (no timed run: best completed level then time, explicitly in_time=false). No invented Retail M+ rating.

`rankings/raids.json`: `policy:"kills_per_difficulty_then_first_kill"`, `characters:[]`, `guilds:[]`. Each ranking/progression `{character_id|guild_id,raid:{id,name},difficulty_id,kills,total,encounters:[{id,name,killed,first_kill_at:null|seconds}],first_kill_at:null|seconds,rank}`. One difficulty per row; total counts authored unique encounter bits for raid/difficulty. Sort within raid/difficulty kills descending, recorded first-kill timestamp ascending, ID stable tie-break. Null timestamps after known timestamps. No reset-lock expiry timestamp mislabeled as a kill timestamp. Reset locks are snapshots, not lifetime progression; exporter labels `scope:"saved_lock"` if that is the only evidence.

## Privacy / availability

Operator must explicitly supply public-realm character ID allowlist, minus opt-out ID list. Redb does not currently store realm membership or armory privacy: never infer public eligibility from account names, character names or map. Private fixtures use explicit fb-owned allowlist only. Filter ranking parties to fully public participants; omit mixed/private parties rather than leak identity/composition. Never emit account IDs/names/emails/IPs/tokens, inventories, money, coordinates or private guild text.

Observed source gaps: current origin/master has no Mythic+ persistence; raid locks lack first-kill times/lifetime/guild-at-kill attribution. Race names/faction catalog absent from local world.db. Character render optional and currently absent. Item level means authored base item level (effective bonus/scaling absent); overall equipped average withheld if actual complete equipment is unavailable. Gaps remain explicit, not invented fixtures.

