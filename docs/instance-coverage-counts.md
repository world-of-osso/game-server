# DB2 instance coverage

Verified: 2026-10-11. Private SQLite backup of canonical world.db; no canonical publication. Importer source `4dbd10be`.

## Selection and preservation

Selection reads the already imported local Map.db2 and MapDifficulty.db2 tables. All dungeon/raid selection columns (ID, InstanceType, Directory) match `/home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.0.69933/Map.csv`; all 1,908 MapDifficulty rows match its pinned CSV. Other Map columns have 22 source-overlay differences and one extra common map2991, none affecting selection. No CSV download or source-table rewrite.

231 selected maps = existing124 plus107 DB2 maps. Garrison directories (including1158) are excluded from expansion; InstanceType5 scenarios are excluded unless already in the TDB union. 26 added maps have creatures (23 party,3 raid); Ancient Zul'Gurub309 is a genuine retired raid with only4 authored rows, not a populated replacement raid. 48 added maps have any raw placement.

Across selected maps: creatures 35260→41925; gameobjects 7676→8657. All124 existing maps have zero bidirectional spawn row differences. Existing-map templates, offered difficulty rows, models, equipment, addons, formations, events, paths, loot and existing pool memberships have zero row differences across35 content_* tables. New shared-entry difficulties and new-map pool memberships are additions outside the old-map projection, not silently counted as old124-map changes.

Shared base-row priority fixes four formerly changed Valarjar templates: new maps' lower difficulties must not alter an existing map's base template. TDB difficulty/phase/fallback/template/loot rules stay shared; resolved zero LootID preserves existing source-backed Wowhead bindings. Hogger regression remains passing.

## Hostability and reachability

Server `maps/instances.rs` creates/admit party copies without instance_template; raids still return MapNotAllowed. `maps/store.rs` resolves entrances through supported AreaTrigger shapes0/1 and TDB teleport→world_safe_locs. Hosted continents are0,1,2991. Entrance presence on an unhosted parent is not normal walk-in reachability.

New creature-populated dungeons reachable by authored walk-in from hosted continents with cached ground: Scarlet Monastery1004 and Scholomance1007. Upper Blackrock Spire1358 also has that path, but no authored creatures. Iron Docks1195 has a valid entrance from unhosted Draenor1116, so normal walk-in is blocked. Other newly covered creature maps have no loaded entrance; owned admin teleport can place a test character but is not an invented player entrance. Algeth'ar has safe_loc100018, no teleport, and all referenced ground files present (WDT4290605,12 root tiles,75 WMO roots/groups).

Ground columns are file-presence/structural census, not gameplay collision/render proof. Full missing FDIDs and entrance/source/target coordinates live in reachability.json. Empty-source maps and <=12-creature rows are labelled data gaps/near-empty candidates; counts above that are not a claim of complete encounters. No rows are filled or fabricated.

## Proof and lead command

Targeted Python importer: selection/Algethar RED2/2; shared-base preservation RED1/1; GREEN7/7 with ResourceWarnings errors. Private `before.sqlite`/`population.sqlite`, audit.py/json/log, reachability.py/json/log and test logs live under engine `data/diagnostics/instance-coverage-2026-10-11/`. Live both-skin proof pending.

After integrating this branch, the lead creates a fresh SQLite backup (not a symlink/hardlink) and runs:

```text
python3 scripts/import_instance_population.py --db /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/instance-coverage-2026-10-11/lead-population.sqlite --report /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/instance-coverage-2026-10-11/lead-import.json
```

Lead owns publication/swap/runtime reload. The private population.sqlite is evidence, not a canonical replacement.

## Per-map census

C/GO counts. Old marks TDB-template maps. Entrance = loaded inbound teleport count; Walk = from a hosted continent and party admission; Ground = all server ground files present. Data gaps are source counts, not import failures.

|Map|Name|Type|Old|TDB C/GO|Before C/GO|After C/GO|Entrance|Walk|Ground|Data gap|
|---:|---|:---:|:---:|---:|---:|---:|---:|:---:|:---:|---|
|33|Shadowfang Keep|P|yes|468/191|468/96|468/96|1|yes|present||
|34|Stormwind Stockade|P|yes|143/67|143/34|143/34|1|yes|present||
|36|Deadmines|P|yes|421/131|421/64|421/64|1|yes|present||
|43|Wailing Caverns|P|yes|148/58|148/58|148/58|1|yes|present||
|47|Razorfen Kraul|P|yes|190/37|190/37|190/37|1|yes|present||
|48|Blackfathom Deeps|P|yes|280/74|280/74|280/74|1|yes|present||
|70|Uldaman|P|yes|318/154|318/154|318/154|2|yes|present||
|90|Gnomeregan|P|yes|410/87|410/87|410/87|2|yes|present||
|109|Sunken Temple|P|yes|105/8|105/8|105/8|1|yes|present||
|129|Razorfen Downs|P|yes|261/46|261/46|261/46|1|yes|present||
|169|Emerald Dream|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|189|Scarlet Monastery of Old|P|yes|339/120|339/107|339/107|0|no|present||
|209|Zul'Farrak|P|yes|268/92|268/92|268/92|1|yes|present||
|229|Lower Blackrock Spire|P|yes|581/384|581/384|581/384|1|yes|present||
|230|Blackrock Depths|P|yes|1103/565|1103/447|1103/447|1|yes|present||
|249|Onyxia's Lair|R|yes|9/249|9/249|9/249|1|no|present|sparse authored creatures (<=12)|
|269|Opening of the Dark Portal|P|yes|261/1|261/1|261/1|1|yes|present||
|289|Scholomance|P|yes|403/61|403/61|403/61|0|no|present||
|309|Ancient Zul'Gurub|P|new|4/0|927/230|4/0|0|no|present|sparse authored creatures (<=12)|
|329|Stratholme|P|yes|461/432|461/432|461/432|2|yes|present||
|349|Maraudon|P|yes|610/60|610/60|610/60|2|yes|present||
|389|Ragefire Chasm|P|yes|138/60|138/60|138/60|1|yes|present||
|409|Molten Core|R|yes|244/22|244/22|244/22|4|no|present||
|429|Dire Maul|P|yes|942/338|942/319|942/319|6|yes|present||
|469|Blackwing Lair|R|yes|348/104|348/104|348/104|1|no|present||
|509|Ruins of Ahn'Qiraj|R|yes|377/10|377/10|377/10|1|no|present||
|531|Ahn'Qiraj Temple|R|yes|578/17|578/17|578/17|1|no|present||
|532|Karazhan|R|yes|889/315|889/314|889/314|2|no|present||
|533|Naxxramas|R|yes|927/132|927/132|927/132|4|no|present||
|534|The Battle for Mount Hyjal|R|yes|141/2|141/2|141/2|4|no|present||
|540|Hellfire Citadel: The Shattered Halls|P|yes|287/15|287/15|287/15|1|no|present||
|542|Hellfire Citadel: The Blood Furnace|P|yes|210/28|210/28|210/28|1|no|present||
|543|Hellfire Citadel: Ramparts|P|yes|101/4|101/4|101/4|1|no|present||
|544|Magtheridon's Lair|R|yes|33/13|33/13|33/13|1|no|present||
|545|Coilfang: The Steamvault|P|yes|162/34|162/34|162/34|1|no|present||
|546|Coilfang: The Underbog|P|yes|221/36|221/36|221/36|1|no|present||
|547|Coilfang: The Slave Pens|P|yes|231/40|231/20|231/20|1|no|present||
|548|Coilfang: Serpentshrine Cavern|R|yes|256/35|256/35|256/35|1|no|present||
|550|Tempest Keep|R|yes|223/9|223/9|223/9|1|no|present||
|552|Tempest Keep: The Arcatraz|P|yes|168/12|168/12|168/12|1|no|present||
|553|Tempest Keep: The Botanica|P|yes|188/2|188/2|188/2|1|no|present||
|554|Tempest Keep: The Mechanar|P|yes|82/12|82/12|82/12|1|no|present||
|555|Auchindoun: Shadow Labyrinth|P|yes|243/55|243/55|243/55|1|no|present||
|556|Auchindoun: Sethekk Halls|P|yes|197/16|197/16|197/16|1|no|present||
|557|Auchindoun: Mana-Tombs|P|yes|170/21|170/21|170/21|1|no|present||
|558|Auchindoun: Auchenai Crypts|P|yes|194/9|194/9|194/9|1|no|present||
|560|The Escape from Durnholde|P|yes|634/182|634/182|634/182|1|yes|present||
|564|Black Temple|R|yes|756/26|756/26|756/26|1|no|present||
|565|Gruul's Lair|R|yes|18/1|18/1|18/1|1|no|present||
|568|Zul'Aman|P|yes|520/74|520/74|520/74|1|no|present||
|574|Utgarde Keep|P|yes|166/57|166/57|166/57|1|no|present||
|575|Utgarde Pinnacle|P|yes|201/90|201/90|201/90|1|no|present||
|576|The Nexus|P|yes|268/27|268/27|268/27|1|no|present||
|578|The Oculus|P|yes|150/18|150/18|150/18|1|no|present||
|580|The Sunwell|R|yes|435/38|302/34|302/34|1|no|present||
|585|Magisters' Terrace|P|yes|226/20|226/20|226/20|1|no|present||
|595|The Culling of Stratholme|P|yes|424/425|424/162|424/162|1|yes|present||
|599|Halls of Stone|P|yes|147/19|147/19|147/19|1|no|present||
|600|Drak'Tharon Keep|P|yes|183/14|183/14|183/14|1|no|present||
|601|Azjol-Nerub|P|yes|120/73|120/73|120/73|1|no|present||
|602|Halls of Lightning|P|yes|189/10|189/10|189/10|1|no|present||
|603|Ulduar|R|yes|1430/167|1422/167|1422/167|1|no|present||
|604|Gundrak|P|yes|230/51|230/51|230/51|2|no|present||
|608|Violet Hold|P|yes|44/20|44/20|44/20|1|no|present||
|615|The Obsidian Sanctum|R|yes|162/4|162/4|162/4|1|no|present||
|616|The Eye of Eternity|R|yes|14/4|14/4|14/4|1|no|present||
|619|Ahn'kahet: The Old Kingdom|P|yes|216/11|216/11|216/11|1|no|present||
|624|Vault of Archavon|R|yes|18/0|18/0|18/0|1|no|present||
|631|Icecrown Citadel|R|yes|671/101|644/101|644/101|1|no|present||
|632|The Forge of Souls|P|yes|122/2|122/2|122/2|1|no|present||
|643|Throne of the Tides|P|yes|576/776|576/776|576/776|1|yes|present||
|644|Halls of Origination|P|yes|353/54|353/54|353/54|1|yes|present||
|645|Blackrock Caverns|P|yes|226/3|226/3|226/3|1|yes|present||
|649|Trial of the Crusader|R|yes|155/19|155/19|155/19|1|no|present||
|650|Trial of the Champion|P|yes|135/16|135/16|135/16|1|no|present||
|657|The Vortex Pinnacle|P|yes|113/2|113/2|113/2|1|yes|present||
|658|Pit of Saron|P|yes|358/71|358/71|358/71|2|no|present||
|668|Halls of Reflection|P|yes|38/14|38/14|38/14|3|no|present||
|669|Blackwing Descent|R|yes|99/40|99/40|99/40|1|no|present||
|670|Grim Batol|P|yes|529/2|529/2|529/2|1|yes|present||
|671|The Bastion of Twilight|R|yes|612/39|544/39|544/39|1|no|present||
|720|Firelands|R|yes|708/32|708/32|708/32|1|no|present||
|721|Firelands Terrain 2|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|724|The Ruby Sanctum|R|yes|50/24|47/23|47/23|1|no|present||
|725|The Stonecore|P|yes|433/5|433/5|433/5|1|no|present||
|734|Ahn'Qiraj Terrace|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|754|Throne of the Four Winds|R|yes|29/13|29/13|29/13|1|no|present||
|755|Lost City of the Tol'vir|P|yes|200/7|200/7|200/7|1|yes|present||
|757|Baradin Hold|R|yes|25/2|25/2|25/2|1|no|present||
|859|Zul'Gurub|P|yes|1002/23|1002/23|1002/23|1|yes|present||
|938|End Time|P|yes|370/37|370/37|370/37|1|yes|present||
|939|Well of Eternity|P|yes|269/32|269/32|269/32|1|yes|present||
|940|Hour of Twilight|P|yes|394/10|339/8|339/8|1|yes|present||
|951|Nexus Legendary|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|959|Shado-Pan Monastery|P|yes|0/1|0/1|0/1|1|no|present|no authored creatures|
|960|Temple of the Jade Serpent|P|yes|0/1|0/1|0/1|1|no|present|no authored creatures|
|961|Stormstout Brewery|P|yes|0/1|0/1|0/1|1|no|present|no authored creatures|
|962|Gate of the Setting Sun|P|yes|0/2|0/2|0/2|1|no|present|no authored creatures|
|967|Dragon Soul|R|yes|273/5|273/5|273/5|1|no|present||
|977|Maelstrom Deathwing Fight|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|994|Mogu'shan Palace|P|yes|0/1|0/1|0/1|1|no|present|no authored creatures|
|995|The Depths [UNUSED]|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|996|Terrace of Endless Spring|R|new|0/0|0/0|0/0|1|no|present|no authored creatures|
|1001|Scarlet Halls|P|yes|208/31|206/31|206/31|1|yes|present||
|1004|Scarlet Monastery|P|new|155/34|0/0|151/24|1|yes|present||
|1007|Scholomance|P|new|382/53|0/0|380/53|1|yes|present||
|1008|Mogu'shan Vaults|R|new|0/0|0/0|0/0|1|no|present|no authored creatures|
|1009|Heart of Fear|R|new|0/0|0/0|0/0|1|no|present|no authored creatures|
|1011|Siege of Niuzao Temple|P|yes|0/1|0/1|0/1|1|no|present|no authored creatures|
|1032|Pet Battle - Jade Forest|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1098|Throne of Thunder|R|yes|0/0|0/0|0/0|0|no|present|no authored creatures|
|1136|Siege of Orgrimmar|R|new|0/140|0/0|0/0|1|no|present|no authored creatures|
|1175|Bloodmaul Slag Mines|P|new|0/1|0/0|0/1|1|no|present|no authored creatures|
|1176|Shadowmoon Burial Grounds|P|new|0/1|0/0|0/1|1|no|present|no authored creatures|
|1182|Auchindoun|P|yes|157/21|157/21|157/21|1|no|present||
|1195|Iron Docks|P|new|238/63|0/0|238/63|1|no|present||
|1205|Blackrock Foundry|R|new|0/0|0/0|0/0|1|no|present|no authored creatures|
|1208|Grimrail Depot|P|new|0/1|0/0|0/1|1|no|present|no authored creatures|
|1209|Skyreach|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1228|Highmaul|R|new|0/0|0/0|0/0|1|no|present|no authored creatures|
|1268|Teron'gor's Confrontation|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1279|The Everbloom|P|new|0/1|0/0|0/1|1|no|present|no authored creatures|
|1358|Upper Blackrock Spire|P|new|0/1|0/0|0/1|1|yes|present|no authored creatures|
|1448|Hellfire Citadel|R|yes|0/0|0/0|0/0|0|no|present|no authored creatures|
|1456|Eye of Azshara|P|yes|622/29|622/29|622/29|0|no|present||
|1458|Neltharion's Lair|P|yes|309/33|309/33|309/33|0|no|present||
|1466|Darkheart Thicket|P|new|416/21|0/0|416/21|0|no|present||
|1477|Halls of Valor|P|new|317/41|0/0|317/41|0|no|present||
|1492|Maw of Souls|P|yes|112/55|112/55|112/55|0|no|present||
|1493|Vault of the Wardens|P|new|318/105|0/0|318/105|0|no|present||
|1501|Black Rook Hold|P|yes|284/25|284/25|284/25|0|no|present||
|1516|The Arcway|P|new|368/20|0/0|368/20|0|no|present||
|1520|The Emerald Nightmare|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1530|The Nighthold|R|new|696/124|0/0|696/124|0|no|present||
|1535|Moonglade|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1544|Assault on Violet Hold|P|new|86/20|0/0|86/20|0|no|present||
|1554|7.0 Boost Experience - Alliance|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1557|7.0 Boost Experience - Horde|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1571|Court of Stars|P|new|372/90|0/0|372/90|0|no|present||
|1592|Sporefall|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1594|The MOTHERLODE!!|P|new|0/2|0/0|0/2|0|no|present|no authored creatures|
|1648|Trial of Valor|R|yes|278/34|278/34|278/34|0|no|present||
|1651|Return to Karazhan|P|yes|630/297|630/297|630/297|0|no|present||
|1676|Tomb of Sargeras|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1677|Cathedral of Eternal Night|P|new|207/20|0/0|207/20|0|no|present||
|1712|Antorus, the Burning Throne|R|yes|1/3|1/3|1/3|0|no|present|sparse authored creatures (<=12)|
|1753|Seat of the Triumvirate|P|new|559/7|0/0|559/7|0|no|present||
|1754|Freehold|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|1762|Kings' Rest|P|yes|212/61|212/61|212/61|0|no|present||
|1763|Atal'Dazar|P|yes|218/21|218/21|218/21|0|no|present||
|1771|Tol Dagor|P|new|0/2|0/0|0/2|0|no|present|no authored creatures|
|1822|Siege of Boralus|P|new|0/2|0/0|0/2|0|no|present|no authored creatures|
|1841|The Underrot|P|yes|152/6|152/6|152/6|0|no|present||
|1861|Uldir|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1862|Waycrest Manor|P|yes|197/27|197/27|197/27|0|no|present||
|1864|Shrine of the Storm|P|yes|399/39|399/39|399/39|0|no|present||
|1877|Temple of Sethraliss|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|1949|8.0 Boost Experience - Alliance|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|1950|8.0 Boost Experience - Horde|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2070|Battle of Dazar'alor|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2096|Crucible of Storms|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2097|Operation: Mechagon|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2164|The Eternal Palace|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2217|Ny'alotha, the Waking City|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2233|Temple of the Jade Serpent - HoA Scenario|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2235|Caverns of Time - Anniversary|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2236|Darkmaul Citadel|P|yes|37/5|37/5|37/5|0|no|present||
|2284|Sanguine Depths|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|2285|Spires of Ascension|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|2286|The Necrotic Wake|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|2287|Halls of Atonement|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|2289|Plaguefall|P|new|0/2|0/0|0/2|0|no|present|no authored creatures|
|2290|Mists of Tirna Scithe|P|new|0/2|0/0|0/2|0|no|present|no authored creatures|
|2291|De Other Side|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|2293|Theater of Pain|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|2296|Castle Nathria|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2441|Tazavesh, the Veiled Market|P|new|750/58|0/0|750/58|0|no|present||
|2450|Sanctum of Domination|R|yes|2/12|2/12|2/12|0|no|present|sparse authored creatures (<=12)|
|2451|Uldaman: Legacy of Tyr|P|new|227/41|0/0|227/41|0|no|present||
|2481|Sepulcher of the First Ones|R|yes|12/2|11/2|11/2|0|no|present|sparse authored creatures (<=12)|
|2515|The Azure Vault|P|yes|225/19|225/19|225/19|0|no|present||
|2516|The Nokhud Offensive|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|2519|Neltharus|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|2520|Brackenhide Hollow|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2521|Ruby Life Pools|P|yes|306/70|306/70|306/70|0|no|present||
|2522|Vault of the Incarnates|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2526|Algeth'ar Academy|P|new|231/29|0/0|231/29|0|no|present||
|2527|Halls of Infusion|P|new|0/1|0/0|0/1|0|no|present|no authored creatures|
|2549|Amirdrassil, the Dream's Hope|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2559|Shadowlands|R|new|0/0|0/0|0/0|0|no|gap|no authored creatures|
|2569|Aberrus, the Shadowed Crucible|R|yes|82/5|82/5|82/5|0|no|present||
|2574|Dragon Isles|R|new|0/0|0/0|0/0|0|no|gap|no authored creatures|
|2579|Dawn of the Infinite|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2594|10.1.5 Time Rifts - Argus Raid UNUSED|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2645|10.1.5 Scarlet Monastery - Headless Horseman Revamp|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2648|The Rookery|P|new|93/95|0/0|93/91|0|no|present||
|2649|Priory of the Sacred Flame|P|new|357/29|0/0|357/29|0|no|present||
|2651|Darkflame Cleft|P|yes|277/241|277/241|277/241|0|no|present||
|2652|The Stonevault|P|yes|177/21|177/21|177/21|0|no|present||
|2657|Nerub-ar Palace|R|yes|80/71|80/71|80/71|0|no|present||
|2660|Ara-Kara, City of Echoes|P|new|408/47|0/0|408/47|0|no|present||
|2661|Cinderbrew Meadery|P|new|282/70|0/0|282/70|0|no|present||
|2662|The Dawnbreaker|P|new|44/47|0/0|44/47|0|no|present||
|2669|City of Threads|P|yes|864/116|864/116|864/116|0|no|present||
|2769|Liberation of Undermine|R|new|76/57|0/0|76/57|0|no|present||
|2773|Operation: Floodgate|P|new|382/121|0/0|382/110|0|no|present||
|2774|Khaz Algar|R|new|0/0|0/0|0/0|0|no|gap|no authored creatures|
|2779|zzzUnused|P|new|0/0|0/0|0/0|0|no|gap|no authored creatures|
|2792|Blackrock Depths|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2805|Windrunner Spire|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2810|Manaforge Omega|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2811|Magisters' Terrace|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2813|Murder Row|P|yes|2/1|2/1|2/1|0|no|present|sparse authored creatures (<=12)|
|2825|Den of Nalorakk|P|new|217/11|0/0|217/11|0|no|present||
|2830|Eco-Dome Al'dani|P|yes|167/12|167/12|167/12|0|no|present||
|2849|Dastardly Dome|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2859|The Blinding Vale|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2874|Maisara Caverns|P|new|376/6|0/0|376/6|0|no|present||
|2912|The Voidspire|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2913|March on Quel'Danas|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2915|Nexus-Point Xenas|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2923|Voidscar Arena|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2930|Midnight|R|new|0/0|0/0|0/0|0|no|gap|no authored creatures|
|2939|The Dreamrift|R|new|37/1|0/0|37/1|0|no|present||
|2950|Vault of the Incarnates|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2972|DEPRECATED Amirdrassil, the Dream's Hope (10.2 Raid - Dream) - Story Mode|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2987|The Tidebound Grotto|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|2993|Altar of Fangs|P|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|3004|The Venomous Abyss|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
|3029|Keystone Dungeons|P|new|0/0|0/0|0/0|0|no|gap|no authored creatures|
|3105|Dummy Dome|R|new|0/0|0/0|0/0|0|no|present|no authored creatures|
