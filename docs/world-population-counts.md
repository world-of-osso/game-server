# Open-world population (2026-10-11)

Scope: TDB1210 base-phase population, not quest/encounter/phase-system parity. Existing canonical world.db was read-only throughout. All31 selected maps had zero canonical content NPC rows and locally present WDT/root ADTs. Selection covers expansion4+ source-backed maps, prioritized by Dragon Isles, Khaz Algar, Shadowlands and BfA; source-empty maps are not fabricated. Unrelated map rows and authored placements/pools remain intact, including placements on a promoted map outside the TDB GUID namespace. Full content regeneration remains a separate operation; reapply the explicit scoped promotion after regeneration.

Private copy adds12,427 creature rows and2,424 gameobject rows. Runtime host allowlist was a second blocker (only0/1/2991); it now includes exactly the31 selected maps. Live startup records2556 Dragon Isles,2848 Khaz Algar surface,43 Khaz Algar underground,827 Shadowlands and2240 Kul Tiras NPCs. Zandalar has only one base-phase source NPC (inactive event12 Headless Ghost), so its living NPC count stays0; six base non-event gameobjects import. This is a source gap, not manufactured hub parity.

Proof: Python process/copy tests2/2, Rust admission/private-copy living-NPC tests2/2, raw-table/non-target spawn row diffs, final42-content-table hash equivalence after reapplication. Private UDP5820, fb_world_population/Worldpop, Boralus zone8567 / area8718: both skins show and natively select authored Wixler Brass124720 at1123.62,-629.936,17.8452; source GUID7000270 → content27000270. Server target matches client target and HP11266/11266. Both1280px PNGs inspected. Nearby64 other NPC display IDs lack imported appearance coverage; that limitation is recorded separately, not hidden by the successful single-NPC spot check.

Evidence: engine `data/diagnostics/world-population-2026-10-11/` (baseline.sqlite, population.sqlite, row-diff.json, diff-content_*.json, final-row-equivalence.json, census.json, source/asset gaps, live logs, both-skin target state); requested PNGs: `/syncthing/AgentShared/2026-10-11/world-population/{forever,modern}-boralus.png`. Engine untouched in git; leased engine/protocol slots released after capture, owned processes stopped.

# Open-world population census

Verified: 2026-10-11. Map.csv InstanceType=0; all maps retained in census, including unused/test/transport variants. Local root ADT presence matched by FDID through the existing community listfile; WDT by FDID. Local tiles prove available terrain, not full rendered gameplay. Source: canonical TDB1210 tables read-only; before: consistent baseline backup; after: private promoted copy.

Counts C/GO. Phase/difficulty/event/template/filter exclusions are disjoint in that order. Event = positive-only membership (potentially inactive); creature event rows remain in content and runtime scheduler decides activity. Filter = otherwise eligible source GUID absent from canonical content (the importer's old map selection), not a claim every variant should be hosted. Source-empty maps are not fabricated. Zone0 means source has no assigned zone. GO loader = model/type candidates, not live ECS count. Runtime columns mean content_* rows; active schedules/pools/difficulty/asset receipt rendering can further restrict live actors.

|Map / zone|Expansion|Local roots / WDT|Source C/GO|Before content C/GO|After copy C/GO|Phase C/GO|Difficulty C/GO|Event C/GO|Missing template C/GO|Map filter C/GO|GO loader candidates|Before / after hosted NPC candidates|
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
|0 Eastern Kingdoms / 0 unassigned|0|1176 / yes|45088/11668|44429/11435|44429/11435|666/39|0/0|90/195|0/0|0/0|7545|44339 / 44339|
|0 Eastern Kingdoms / 1 Dun Morogh|0|1176 / yes|221/815|221/63|221/63|0/0|0/0|178/752|0/0|0/0|0|43 / 43|
|0 Eastern Kingdoms / 3 Badlands|0|1176 / yes|7/244|7/1|7/1|0/1|0/0|5/243|0/0|0/0|1|2 / 2|
|0 Eastern Kingdoms / 4 Blasted Lands|0|1176 / yes|20/351|12/0|12/0|9/1|0/0|6/350|0/0|0/0|0|6 / 6|
|0 Eastern Kingdoms / 8 Swamp of Sorrows|0|1176 / yes|7/266|7/0|7/0|0/0|0/0|3/266|0/0|0/0|0|4 / 4|
|0 Eastern Kingdoms / 10 Duskwood|0|1176 / yes|6/163|6/0|6/0|0/0|0/0|2/163|0/0|0/0|0|4 / 4|
|0 Eastern Kingdoms / 11 Wetlands|0|1176 / yes|10/185|9/0|9/0|1/0|0/0|3/185|0/0|0/0|0|6 / 6|
|0 Eastern Kingdoms / 12 Elwynn Forest|0|1176 / yes|122/508|120/0|120/0|2/6|0/0|118/502|0/0|0/0|0|2 / 2|
|0 Eastern Kingdoms / 28 Western Plaguelands|0|1176 / yes|4/179|4/4|4/4|0/0|0/0|1/175|0/0|0/0|2|3 / 3|
|0 Eastern Kingdoms / 33 Northern Stranglethorn|0|1176 / yes|8/152|8/1|8/1|0/0|0/0|7/151|0/0|0/0|0|1 / 1|
|0 Eastern Kingdoms / 38 Loch Modan|0|1176 / yes|3/142|3/0|3/0|0/0|0/0|2/142|0/0|0/0|0|1 / 1|
|0 Eastern Kingdoms / 40 Westfall|0|1176 / yes|56/106|56/48|56/48|0/30|0/0|5/28|0/0|0/0|0|51 / 51|
|0 Eastern Kingdoms / 41 Deadwind Pass|0|1176 / yes|2/0|1/0|1/0|1/0|0/0|0/0|0/0|0/0|0|1 / 1|
|0 Eastern Kingdoms / 44 Redridge Mountains|0|1176 / yes|4/100|4/0|4/0|0/0|0/0|2/100|0/0|0/0|0|2 / 2|
|0 Eastern Kingdoms / 45 Arathi Highlands|0|1176 / yes|8/167|8/0|8/0|0/0|0/0|5/167|0/0|0/0|0|3 / 3|
|0 Eastern Kingdoms / 46 Burning Steppes|0|1176 / yes|10/57|10/5|10/5|0/0|0/0|4/52|0/0|0/0|4|6 / 6|
|0 Eastern Kingdoms / 47 The Hinterlands|0|1176 / yes|7/314|7/0|7/0|0/0|0/0|4/314|0/0|0/0|0|3 / 3|
|0 Eastern Kingdoms / 51 Searing Gorge|0|1176 / yes|1/109|1/1|1/1|0/0|0/0|0/108|0/0|0/0|0|1 / 1|
|0 Eastern Kingdoms / 85 Tirisfal Glades|0|1176 / yes|81/510|72/67|72/67|9/0|0/0|67/443|0/0|0/0|0|5 / 5|
|0 Eastern Kingdoms / 130 Silverpine Forest|0|1176 / yes|249/377|131/6|131/6|118/11|0/0|57/360|0/0|0/0|0|74 / 74|
|0 Eastern Kingdoms / 139 Eastern Plaguelands|0|1176 / yes|234/229|168/3|168/3|66/17|0/0|5/209|0/0|0/0|2|163 / 163|
|0 Eastern Kingdoms / 267 Hillsbrad Foothills|0|1176 / yes|12/182|12/6|12/6|0/0|0/0|8/176|0/0|0/0|6|4 / 4|
|0 Eastern Kingdoms / 1497 Undercity|0|1176 / yes|6/152|6/0|6/0|0/0|0/0|5/152|0/0|0/0|0|1 / 1|
|0 Eastern Kingdoms / 1519 Stormwind City|0|1176 / yes|1230/5094|506/100|506/100|724/253|0/0|236/4741|0/0|0/0|86|270 / 270|
|0 Eastern Kingdoms / 1537 Ironforge|0|1176 / yes|30/517|30/0|30/0|0/0|0/0|30/517|0/0|0/0|0|0 / 0|
|0 Eastern Kingdoms / 4706 Ruins of Gilneas|0|1176 / yes|230/20|0/0|0/0|230/20|0/0|0/0|0/0|0/0|0|0 / 0|
|0 Eastern Kingdoms / 4815 Kelp'thar Forest|0|1176 / yes|16/15|16/0|16/0|0/0|0/0|0/15|0/0|0/0|0|16 / 16|
|0 Eastern Kingdoms / 4922 Twilight Highlands|0|1176 / yes|6691/1670|4943/907|4943/907|1748/291|0/0|2/472|0/0|0/0|689|4941 / 4941|
|0 Eastern Kingdoms / 5144 Shimmering Expanse|0|1176 / yes|2/123|2/0|2/0|0/0|0/0|0/123|0/0|0/0|0|2 / 2|
|0 Eastern Kingdoms / 5145 Abyssal Depths|0|1176 / yes|3/117|3/1|3/1|0/0|0/0|0/116|0/0|0/0|0|3 / 3|
|0 Eastern Kingdoms / 5146 Vashj'ir|0|1176 / yes|1/0|1/0|1/0|0/0|0/0|0/0|0/0|0/0|0|1 / 1|
|0 Eastern Kingdoms / 5287 The Cape of Stranglethorn|0|1176 / yes|7/152|7/0|7/0|0/0|0/0|6/152|0/0|0/0|0|1 / 1|
|0 Eastern Kingdoms / 5339 Stranglethorn Vale|0|1176 / yes|2/0|2/0|2/0|0/0|0/0|0/0|0/0|0/0|0|2 / 2|
|0 Eastern Kingdoms / 6170 Northshire|0|1176 / yes|0/25|0/0|0/0|0/0|0/0|0/25|0/0|0/0|0|0 / 0|
|0 Eastern Kingdoms / 6176 Coldridge Valley|0|1176 / yes|0/5|0/0|0/0|0/0|0/0|0/5|0/0|0/0|0|0 / 0|
|0 Eastern Kingdoms / 6454 Deathknell|0|1176 / yes|4/4|1/0|1/0|3/0|0/0|0/4|0/0|0/0|0|1 / 1|
|0 Eastern Kingdoms / 6457 New Tinkertown|0|1176 / yes|0/29|0/0|0/0|0/0|0/0|0/29|0/0|0/0|0|0 / 0|
|0 Eastern Kingdoms / 11014 The Temple of Purity|0|1176 / yes|2/0|2/0|2/0|0/0|0/0|0/0|0/0|0/0|0|2 / 2|
|0 Eastern Kingdoms / 14046 Tirisfal Glades|0|1176 / yes|96/376|96/6|96/6|0/0|0/0|93/370|0/0|0/0|3|3 / 3|
|0 Eastern Kingdoms / 15947 Zul'Aman|0|1176 / yes|7/0|7/0|7/0|0/0|0/0|0/0|0/0|0/0|0|7 / 7|
|0 Eastern Kingdoms / 15968 Eversong Woods|0|1176 / yes|14/0|11/0|11/0|3/0|0/0|0/0|0/0|0/0|0|11 / 11|
|0 Eastern Kingdoms / 15969 Silvermoon City|0|1176 / yes|7/0|2/0|2/0|5/0|0/0|0/0|0/0|0/0|0|2 / 2|
|1 Kalimdor / 0 unassigned|0|1043 / yes|49435/11040|49300/10648|49300/10648|135/42|0/0|44/350|0/0|0/0|6773|49256 / 49256|
|1 Kalimdor / 14 Durotar|0|1043 / yes|247/819|207/14|207/14|40/1|0/0|203/804|0/0|0/0|14|4 / 4|
|1 Kalimdor / 15 Dustwallow Marsh|0|1043 / yes|7/115|7/2|7/2|0/0|0/0|4/113|0/0|0/0|0|3 / 3|
|1 Kalimdor / 16 Azshara|0|1043 / yes|14/63|8/0|8/0|6/0|0/0|0/63|0/0|0/0|0|8 / 8|
|1 Kalimdor / 17 Northern Barrens|0|1043 / yes|20/244|20/0|20/0|0/0|0/0|15/244|0/0|0/0|0|5 / 5|
|1 Kalimdor / 141 Teldrassil|0|1043 / yes|13/186|13/67|13/67|0/0|0/0|7/119|0/0|0/0|1|6 / 6|
|1 Kalimdor / 148 Darkshore|0|1043 / yes|6/47|6/0|6/0|0/0|0/0|2/47|0/0|0/0|0|4 / 4|
|1 Kalimdor / 215 Mulgore|0|1043 / yes|9/189|9/65|9/65|0/0|0/0|7/124|0/0|0/0|0|2 / 2|
|1 Kalimdor / 331 Ashenvale|0|1043 / yes|10/223|10/0|10/0|0/0|0/0|5/223|0/0|0/0|0|5 / 5|
|1 Kalimdor / 357 Feralas|0|1043 / yes|14/256|14/0|14/0|0/0|0/0|7/256|0/0|0/0|0|7 / 7|
|1 Kalimdor / 361 Felwood|0|1043 / yes|4/112|4/0|4/0|0/0|0/0|2/112|0/0|0/0|0|2 / 2|
|1 Kalimdor / 400 Thousand Needles|0|1043 / yes|10/26|10/0|10/0|0/0|0/0|3/26|0/0|0/0|0|7 / 7|
|1 Kalimdor / 405 Desolace|0|1043 / yes|10/191|10/0|10/0|0/0|0/0|4/191|0/0|0/0|0|6 / 6|
|1 Kalimdor / 406 Stonetalon Mountains|0|1043 / yes|11/412|11/0|11/0|0/0|0/0|4/412|0/0|0/0|0|7 / 7|
|1 Kalimdor / 440 Tanaris|0|1043 / yes|20/171|20/8|20/8|0/0|0/0|13/163|0/0|0/0|0|7 / 7|
|1 Kalimdor / 490 Un'Goro Crater|0|1043 / yes|2/35|2/0|2/0|0/0|0/0|0/35|0/0|0/0|0|2 / 2|
|1 Kalimdor / 493 Moonglade|0|1043 / yes|20/0|2/0|2/0|18/0|0/0|0/0|0/0|0/0|0|2 / 2|
|1 Kalimdor / 616 Mount Hyjal|0|1043 / yes|5082/914|2335/427|2335/427|2747/424|0/0|0/63|0/0|0/0|300|2335 / 2335|
|1 Kalimdor / 618 Winterspring|0|1043 / yes|14/117|14/0|14/0|0/0|0/0|12/117|0/0|0/0|0|2 / 2|
|1 Kalimdor / 1377 Silithus|0|1043 / yes|6/68|6/0|6/0|0/0|0/0|3/68|0/0|0/0|0|3 / 3|
|1 Kalimdor / 1519 Stormwind City|0|1043 / yes|1/0|1/0|1/0|0/0|0/0|1/0|0/0|0/0|0|0 / 0|
|1 Kalimdor / 1637 Orgrimmar|0|1043 / yes|241/1071|197/15|197/15|44/0|0/0|35/1056|0/0|0/0|8|162 / 162|
|1 Kalimdor / 1638 Thunder Bluff|0|1043 / yes|32/371|32/0|32/0|0/0|0/0|32/371|0/0|0/0|0|0 / 0|
|1 Kalimdor / 1657 Darnassus|0|1043 / yes|45/289|45/0|45/0|0/0|0/0|44/289|0/0|0/0|0|1 / 1|
|1 Kalimdor / 4709 Southern Barrens|0|1043 / yes|10/438|10/0|10/0|0/0|0/0|4/438|0/0|0/0|0|6 / 6|
|1 Kalimdor / 5034 Uldum|0|1043 / yes|9/103|9/4|9/4|0/0|0/0|0/99|0/0|0/0|1|9 / 9|
|1 Kalimdor / 5695 Ahn'Qiraj: The Fallen Kingdom|0|1043 / yes|3/0|3/0|3/0|0/0|0/0|0/0|0/0|0/0|0|3 / 3|
|1 Kalimdor / 6450 Shadowglen|0|1043 / yes|0/5|0/0|0/0|0/0|0/0|0/5|0/0|0/0|0|0 / 0|
|1 Kalimdor / 6451 Valley of Trials|0|1043 / yes|0/7|0/0|0/0|0/0|0/0|0/7|0/0|0/0|0|0 / 0|
|1 Kalimdor / 6452 Camp Narache|0|1043 / yes|0/5|0/0|0/0|0/0|0/0|0/5|0/0|0/0|0|0 / 0|
|1 Kalimdor / 6453 Echo Isles|0|1043 / yes|4/4|4/0|4/0|0/0|0/0|0/4|0/0|0/0|0|4 / 4|
|1 Kalimdor / 9310 Silithus: The Wound|0|1043 / yes|17/1|0/0|0/0|17/1|0/0|0/0|0/0|0/0|0|0 / 0|
|369 Deeprun Tram / 0 unassigned|0|0 / yes|110/197|28/91|28/91|0/0|0/0|0/0|0/0|110/197|91|0 / 0|
|369 Deeprun Tram / 2257 Deeprun Tram|0|0 / yes|0/0|0/15|0/15|0/0|0/0|0/0|0/0|0/0|6|0 / 0|
|449 Alliance PVP Barracks / 0 unassigned|0|0 / yes|10/21|0/1|0/1|0/0|0/0|0/0|0/0|10/21|0|0 / 0|
|450 Horde PVP Barracks / 0 unassigned|0|0 / yes|11/12|0/1|0/1|0/0|0/0|0/0|0/0|11/12|0|0 / 0|
|530 Outland / 0 unassigned|1|826 / yes|30264/17229|30162/13144|30162/13144|229/99|0/0|1115/7111|0/0|28920/10019|12755|0 / 0|
|530 Outland / 210 Icecrown|1|826 / yes|0/1|0/0|0/0|0/0|0/0|0/0|0/0|0/1|0|0 / 0|
|530 Outland / 3430 Eversong Woods (Burning Crusade)|1|826 / yes|82/424|0/600|0/600|0/0|0/0|80/332|0/0|2/92|270|0 / 0|
|530 Outland / 3433 Ghostlands|1|826 / yes|3/123|95/530|95/530|1/0|0/0|2/81|0/0|0/42|190|0 / 0|
|530 Outland / 3479 The Veiled Sea|1|826 / yes|0/1|0/1|0/1|0/0|0/0|0/0|0/0|0/1|1|0 / 0|
|530 Outland / 3483 Hellfire Peninsula|1|826 / yes|4/7|0/1300|0/1300|0/0|0/0|0/0|0/0|4/7|547|0 / 0|
|530 Outland / 3487 Silvermoon City (Burning Crusade)|1|826 / yes|36/1096|0/65|0/65|0/0|0/0|36/1095|0/0|0/1|65|0 / 0|
|530 Outland / 3518 Nagrand|1|826 / yes|3/82|47/1165|47/1165|0/0|0/0|0/82|0/0|3/0|381|0 / 0|
|530 Outland / 3519 Terokkar Forest|1|826 / yes|51/58|183/1098|183/1098|0/0|0/0|18/53|0/0|33/5|669|0 / 0|
|530 Outland / 3520 Shadowmoon Valley|1|826 / yes|0/177|524/1372|524/1372|0/0|0/0|0/126|0/0|0/51|928|0 / 0|
|530 Outland / 3521 Zangarmarsh|1|826 / yes|0/54|4/1179|4/1179|0/0|0/0|0/50|0/0|0/4|445|0 / 0|
|530 Outland / 3522 Blade's Edge Mountains|1|826 / yes|0/66|0/1101|0/1101|0/0|0/0|0/66|0/0|0/0|618|0 / 0|
|530 Outland / 3523 Netherstorm|1|826 / yes|7/12|44/1106|44/1106|0/0|0/0|6/0|0/0|1/12|658|0 / 0|
|530 Outland / 3524 Azuremyst Isle|1|826 / yes|73/494|0/540|0/540|0/0|0/0|68/421|0/0|5/73|254|0 / 0|
|530 Outland / 3525 Bloodmyst Isle|1|826 / yes|2/117|0/510|0/510|0/0|0/0|2/89|0/0|0/28|32|0 / 0|
|530 Outland / 3540 Twisting Nether|1|826 / yes|0/0|0/31|0/31|0/0|0/0|0/0|0/0|0/0|31|0 / 0|
|530 Outland / 3557 The Exodar|1|826 / yes|25/720|0/18|0/18|0/0|0/0|25/719|0/0|0/1|18|0 / 0|
|530 Outland / 3703 Shattrath City|1|826 / yes|43/1|0/89|0/89|0/0|0/0|29/0|0/0|14/1|77|0 / 0|
|530 Outland / 4080 Isle of Quel'Danas|1|826 / yes|119/2|0/164|0/164|119/1|0/0|0/0|0/0|0/1|110|0 / 0|
|530 Outland / 4815 Kelp'thar Forest|1|826 / yes|0/3|0/0|0/0|0/0|0/0|0/0|0/0|0/3|0|0 / 0|
|530 Outland / 5340 Hall of Arms|1|826 / yes|0/0|0/1|0/1|0/0|0/0|0/0|0/0|0/0|1|0 / 0|
|530 Outland / 6455 Sunstrider Isle|1|826 / yes|257/1|0/0|0/0|0/0|0/0|0/0|0/0|257/1|0|0 / 0|
|530 Outland / 6456 Ammen Vale|1|826 / yes|0/3|0/0|0/0|0/0|0/0|0/3|0/0|0/0|0|0 / 0|
|571 Northrend / 0 unassigned|2|1131 / yes|35018/18089|33595/13387|33595/13387|1389/566|0/0|240/6574|0/0|33389/10949|11907|0 / 0|
|571 Northrend / 65 Dragonblight|2|1131 / yes|0/254|1/381|1/381|0/0|0/0|0/247|0/0|0/7|267|0 / 0|
|571 Northrend / 66 Zul'Drak|2|1131 / yes|0/77|0/790|0/790|0/0|0/0|0/74|0/0|0/3|339|0 / 0|
|571 Northrend / 67 The Storm Peaks|2|1131 / yes|178/36|0/1350|0/1350|164/10|0/0|2/23|0/0|12/3|465|0 / 0|
|571 Northrend / 210 Icecrown|2|1131 / yes|532/80|0/1977|0/1977|517/1|0/0|0/0|0/0|15/79|989|0 / 0|
|571 Northrend / 394 Grizzly Hills|2|1131 / yes|0/123|1/313|1/313|0/0|0/0|0/123|0/0|0/0|100|0 / 0|
|571 Northrend / 495 Howling Fjord|2|1131 / yes|0/199|0/460|0/460|0/0|0/0|0/197|0/0|0/2|140|0 / 0|
|571 Northrend / 1637 Orgrimmar|2|1131 / yes|12/0|0/0|0/0|0/0|0/0|12/0|0/0|0/0|0|0 / 0|
|571 Northrend / 2817 Crystalsong Forest|2|1131 / yes|0/0|0/24|0/24|0/0|0/0|0/0|0/0|0/0|16|0 / 0|
|571 Northrend / 3537 Borean Tundra|2|1131 / yes|1/182|33/348|33/348|0/0|0/0|0/180|0/0|1/2|146|0 / 0|
|571 Northrend / 3711 Sholazar Basin|2|1131 / yes|2/48|0/920|0/920|0/0|0/0|0/45|0/0|2/3|433|0 / 0|
|571 Northrend / 4197 Wintergrasp|2|1131 / yes|0/4|0/610|0/610|0/0|0/0|0/4|0/0|0/0|265|0 / 0|
|571 Northrend / 4395 Dalaran|2|1131 / yes|22/40|0/6|0/6|0/0|0/0|22/39|0/0|0/1|6|0 / 0|
|571 Northrend / 6455 Sunstrider Isle|2|1131 / yes|1/0|0/0|0/0|0/0|0/0|0/0|0/0|1/0|0|0 / 0|
|582 Transport: Rut'theran to Auberdine / 0 unassigned|0|0 / no|8/0|6/0|6/0|0/0|0/0|0/0|0/0|8/0|0|0 / 0|
|584 Transport: Menethil to Theramore / 0 unassigned|0|0 / no|11/0|11/9|11/9|0/0|0/0|0/0|0/0|11/0|9|0 / 0|
|586 Transport: Exodar to Auberdine / 0 unassigned|0|0 / no|8/0|6/0|6/0|0/0|0/0|0/0|0/0|8/0|0|0 / 0|
|587 Transport: Feathermoon Ferry / 0 unassigned|0|0 / no|9/0|9/0|9/0|0/0|0/0|0/0|0/0|9/0|0|0 / 0|
|588 Transport: Menethil to Auberdine / 0 unassigned|0|0 / no|10/0|10/9|10/9|0/0|0/0|0/0|0/0|10/0|9|0 / 0|
|589 Transport: Orgrimmar to Grom'Gol / 0 unassigned|0|0 / no|11/0|11/0|11/0|0/0|0/0|0/0|0/0|11/0|0|0 / 0|
|590 Transport: Grom'Gol to Undercity / 0 unassigned|0|0 / no|11/0|11/0|11/0|0/0|0/0|0/0|0/0|11/0|0|0 / 0|
|591 Transport: Undercity to Orgrimmar / 0 unassigned|0|0 / no|21/0|21/0|21/0|0/0|0/0|0/0|0/0|21/0|0|0 / 0|
|592 Transport: Borean Tundra Test / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|593 Transport: Booty Bay to Ratchet / 0 unassigned|0|0 / no|10/9|8/9|8/9|0/0|0/0|0/0|0/0|10/9|9|0 / 0|
|594 Transport: Howling Fjord Sister Mercy (Quest) / 0 unassigned|2|0 / no|11/0|24/10|24/10|0/0|0/0|0/0|0/0|11/0|10|0 / 0|
|594 Transport: Howling Fjord Sister Mercy (Quest) / 495 Howling Fjord|2|0 / no|57/0|0/0|0/0|0/0|0/0|0/0|0/0|57/0|0|0 / 0|
|596 Transport: Naglfar / 0 unassigned|2|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|609 Ebon Hold / 0 unassigned|2|42 / yes|1939/366|2041/777|2041/777|1343/83|0/0|0/0|0/0|596/283|653|0 / 0|
|610 Transport: Tirisfal to Vengeance Landing / 0 unassigned|0|0 / no|7/0|7/0|7/0|0/0|0/0|0/0|0/0|7/0|0|0 / 0|
|612 Transport: Menethil to Valgarde / 0 unassigned|0|0 / no|6/0|6/0|6/0|0/0|0/0|0/0|0/0|6/0|0|0 / 0|
|613 Transport: Orgrimmar to Warsong Hold / 0 unassigned|0|0 / no|7/0|7/0|7/0|0/0|0/0|0/0|0/0|7/0|0|0 / 0|
|614 Transport: Stormwind to Valiance Keep / 0 unassigned|0|0 / no|6/0|6/0|6/0|0/0|0/0|0/0|0/0|6/0|0|0 / 0|
|620 Transport: Moa'ki to Unu'pe / 0 unassigned|2|0 / no|2/0|2/0|2/0|0/0|0/0|0/0|0/0|2/0|0|0 / 0|
|621 Transport: Moa'ki to Kamagua / 0 unassigned|2|0 / no|2/0|2/0|2/0|0/0|0/0|0/0|0/0|2/0|0|0 / 0|
|622 Transport: Orgrim's Hammer / 0 unassigned|2|0 / no|71/0|71/1|71/1|0/0|0/0|0/0|0/0|71/0|1|0 / 0|
|622 Transport: Orgrim's Hammer / 210 Icecrown|2|0 / no|3/0|0/0|0/0|0/0|0/0|0/0|0/0|3/0|0|0 / 0|
|623 Transport: The Skybreaker / 0 unassigned|2|0 / no|77/0|77/7|77/7|0/0|0/0|0/0|0/0|77/0|7|0 / 0|
|623 Transport: The Skybreaker / 210 Icecrown|2|0 / no|4/0|0/0|0/0|0/0|0/0|0/0|0/0|4/0|0|0 / 0|
|673 Transport: Orgrim's Hammer (Icecrown Citadel Raid) / 0 unassigned|2|0 / no|29/4|29/4|29/4|0/0|2/3|0/0|0/0|27/1|4|0 / 0|
|674 Transport: Ship to Vashj'ir / 0 unassigned|3|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|638 Gilneas / 0 unassigned|0|11 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|641 Transport: Alliance Airship BG / 0 unassigned|2|0 / no|18/2|18/2|18/2|0/0|0/0|0/0|0/0|18/2|0|0 / 0|
|641 Transport: Alliance Airship BG / 4710 Isle of Conquest|2|0 / no|1/0|0/0|0/0|0/0|0/0|0/0|0/0|1/0|0|0 / 0|
|642 Transport: HordeAirshipBG / 0 unassigned|2|0 / no|18/2|18/2|18/2|0/0|0/0|0/0|0/0|18/2|0|0 / 0|
|642 Transport: HordeAirshipBG / 4710 Isle of Conquest|2|0 / no|2/0|0/0|0/0|0/0|0/0|0/0|0/0|2/0|0|0 / 0|
|646 Deepholm / 5042 Deepholm|3|100 / yes|6524/1063|0/0|0/0|1111/95|0/0|0/27|0/1|5413/940|0|0 / 0|
|647 Transport: Orgrimmar to Thunder Bluff / 0 unassigned|2|0 / no|12/0|12/0|12/0|0/0|0/0|0/0|0/0|12/0|0|0 / 0|
|648 LostIsles / 0 unassigned|0|149 / yes|27/13|0/0|0/0|0/0|0/0|0/0|0/0|27/13|0|0 / 0|
|648 LostIsles / 4720 The Lost Isles|0|149 / yes|5622/1000|0/0|0/0|5621/1000|0/0|0/0|0/0|1/0|0|0 / 0|
|648 LostIsles / 4737 Kezan|0|149 / yes|2007/710|0/0|0/0|2007/710|0/0|0/0|0/0|0/0|0|0 / 0|
|654 Gilneas2 / 4714 Gilneas|0|60 / yes|2293/366|0/0|0/0|2293/366|0/0|0/0|0/0|0/0|0|0 / 0|
|654 Gilneas2 / 4755 Gilneas City|0|60 / yes|1968/61|0/0|0/0|1891/61|0/0|0/0|0/0|77/0|0|0 / 0|
|655 GilneasPhase1 / 0 unassigned|0|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|656 GilneasPhase2 / 0 unassigned|0|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|659 Lost Isles Volcano Eruption / 0 unassigned|0|12 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|661 Lost Isles Town in a Box / 0 unassigned|0|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|662 Transport: Alliance Vashj'ir Ship / 0 unassigned|3|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|672 Transport: The Skybreaker (Icecrown Citadel Raid) / 0 unassigned|2|0 / no|30/4|29/4|29/4|0/0|2/3|0/0|0/0|28/1|4|0 / 0|
|712 Transport: The Skybreaker (IC Dungeon) / 0 unassigned|2|0 / no|28/3|50/3|50/3|0/0|0/1|0/0|0/0|28/2|2|0 / 0|
|713 Transport: Orgrim's Hammer (IC Dungeon) / 0 unassigned|2|0 / no|55/4|48/3|48/3|0/0|0/1|0/0|0/0|55/3|2|0 / 0|
|718 Transport: The Mighty Wind (Icecrown Citadel Raid) / 0 unassigned|2|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|719 Mount Hyjal Phase 1 / 0 unassigned|0|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|723 Stormwind / 0 unassigned|0|25 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|730 Maelstrom Zone / 4922 Twilight Highlands|3|25 / yes|80/0|0/0|0/0|80/0|0/0|0/0|0/0|0/0|0|0 / 0|
|730 Maelstrom Zone / 5416 The Maelstrom|3|25 / yes|7/0|0/0|0/0|0/0|0/0|0/0|0/0|7/0|0|0 / 0|
|731 Stonetalon Bomb / 0 unassigned|0|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|732 Tol Barad / 0 unassigned|3|42 / yes|828/325|0/0|0/0|0/0|0/0|0/0|0/0|828/325|0|0 / 0|
|736 Twilight Highlands Dragonmaw Phase / 0 unassigned|0|2 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|738 Ship to Vashj'ir (Orgrimmar -> Vashj'ir) / 0 unassigned|3|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|739 Vashj'ir Sub - Horde / 0 unassigned|3|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|740 Vashj'ir Sub - Alliance / 0 unassigned|3|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|741 Twilight Highlands Horde Transport / 11 Wetlands|3|0 / no|13/0|0/0|0/0|13/0|0/0|0/0|0/0|0/0|0|0 / 0|
|741 Twilight Highlands Horde Transport / 16 Azshara|3|0 / no|14/0|0/0|0/0|14/0|0/0|0/0|0/0|0/0|0|0 / 0|
|741 Twilight Highlands Horde Transport / 4922 Twilight Highlands|3|0 / no|6/0|0/0|0/0|6/0|0/0|0/0|0/0|0/0|0|0 / 0|
|742 Vashj'ir Sub - Horde - Circling Abyssal Maw / 0 unassigned|3|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|743 Vashj'ir Sub - Alliance circling Abyssal Maw / 0 unassigned|3|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|746 Uldum Phase Oasis / 0 unassigned|0|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|747 Transport: Deepholm Gunship / 5042 Deepholm|3|0 / no|78/11|0/0|0/0|0/0|0/0|0/0|0/0|78/11|0|0 / 0|
|748 Transport: Onyxia/Nefarian Elevator / 0 unassigned|3|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|749 Transport: Gilneas Moving Gunship / 4714 Gilneas|0|0 / no|104/4|0/0|0/0|104/4|0/0|0/0|0/0|0/0|0|0 / 0|
|750 Transport: Gilneas Static Gunship / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|751 Redridge - Orc Bomb / 0 unassigned|0|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|752 Redridge - Bridge Phase One / 0 unassigned|0|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|753 Redridge - Bridge Phase Two / 0 unassigned|0|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|759 Uldum Phased Entrance / 0 unassigned|0|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|760 Twilight Highlands Phased Entrance / 0 unassigned|0|2 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|762 Twilight Highlands Zeppelin 1 / 4922 Twilight Highlands|3|0 / no|8/10|0/0|0/0|0/0|0/0|0/0|0/0|8/10|0|0 / 0|
|763 Twilight Highlands Zeppelin 2 / 4922 Twilight Highlands|3|0 / no|4/5|0/0|0/0|0/0|0/0|0/0|0/0|4/5|0|0 / 0|
|764 Uldum - Phase Wrecked Camp / 0 unassigned|0|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|765 Krazzworks Attack Zeppelin / 0 unassigned|3|0 / no|30/1|0/0|0/0|30/1|0/0|0/0|0/0|0/0|0|0 / 0|
|765 Krazzworks Attack Zeppelin / 4922 Twilight Highlands|3|0 / no|0/32|0/0|0/0|0/32|0/0|0/0|0/0|0/0|0|0 / 0|
|766 Transport: Gilneas Moving Gunship 02 / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|767 Transport: Gilneas Moving Gunship 03 / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|860 The Wandering Isle / 0 unassigned|0|100 / yes|2229/816|0/0|0/0|6/6|0/0|0/0|0/0|2223/810|0|0 / 0|
|860 The Wandering Isle / 5736 The Wandering Isle|0|100 / yes|60/82|0/0|0/0|2/70|0/0|0/0|0/0|58/12|0|0 / 0|
|861 Molten Front / 0 unassigned|3|80 / yes|430/0|0/0|0/0|0/0|0/0|0/0|0/0|430/0|0|0 / 0|
|870 Pandaria / 5785 The Jade Forest **promoted**|4|684 / yes|477/178|0/0|16/2|461/133|0/0|0/43|0/0|16/2|0|0 / 16|
|870 Pandaria / 5805 Valley of the Four Winds **promoted**|4|684 / yes|9/7|0/0|9/1|0/0|0/0|0/6|0/0|9/1|0|0 / 9|
|870 Pandaria / 5840 Vale of Eternal Blossoms **promoted**|4|684 / yes|8/6|0/0|8/2|0/0|0/0|0/4|0/0|8/2|0|0 / 8|
|870 Pandaria / 5841 Kun-Lai Summit **promoted**|4|684 / yes|18/17|0/0|18/1|0/6|0/0|0/10|0/0|18/1|0|0 / 18|
|870 Pandaria / 5842 Townlong Steppes **promoted**|4|684 / yes|10/4|0/0|10/1|0/0|0/0|0/3|0/0|10/1|0|0 / 10|
|870 Pandaria / 6006 The Veiled Stair **promoted**|4|684 / yes|2/4|0/0|2/0|0/0|0/0|0/4|0/0|2/0|0|0 / 2|
|870 Pandaria / 6134 Krasarang Wilds **promoted**|4|684 / yes|7/9|0/0|7/0|0/0|0/0|0/9|0/0|7/0|0|0 / 7|
|870 Pandaria / 6138 Dread Wastes **promoted**|4|684 / yes|10/6|0/0|10/0|0/0|0/0|0/6|0/0|10/0|0|0 / 10|
|870 Pandaria / 6142 Shrine of Seven Stars **promoted**|4|684 / yes|0/4|0/0|0/0|0/0|0/0|0/4|0/0|0/0|0|0 / 0|
|870 Pandaria / 6554 Shrine of Two Moons **promoted**|4|684 / yes|0/2|0/0|0/0|0/0|0/0|0/2|0/0|0/0|0|0 / 0|
|870 Pandaria / 6661 Isle of Giants **promoted**|4|684 / yes|1/0|0/0|1/0|0/0|0/0|0/0|0/0|1/0|0|0 / 1|
|870 Pandaria / 6757 Timeless Isle **promoted**|4|684 / yes|10/0|0/0|10/0|0/0|0/0|0/0|0/0|10/0|0|0 / 10|
|971 Jade Forest Alliance Hub Phase / 0 unassigned|4|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|972 Jade Forest Battlefield Phase / 0 unassigned|4|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|974 Darkmoon Faire / 0 unassigned|0|49 / yes|309/676|0/0|0/0|0/0|0/0|0/0|0/0|309/676|0|0 / 0|
|974 Darkmoon Faire / 5861 Darkmoon Island|0|49 / yes|127/23|0/0|0/0|0/0|0/0|0/0|0/0|127/23|0|0 / 0|
|975 Turtle Ship Phase 01 / 0 unassigned|0|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|976 Turtle Ship Phase 02 / 0 unassigned|0|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1019 Ruins of Theramore / 0 unassigned|4|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1043 Brawl'gar Arena / 0 unassigned|2|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1061 Horde Beach Daily Area / 0 unassigned|4|8 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1062 Alliance Beach Daily Area / 0 unassigned|4|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1064 Mogu Island Daily Area / 6507 Isle of Thunder **promoted**|4|64 / yes|2/0|0/0|2/0|0/0|0/0|0/0|0/0|2/0|0|0 / 2|
|1066 Stormwind Gunship Pandaria Start Area / 0 unassigned|0|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1074 Orgrimmar Gunship Pandaria Start / 0 unassigned|0|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1075 Theramore's Fall Phase / 0 unassigned|0|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1076 Jade Forest Horde Starting Area / 0 unassigned|4|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1101 Defense Of The Ale House BG / 0 unassigned|4|42 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1106 Jaina Dalaran Scenario / 0 unassigned|4|20 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1107 Dreadscar Rift / 0 unassigned|6|16 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1113 Transport: DarkmoonCarousel / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1116 Draenor / 0 unassigned **promoted**|5|1221 / yes|201/105|0/0|201/105|0/0|0/0|0/0|0/0|201/105|99|0 / 201|
|1116 Draenor / 6662 Talador **promoted**|5|1221 / yes|7/2|0/0|7/2|0/0|0/0|0/0|0/0|7/2|0|0 / 7|
|1116 Draenor / 6719 Shadowmoon Valley **promoted**|5|1221 / yes|901/47|0/0|850/44|51/0|0/0|185/3|0/0|665/44|42|0 / 665|
|1116 Draenor / 6720 Frostfire Ridge **promoted**|5|1221 / yes|19/1|0/0|13/1|6/0|0/0|0/0|0/0|13/1|0|0 / 13|
|1116 Draenor / 6721 Gorgrond **promoted**|5|1221 / yes|12/3|0/0|12/3|0/0|0/0|0/0|0/0|12/3|0|0 / 12|
|1116 Draenor / 6722 Spires of Arak **promoted**|5|1221 / yes|10/1|0/0|10/1|0/0|0/0|0/0|0/0|10/1|0|0 / 10|
|1116 Draenor / 6755 Nagrand **promoted**|5|1221 / yes|5/0|0/0|5/0|0/0|0/0|0/0|0/0|5/0|0|0 / 5|
|1116 Draenor / 6941 Ashran **promoted**|5|1221 / yes|1/0|0/0|1/0|0/0|0/0|0/0|0/0|1/0|0|0 / 1|
|1116 Draenor / 7004 Frostwall **promoted**|5|1221 / yes|10/2|0/0|0/0|10/2|0/0|0/0|0/0|0/0|0|0 / 0|
|1116 Draenor / 7078 Lunarfall **promoted**|5|1221 / yes|195/109|0/0|44/3|151/106|0/0|0/0|0/0|44/3|3|0 / 44|
|1120 Thunder King Horde Hub / 0 unassigned|4|2 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1121 Thunder Island Alliance Hub / 0 unassigned|4|2 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1128 Mogu Island Events - Horde Base / 0 unassigned|4|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1129 Mogu Island Events - Alliance Base / 0 unassigned|4|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1132 Transport: The Skybag (Brawl'gar Arena) / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1133 Transport: Zandalari Ship (Mogu Island) / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1172 Transport: Siege of Orgrimmar (Alliance) / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1173 Transport: Siege of Orgrimmar (Horde) / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1179 Warcraft Heroes / 0 unassigned|5|36 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1181 PattyMack Test Garrison Bldg Map / 0 unassigned|0|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1190 Blasted Lands Phase / 0 unassigned|4|16 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1192 Transport: Gorgrond Train / 0 unassigned|5|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1212 Garrison - Alliance - Blacksmith 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1213 Garrison - Alliance - Alchemy 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1214 Garrison - Alliance - Enchanting 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1215 Garrison - Alliance - Engineering 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1216 Garrison - Alliance - Farm1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1217 Garrison - Alliance - Inscription 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1218 Garrison - Alliance - Jewelcrafting 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1219 Garrison - Alliance - Leatherworking 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1220 Broken Isles / 0 unassigned **promoted**|6|729 / yes|31/0|0/0|31/0|0/0|0/0|0/0|0/0|31/0|0|0 / 31|
|1220 Broken Isles / 7334 Azsuna **promoted**|6|729 / yes|716/142|0/0|616/134|100/8|0/0|0/0|0/0|616/134|125|0 / 616|
|1220 Broken Isles / 7502 Dalaran **promoted**|6|729 / yes|109/538|0/0|22/20|87/166|0/0|15/352|0/0|7/20|16|0 / 7|
|1220 Broken Isles / 7503 Highmountain **promoted**|6|729 / yes|0/1|0/0|0/1|0/0|0/0|0/0|0/0|0/1|1|0 / 0|
|1220 Broken Isles / 7541 Stormheim **promoted**|6|729 / yes|0/2|0/0|0/2|0/0|0/0|0/0|0/0|0/2|2|0 / 0|
|1220 Broken Isles / 7543 Broken Shore **promoted**|6|729 / yes|0/1|0/0|0/1|0/0|0/0|0/0|0/0|0/1|1|0 / 0|
|1220 Broken Isles / 7558 Val'sharah **promoted**|6|729 / yes|11/26|0/0|1/2|10/1|0/0|1/23|0/0|0/2|2|0 / 0|
|1220 Broken Isles / 7637 Suramar **promoted**|6|729 / yes|0/2|0/0|0/2|0/0|0/0|0/0|0/0|0/2|2|0 / 0|
|1221 Garrison - Alliance Mine 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1222 Garrison - Alliance Mine 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1223 Garrison - Alliance Mine 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1224 Garrison - Alliance Stable 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1225 Garrison - Alliance Stable 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1226 Garrison - Alliance Stable 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1227 Garrison - Alliance - Tailoring 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1229 Garrison - Alliance - Inn 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1230 Garrison - Alliance - Barn 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1231 Transport: Wavemurder Barge / 0 unassigned|5|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1247 Garrison - Alliance - Armory 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1248 Garrison - Alliance - Barracks 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1249 Garrison - Alliance - Engineering 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1250 Alliance - Garrison - Herb Garden 1 (UNUSED) / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1251 Alliance - Garrison - Inn 1 DONT USE / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1252 Garrison - Alliance - Lumbermill 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1253 Garrison - Alliance - Mage Tower 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1254 Garrison - Alliance - Sparring Arena 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1255 Garrison - Alliance - Salvage Yard 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1256 Garrison - Alliance - StoreHouse 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1257 Garrison - Alliance - Trading Post 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1258 Garrison - Alliance - Tailoring 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1259 Garrison - Alliance - Enchanting 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1260 Garrison - Alliance - Blacksmith 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1261 Garrison - Alliance - Plot Small / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1262 Garrison - Allilance - Plot Medium / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1263 Garrison - Alliance - Plot Large / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1265 Tanaan Jungle Intro / 7025 Tanaan Jungle **promoted**|5|36 / yes|134/52|0/0|27/2|107/50|0/0|0/0|0/0|27/2|0|0 / 27|
|1273 Nagrand - Garrison Camp - Stable2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1278 Garrison - Horde - Barracks 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1281 Garrison - Horde - Alchemy 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1282 Garrison - Horde - Armory 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1283 Garrison - Horde - Barn 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1284 Garrison - Horde - Blacksmith 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1285 Garrison - Horde - Enchanting 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1286 Garrison - Horde - Engineering 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1287 Garrison - Horde - Inn 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1288 Garrison - Horde - Inscription 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1289 Garrison - Horde - Jewelcrafting 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1290 Garrison - Horde - Leatherworking 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1291 Garrison - Horde - Lumbermill 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1292 Garrison - Horde - Mage Tower 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1293 Garrison - Horde - Mine 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1294 Garrison - Horde - Pet Stable 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1295 Garrison - Horde - Salvage Yard 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1296 Garrison - Horde - Sparring Arena 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1297 Garrison - Horde - Stable 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1298 Garrison - Horde Storehouse 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1299 Garrison - Horde - Tailoring 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1300 Garrison - Horde - Trading Post 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1301 Garrison - Horde - Workshop 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1302 Garrison - Alliance - Workshop 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1303 Garrison - Horde - Farm1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1304 Garrison - Horde - Plot Large / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1305 Garrison - Horde - Plot Medium / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1306 Garrison - Horde - Plot Small / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1307 Tanaan Jungle Intro - Forge Phase / 0 unassigned|5|2 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1308 Garrison - Horde - Fishing 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1309 Garrison - Alliance - Fishing 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1311 Outdoor Garrison - Arena - Horde (NEEDS WMO) / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1312 Outdoor Garrison - Arena - Alliance (NEEDS WMO) / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1313 Outdoor Garrison - Lumber Mill - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1314 Outdoor Garrison - Lumber Mill - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1315 Outdoor Garrison - Armory - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1316 Outdoor Garrison - Armory - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1317 Outdoor Garrison - Mage Tower - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1318 Outdoor Garrison - Mage Tower - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1319 Outdoor Garrison - Stables - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1320 Outdoor Garrison - Stables - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1321 Outdoor Garrison - Workshop - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1322 Outdoor Garrison - Workshop - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1323 Outdoor Garrison - Inn - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1324 Outdoor Garrison - Inn - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1325 Outdoor Garrison - Tradiing Post 1 - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1326 Outdoor Garrison - Tradiing Post - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1327 Outdoor Garrison - Construction Plot, Medium - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1328 Outdoor Garrison - Construction Plot, Medium - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1332 Garrison - Horde - Barracks 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1333 Garrison - Horde - Armory 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1334 Garrison - Horde - Barn 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1335 Garrison - Horde - Inn 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1336 Garrison - Horde - Lumbermill 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1337 Garrison - Horde - Mage Tower 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1338 Garrison - Horde - Pet Stable 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1339 Garrison - Horde - Stable 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1340 Garrison - Horde - Trading Post 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1341 Garrison - Horde - Workshop 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1342 Garrison - Horde - Barracks 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1343 Garrison - Horde - Armory 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1344 Garrison - Horde - Barn 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1345 Garrison - Horde - Inn 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1346 Garrison - Horde - Mage Tower 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1347 Garrison - Horde - Pet Stable 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1348 Garrison - Horde - Stable 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1349 Garrison - Horde - Trading Post 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1350 Garrison - Horde - Workshop 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1351 Garrison - Alliance - Large - Construction / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1352 Garrison - Alliance - Medium - Construction / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1353 Garrison - Horde - Large - Construction / 0 unassigned|5|0 / yes|11/6|0/0|0/0|0/0|11/6|0/0|0/0|0/0|0|0 / 0|
|1354 Garrison - Horde - Medium - Construction / 0 unassigned|5|0 / yes|11/5|0/0|0/0|0/0|11/5|0/0|0/0|0/0|0|0 / 0|
|1361 Garrison - Alliance - Mage Tower 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1362 Garrison - Alliance - Mage Tower 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1363 Garrison - Horde - Mine 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1364 Garrison - Horde - Mine 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1367 Garrison - Alliance - Workshop 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1368 Garrison - Alliance - Workshop 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1369 Garrison - Alliance - Lumbermill 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1370 Garrison - Alliance - Lumbermill 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1371 Garrison - Horde - Small - Construction / 0 unassigned|5|0 / yes|7/6|0/0|0/0|0/0|7/6|0/0|0/0|0/0|0|0 / 0|
|1372 Garrison - Alliance - Small - Construction / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1375 Garrison - Alliance - Alchemy 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1376 Garrison - Alliance - Alchemy 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1377 Garrison - Alliance - Blacksmith 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1378 Garrison - Alliance - Enchanting 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1379 Garrison - Alliance - Engineering 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1380 Garrison - Alliance - Inscription 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1381 Garrison - Alliance - Inscription 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1382 Garrison - Alliance - Jewelcrafting 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1383 Garrison - Alliance - Jewelcrafting 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1384 Garrison - Alliance - Leatherworking 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1385 Garrison - Alliance - Leatherworking 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1386 Garrison - Alliance - Tailoring 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1387 Garrison - Alliance - StoreHouse 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1388 Garrison - Alliance - StoreHouse 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1389 Garrison - Horde Storehouse 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1390 Garrison - Horde Storehouse 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1391 Garrison - Alliance - Salvage Yard 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1392 Garrison - Alliance - Salvage Yard 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1393 Garrison - Horde - Lumbermill 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1394 Garrison - Alliance - Sparring Arena 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1395 Garrison - Alliance - Sparring Arena 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1396 Garrison - Alliance - Trading Post 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1397 Garrison - Alliance - Trading Post 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1398 Garrison - Alliance - Barn 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1399 Garrison - Alliance - Barn 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1400 Garrison - Alliance - Inn 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1401 Garrison - Alliance - Inn 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1403 Garrison - Alliance - Barracks 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1404 Garrison - Alliance - Barracks 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1405 Garrison - Alliance - Armory 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1406 Garrison - Alliance - Armory 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1409 Garrison - Horde - Sparring Arena 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1410 Garrison - Horde - Sparring Arena 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1411 Garrison - Horde - Alchemy 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1412 Garrison - Horde - Alchemy 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1413 Garrison - Horde - Blacksmith 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1414 Garrison - Horde - Blacksmith 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1415 Garrison - Horde - Enchanting 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1416 Garrison - Horde - Enchanting 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1417 Garrison - Horde - Inscription 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1418 Garrison - Horde - Inscription 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1419 Garrison - Horde - Leatherworking 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1420 Garrison - Horde - Leatherworking 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1421 Garrison - Horde - Jewelcrafting 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1422 Garrison - Horde - Jewelcrafting 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1423 Garrison - Horde - Tailoring 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1424 Garrison - Horde - Tailoring 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1425 Garrison - Horde - Salvage Yard 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1426 Garrison - Horde - Salvage Yard 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1427 PattyMack Test Garrison Bld Map2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1429 Garrison - Horde - Engineering 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1430 Garrison - Horde - Engineering 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1432 Garrison - Horde - Fishing 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1433 Garrison - Horde - Fishing 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1434 Garrison - Alliance - Fishing 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1435 Garrison - Alliance - Fishing 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1437 Garrison - Alliance - Pet Stable 1 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1438 Garrison - Alliance - Pet Stable 2 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1439 Garrison - Alliance - Pet Stable 3 / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1440 Garrison - Alliance - Infirmary 1 / 0 unassigned|0|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1441 Garrison - Alliance - Infirmary 2 / 0 unassigned|0|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1442 Garrison - Alliance - Infirmary 3 / 0 unassigned|0|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1446 Outdoor Garrison - Construction Plot, Large - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1447 Outdoor Garrison - Construction Plot, Large - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1459 Transport: The Iron Mountain / 0 unassigned|5|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1462 Illidans Rock / 0 unassigned|6|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1463 Helhiem Exterior Area / 0 unassigned|6|12 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1464 Tanaan Jungle / 6723 Tanaan Jungle **promoted**|5|70 / yes|12/0|0/0|12/0|0/0|0/0|0/0|0/0|12/0|0|0 / 12|
|1465 Tanaan Jungle - No Hubs Phase / 0 unassigned|5|2 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1468 Warden Prison DH Quests / 0 unassigned|5|42 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1469 The Heart of Azeroth / 0 unassigned|3|25 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1471 Dungeon Test Map 1466 / 0 unassigned|0|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1473 Garrison - Alliance - Shipyard / 0 unassigned|5|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1474 Garrison - Horde - Shipyard / 0 unassigned|5|2 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1476 Transport: The Maw of Nashal / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1479 Skyhold / 7813 Skyhold **promoted**|6|25 / yes|125/15|0/0|0/0|125/15|0/0|0/0|0/0|0/0|0|0 / 0|
|1481 Mardum / 7705 Mardum, the Shattered Abyss **promoted**|5|110 / yes|919/33|0/0|490/27|429/6|0/0|0/0|0/0|490/27|19|0 / 490|
|1482 Garrison Shipyard - Submarine - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1483 Garrison Shipyard - Destroyer - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1484 Garrison Shipyard - Transport / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1485 Garrison Shipyard - Battleship - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1486 Garrison Shipyard - Carrier - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1487 Garrison Shipyard - Submarine - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1488 Garrison Shipyard - Destroyer - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1496 Garrison Shipyard - Carrier - Alliance / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1497 Garrison Shipyard - Battleship - Horde / 0 unassigned|5|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1502 Dalaran Underbelly / 8392 Dalaran Sewers **promoted**|6|9 / yes|0/84|0/0|0/0|0/0|0/0|0/84|0/0|0/0|0|0 / 0|
|1509 Bloodtotem Cavern - Fel Phase / 0 unassigned|6|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1510 Bloodtotem Cavern - Tauren Phase / 0 unassigned|6|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1512 Netherlight Temple / 0 unassigned|6|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1513 Hall of the Guardian / 0 unassigned|6|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1514 The Wandering Isle / 0 unassigned|6|90 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1515 Huln's War / 0 unassigned|6|20 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1517 Stormheim Prescenario Windrunner / 0 unassigned|6|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1518 Stormheim Prescenario Skyfire / 0 unassigned|6|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1519 The Fel Hammer / 0 unassigned|6|127 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1527 High Mountain Scenario / 0 unassigned|6|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1532 High Mountain Mesa Scenario / 0 unassigned|6|2 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1538 test / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1540 Emerald Dreamway / 0 unassigned|6|18 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1547 Artifact - Combat - Acquisition Ship / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1555 zzOLD - Transport: Boost Experience - Alliance Gunship / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1556 zzOLD - Transport: Boost Experience - Horde Gunship / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1559 7.0 Transport: Boost Experience - Horde Gunship / 0 unassigned|5|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1560 7.0 Transport: Boost Experience - Alliance Gunship / 0 unassigned|5|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1586 Assault on Stormwind / 0 unassigned|6|20 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1602 Artifact - The Vortex Pinnacle - Shaman Order Hall / 0 unassigned|6|25 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1604 Artifact - Order Campaign - Portal World Niskara / 0 unassigned|6|110 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1605 Firelands_Artifact / 0 unassigned|6|24 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1608 Hyjal War of the Ancients Quest / 0 unassigned|6|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1618 Death Knight Campaign - Scarlet Monastery / 0 unassigned|6|6 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1627 Fel Hammer DH Scenario / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1628 Transport: Alliance Battleship / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1637 Transport: Alliance Ship PhaseableMO / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1638 Transport: Horde Ship PhaseableMO / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1639 Transport: Kvaldir Ship PhaseableMO / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1642 Zandalar / 8499 Zuldazar **promoted**|7|981 / yes|2/89|0/0|1/4|1/0|0/0|1/85|0/0|0/4|4|0 / 0|
|1642 Zandalar / 8500 Nazmir **promoted**|7|981 / yes|1/1|0/0|0/1|1/0|0/0|0/0|0/0|0/1|1|0 / 0|
|1642 Zandalar / 8501 Vol'dun **promoted**|7|981 / yes|0/1|0/0|0/1|0/0|0/0|0/0|0/0|0/1|1|0 / 0|
|1643 Kul Tiras / 0 unassigned **promoted**|7|900 / yes|124/0|0/0|124/0|0/0|0/0|0/0|0/0|124/0|0|0 / 124|
|1643 Kul Tiras / 8567 Tiragarde Sound **promoted**|7|900 / yes|44/36|0/0|1/6|43/12|0/0|1/18|0/0|0/6|6|0 / 0|
|1643 Kul Tiras / 8717 Boralus Harbor **promoted**|7|900 / yes|87/323|0/0|74/277|13/3|0/0|0/43|0/0|74/277|268|0 / 74|
|1643 Kul Tiras / 8721 Drustvar **promoted**|7|900 / yes|1969/274|0/0|1861/250|108/24|0/0|0/0|0/0|1861/250|237|0 / 1861|
|1643 Kul Tiras / 9042 Stormsong Valley **promoted**|7|900 / yes|371/84|0/0|181/29|190/55|0/0|0/0|0/0|181/29|28|0 / 181|
|1643 Kul Tiras / 10015 Tol Dagor **promoted**|7|900 / yes|0/1|0/0|0/1|0/0|0/0|0/0|0/0|0/1|1|0 / 0|
|1644 Plunder Isle / 0 unassigned|6|144 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1647 Vol'jin's Funeral Pyre / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1649 Transport: Horde Battleship / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1650 Legion Command Ship Transport / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1657 zzOldBlade's Edge Arena / 0 unassigned|6|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1669 Argus / 8701 Eredath **promoted**|6|272 / yes|0/1|0/0|0/1|0/0|0/0|0/0|0/0|0/1|1|0 / 0|
|1670 Broken Shore (Delete) / 0 unassigned|6|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1671 Broken Shore - Prepatch 7.2 (Backup) / 0 unassigned|6|45 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1675 Winter AB_old / 0 unassigned|6|16 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1678 AB Phase 1 / 0 unassigned|6|16 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1679 AB Phase 2 / 0 unassigned|6|16 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1680 AB Phase 3 / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1682 The Fel Hammer Terrain Phase / 0 unassigned|6|77 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1692 Pit of Saron - Death Knight Campaign / 0 unassigned|6|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1696 Legion - Command Center / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1697 Legion - Legion Spider Cave / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1699 Legion - Legion Fel Cave / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1700 Legion - Legion Fel Firenova Area / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1701 Legion - Legion Barracks / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1711 Alliance Submarine (7.1.5 Boat Holiday) / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1716 Broken Shore Mage Tower / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1718 Nazjatar / 0 unassigned|7|220 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1741 The Vindicaar / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1750 Azuremyst Isle / 0 unassigned|6|20 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1751 Alliance Battleship (7.3 Intro) / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1752 Sunstrider Battleship (7.3 Intro) / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1773 Shadow of Azeroth / 0 unassigned|0|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1775 Maw of N'Zoth / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1776 Horde Battleship - The Banshee's Wail / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1779 Invasion Points / 0 unassigned|6|164 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1785 8.0 Islands - Ship - Pirateship / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1786 8.0 Islands - Zeppelin - Goblin / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1787 8.0 Islands - Micro - Mogu Crypt / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1795 Vale of Eternal Blossoms / 0 unassigned|7|24 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1806 Alliance Battleship -  Wind's Redemption / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1815 Silithus Phase / 0 unassigned|6|40 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1817 Silithus: The Wound / 0 unassigned|0|40 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1839 Karazan Chess / 0 unassigned|7|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1844 8.0 Islands - Event / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1845 8.0 Islands - Event / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1846 8.0 Islands - Ship - 6HU Cargoship / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1850 8.2.5 - MSEM - 1850 / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1851 8.0 Islands - Ship - 7FK Buccaneer / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1855 The Vindicaar / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1856 Transport: Azerite BG - Alliance Airship / 9136 Seething Shore|6|0 / no|28/3|0/0|0/0|0/0|0/0|0/0|0/0|28/3|0|0 / 0|
|1857 Transport: Azerite BG - Horde Gunship / 9136 Seething Shore|6|0 / no|21/3|0/0|0/0|0/0|0/0|0/0|0/0|21/3|0|0 / 0|
|1860 The Vindicaar / 9359 The Vindicaar **promoted**|6|2 / yes|62/26|0/0|53/17|9/9|0/0|0/0|0/0|53/17|16|0 / 53|
|1865 Telogrus Rift / 9415 Telogrus Rift **promoted**|6|16 / yes|70/46|0/0|70/46|0/0|0/0|0/0|0/0|70/46|45|0 / 70|
|1902 Silithus Alliance Gunship / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1903 Silithus Horde Gunship / 0 unassigned|6|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1909 [UNUSED] Transport: Kul Tiras - Bridgeport / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1910 Transport: Kul Tiras - Bridgeport - The Maiden's Virtue / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1913 PhaseableMO: Nazmir - Triangulation Platform / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1916 PhaseableMO: Nazmir - Ship World Quest / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1918 Kalimdor Darkshore Phase / 0 unassigned|7|192 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1919 8.0 Islands - Platform - KL Skywall Entrance Building 03 / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1920 8.0 Islands - Platform - MD Flamegate E / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1921 8.0 Islands - Ship - Player Alliance - Kul'Tiras Medium 01 With Gangplank / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1922 8.0 Islands - Ship - Player Horde - Zandalari Medium 01 With Gangplank / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1923 8.0 Islands - Platform - Deepholm Mercury Pool 01 / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1924 8.0 Islands - Platform - 7AZ Vashjir Small Cave A / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1926 8.1.5 Allies - Kul Tiran Flagship (JAK) / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1927 Zandalari Flagship / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1929 Chamber of Heart / 0 unassigned|7|10 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1930 Zandalar Alliance Intro Scenario / 0 unassigned|7|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1931 Lordaeron Blight / 0 unassigned|6|70 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1935 8.0 Islands - Cave - 8KUL_CaveMicro01 / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1936 8.0 Islands - Cave - 8KUL_CaveMicro02 / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1937 8.0 Islands - Cave - 8KUL_CaveMicro03 / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1938 8.0 Islands - Cave - 8RIV_CaveMicro_01 / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1939 8.0 Islands - Cave - 8RIV_CaveMicro_02 / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1940 Alliance Battleship -  Wind's Redemption Voldun / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1941 Transport: Nazmir Raid Elevator / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1942 Molten Core / 0 unassigned|7|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1945 Arathi Highlands 2 / 0 unassigned|7|30 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1951 8.0 Transport: Boost Experience - Alliance Gunship / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1952 8.0 Transport: Boost Experience - Horde Gunship / 0 unassigned|5|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1953 8.0 Islands - Platform - demigod dome living / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1954 The Great Sea Alliance / 0 unassigned|7|25 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1956 8.0 Islands - Platform - Hot Old God 03 / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1957 8.0 Islands - Platform - Old God / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1958 The Golden Skipper - Zandalari Transport Ship / 8499 Zuldazar|7|0 / no|4/0|0/0|0/0|0/0|0/0|0/0|0/0|4/0|0|0 / 0|
|1959 Zandalari Transport Ship Large / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1962 Goblin Ship Final / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1964 PhaseableMO: War Campaign - Horde - Chapter 05 - Kul Tiras Flagship / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|1965 PhaseableMO: War Campaign - Horde - Chapter 05 - Kul Tiras Ship / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2066 The Great Sea Horde / 0 unassigned|7|25 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2067 Zandalari Battleship - Gral's Call / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2069 Zuldazar Alliance Bombing Run / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2074 8.0 Transport: Boralus to Stormwind / 0 unassigned|7|0 / no|15/24|0/0|0/0|0/0|0/0|0/0|0/0|15/24|0|0 / 0|
|2075 8.0 Islands - Event / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2077 8.0 Islands - Event / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2078 8.0 Islands - Event / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2081 Blackrock Depths - Dark Iron Dwarf Hub / 10028 Blackrock Depths **promoted**|7|16 / yes|135/198|0/0|135/197|0/0|0/0|0/0|0/1|135/197|137|0 / 135|
|2082 8.0 Islands - Micro - Mogu Crypt / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2083 8.0 Islands - Micro - Mogu Crypt / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2084 8.0 Islands - Micro - Mogu Crypt / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2085 8.0 Islands - Micro - Mogu Crypt / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2086 8.0 Islands - Micro - Mogu Crypt / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2087 8.0 Islands - Micro - Mogu Crypt / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2088 8.0 Islands - Micro - Mogu Crypt / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2094 8.0 Transport: Boralus Canals / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2095 8.0 Transport: Boralus Sound / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2101 8.0 Transport: Boralus Barge / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2102 8.0 Transport: Boralus Ship Small / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2103 Darkshore Prepatch Darnassian Ship / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2109 8.1.0 Incursion Airship Horde / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2110 8.1.0 Incursion Airship Alliance / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2114 Maw Prototype / 0 unassigned|7|16 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2119 Transport: Cooking: Impossible - Hot Air Balloon - Alliance / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2120 Transport: Cooking: Impossible - Hot Air Balloon - Horde / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2123 Shrine of the Storm Quests / 0 unassigned|7|25 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2127 Siren Isle / 10416 Siren Isle **promoted**|10|25 / yes|236/24|0/0|236/24|0/0|0/0|0/0|0/0|236/24|23|0 / 236|
|2128 Dagger Realm / 0 unassigned|7|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2129 8.1 Darkshore Horde Quests / 0 unassigned|7|77 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2131 8.1 Darkshore Outdoor Final Phase / 0 unassigned|7|105 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2136 Transport: Zuldazar Raid Ship / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2145 Transport: Night Elf Warship / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2146 Transport: Forsaken Warship / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2147 Icecrown Citadel (8.1) / 0 unassigned|7|89 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2160 Maw Prototype - Dragonblight / 0 unassigned|0|20 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2165 SpawnedMO: Night Elf Warship / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2166 Transport: Zuldazar Raid Boralus Ship / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2175 9.0 NPE (New Player Experience) - Exile's Reach / 10424 Exile's Reach **promoted**|7|1202 / yes|900/29|0/0|441/18|459/11|0/0|0/0|0/0|441/18|17|0 / 441|
|2176 SpawnedMO: Forsaken Ship Large / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2186 The Obsidian Sanctum - 8.2 Magni - Heart of Azeroth / 0 unassigned|7|16 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2194 Hearthstone Tavern / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2195 Arcantina / 0 unassigned|11|225 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2205 9.0 Transport: Zone Transition Test / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|3075 Naigtal / 0 unassigned|11|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2215 Chamber of Heart - Repaired / 0 unassigned|7|10 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2222 The Shadowlands / 10413 Revendreth **promoted**|8|1155 / yes|15/2|0/0|9/2|6/0|0/0|0/0|0/0|9/2|2|0 / 9|
|2222 The Shadowlands / 10534 Bastion **promoted**|8|1155 / yes|614/47|0/0|596/47|18/0|0/0|0/0|0/0|596/47|35|0 / 596|
|2222 The Shadowlands / 10565 Oribos **promoted**|8|1155 / yes|237/142|0/0|195/142|42/0|0/0|0/0|0/0|195/142|116|0 / 195|
|2222 The Shadowlands / 11462 Maldraxxus **promoted**|8|1155 / yes|8/2|0/0|8/2|0/0|0/0|0/0|0/0|8/2|2|0 / 8|
|2222 The Shadowlands / 11510 Ardenweald **promoted**|8|1155 / yes|6/2|0/0|6/2|0/0|0/0|0/0|0/0|6/2|2|0 / 6|
|2222 The Shadowlands / 13672 Tazavesh, the Veiled Market **promoted**|8|1155 / yes|13/14|0/0|13/14|0/0|0/0|0/0|0/0|13/14|3|0 / 13|
|2237 8.2.5 Orgrimmar / 0 unassigned|7|36 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2241 Uldum / 10833 Uldum **promoted**|7|82 / yes|0/3|0/0|0/3|0/0|0/0|0/0|0/0|0/3|0|0 / 0|
|2246 Solesa Naksu [DNT] / 0 unassigned|7|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2261 9.0 Transport - NPE - Alliance Battleship / 10424 Exile's Reach|7|0 / no|53/2|0/0|0/0|30/0|0/0|0/0|0/0|23/2|0|0 / 0|
|2263 Crapopolis / 0 unassigned|7|16 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2267 city / 0 unassigned|7|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2268 Mechagon City / 12825 Mechagon City **promoted**|7|30 / yes|38/84|0/0|0/0|38/84|0/0|0/0|0/0|0/0|0|0 / 0|
|2274 Vision of the Twisting Sands / 0 unassigned|7|80 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2275 Vale of Eternal Twilight / 0 unassigned|7|24 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2276 9.0 Transport - Maldraxxus Necropolis [Transport] / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2277 9.0 SpawnedMO - Maldraxxus Necropolis [WMO] / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2297 Icecrown Citadel (8.3) / 12952 Icecrown Citadel **promoted**|7|89 / yes|33/16|0/0|33/16|0/0|0/0|0/0|0/0|33/16|1|0 / 33|
|2309 Transport: Torghast Bridge Platform01 Elevator / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2355 The Necrotic Wake (Transport) / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2364 The Maw / 0 unassigned|8|30 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2365 9.0 Maw - Torghast - 9Maw_Bridge_SmallPlatform01 / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2366 9.0 Maw - Torghast - 9Maw_Zone_WallTurret01 / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2367 9.0 Maw - Torghast - 9Maw_Bridge_Straight03 / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2368 Maldraxxus Broker Islands / 0 unassigned|8|27 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2369 9.0 NPE - Horde - Starter Transport Ship - (STM) / 10424 Exile's Reach|7|0 / no|45/0|0/0|0/0|24/0|0/0|0/0|0/0|21/0|0|0 / 0|
|2370 Maldraxxus Arena / 0 unassigned|8|4 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2374 Zereth Mortis / 13536 Zereth Mortis **promoted**|8|255 / yes|6/9|0/0|0/0|6/9|0/0|0/0|0/0|0/0|0|0 / 0|
|2439 De Other Side (Ardenweald Campaign) / 0 unassigned|8|40 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2444 Dragon Isles / 13644 The Waking Shores **promoted**|9|1634 / yes|41/86|0/0|41/0|0/2|0/0|0/84|0/0|41/0|0|0 / 41|
|2444 Dragon Isles / 13645 Ohn'ahran Plains **promoted**|9|1634 / yes|72/71|0/0|42/0|30/1|0/0|0/70|0/0|42/0|0|0 / 42|
|2444 Dragon Isles / 13646 The Azure Span **promoted**|9|1634 / yes|2523/225|0/0|2305/171|218/19|0/0|0/35|0/0|2305/171|139|0 / 2305|
|2444 Dragon Isles / 13647 Thaldraszus **promoted**|9|1634 / yes|7/29|0/0|5/0|2/1|0/0|0/28|0/0|5/0|0|0 / 5|
|2444 Dragon Isles / 13862 Valdrakken **promoted**|9|1634 / yes|79/380|0/0|49/51|30/2|0/0|1/327|0/0|48/51|5|0 / 48|
|2444 Dragon Isles / 14433 The Forbidden Reach **promoted**|9|1634 / yes|0/7|0/0|0/0|0/0|0/0|0/7|0/0|0/0|0|0 / 0|
|2444 Dragon Isles / 15105 Amirdrassil **promoted**|9|1634 / yes|65/10|0/0|65/10|0/0|0/0|0/0|0/0|65/10|9|0 / 65|
|2444 Dragon Isles / 15115 Bel'ameth **promoted**|9|1634 / yes|50/30|0/0|50/30|0/0|0/0|0/0|0/0|50/30|26|0 / 50|
|2453 Torghast / 0 unassigned|8|20 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2454 Zaralek Cavern / 14022 Zaralek Cavern **promoted**|9|435 / yes|347/15|0/0|347/1|0/0|0/0|0/14|0/0|347/1|1|0 / 347|
|2456 The Maw / 0 unassigned|8|180 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2469 Torghast / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2472 Skyhold / 0 unassigned|8|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2482 Font of Fealty / 0 unassigned|8|56 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2485 Torghast / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2486 Torghast / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2487 Torghast / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2488 Torghast / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2489 Torghast / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2490 Torghast / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2507 Void Zone: Airship / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2512 Grand Time Adventure / 0 unassigned|9|448 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2518 Old Beyfir / 0 unassigned|9|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2523 The Defiant Dragonscale / 0 unassigned|9|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2524 The Rugged Dragonscale / 1519 Stormwind City|9|0 / no|45/30|0/0|0/0|0/0|0/0|0/0|0/0|45/30|0|0 / 0|
|2529 Young Belvo / 0 unassigned|9|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2532 Ruins of Lordaeron / 0 unassigned|8|25 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2534 10.0 Dragon Isles -  Black Dragon Citadel - Narrative Version / 0 unassigned|9|15 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2548 Emerald Dream / 0 unassigned|9|578 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2552 Khaz Algar (Surface) / 14717 Isle of Dorn **promoted**|10|1023 / yes|2009/139|0/0|1720/119|289/6|0/0|0/14|0/0|1720/119|112|0 / 1720|
|2552 Khaz Algar (Surface) / 14771 Dornogal **promoted**|10|1023 / yes|1144/810|0/0|1128/509|16/6|0/0|0/295|0/0|1128/509|485|0 / 1128|
|2570 10.0 The Forbidden Reach (Child of 2444) / 13769 The Forbidden Reach **promoted**|8|131 / yes|49/39|0/0|49/39|0/0|0/0|0/0|0/0|49/39|26|0 / 49|
|2573 Lost Arvillo / 0 unassigned|9|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2576 Sully Test Space / 0 unassigned|9|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2601 Khaz Algar / 14752 Azj-Kahet **promoted**|10|855 / yes|5/34|0/0|5/0|0/0|0/0|0/34|0/0|5/0|0|0 / 5|
|2601 Khaz Algar / 14753 City of Threads **promoted**|10|855 / yes|1/22|0/0|1/0|0/0|0/0|0/22|0/0|1/0|0|0 / 1|
|2601 Khaz Algar / 14795 The Ringing Deeps **promoted**|10|855 / yes|18/28|0/0|18/0|0/0|0/0|0/28|0/0|18/0|0|0 / 18|
|2601 Khaz Algar / 14838 Hallowfall **promoted**|10|855 / yes|19/38|0/0|19/0|0/0|0/0|0/38|0/0|19/0|0|0 / 19|
|2616 Zskera Vaults / 0 unassigned|9|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2637 Dragon Isles / 0 unassigned|9|1155 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2666 Amirdrassil / 0 unassigned|9|54 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2668 10.1.7 Tyr Scenario - Black Empire / 0 unassigned|9|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2671 The Dawnbreaker / 14971 The Dawnbreaker|10|0 / no|21/44|0/0|0/0|0/0|21/44|0/0|0/0|0/0|0|0 / 0|
|2672 The Dawnbreaker / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2673 Ambush Airship A / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2674 Ambush Airship B / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2676 Ambush Airship C / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2677 Ambush Airship D / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2678 The Timeways (Dawn of the Infinite Outdoor) / 0 unassigned|9|64 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2694 Harandar / 15355 Harandar **promoted**|11|323 / yes|162/6|0/0|49/0|113/6|0/0|0/0|0/0|49/0|0|0 / 49|
|2698 11.0 - Chamber of Heart / 0 unassigned|0|10 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2703 11.0 Warband Screen / 0 unassigned|10|2 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2704 11.0 Warband Testing 2 / 0 unassigned|10|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2706 Undermine / 0 unassigned|10|90 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2708 10.2.5 Transport: Bel'ameth -> Gilneas / 15105 Amirdrassil|9|0 / no|14/0|0/0|0/0|0/0|0/0|0/0|0/0|14/0|0|0 / 0|
|2728 11.0.0 Transport: The Reckoning (Crashed Airship) / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2731 11.0 Dalaran Scenario / 0 unassigned|10|25 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2738 K'aresh / 15336 K'aresh **promoted**|10|4096 / yes|23/17|0/0|20/16|3/1|0/0|0/0|0/0|20/16|15|0 / 20|
|2738 K'aresh / 15781 Tazavesh, the Veiled Market **promoted**|10|4096 / yes|39/0|0/0|1/0|38/0|0/0|0/0|0/0|1/0|0|0 / 1|
|2739 Hall of Awakening / 0 unassigned|10|20 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2760 11.0.0 Transport: Arathi Airship 1 / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2761 11.0.0 Transport: Arathi Airship 2 / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2762 11.0.0 Transport: Arathi Airship 3 / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2763 11.0.0 Transport: Arathi Airship 4 / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2764 11.0.0 Transport: Arathi Airship 5 / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2771 Voidstorm / 15458 Voidstorm **promoted**|11|256 / yes|5/0|0/0|5/0|0/0|0/0|0/0|0/0|5/0|0|0 / 5|
|2780 11.0.0 Transport: Arathi Airship 7 / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2782 11.0 - Vault of Memory / 0 unassigned|10|9 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2785 Dragon Isles / 15509 The Forbidden Reach **promoted**|9|56 / yes|40/23|0/0|36/21|4/2|0/0|0/0|0/0|36/21|20|0 / 36|
|2786 Pocket Dimension / 0 unassigned|10|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2794 11.2.7 - The Wandering Isle / 0 unassigned|6|90 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2796 Arathi Highlands 3 / 0 unassigned|10|30 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2801 Ny'alotha, the Waking City / 0 unassigned|0|110 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2816 Delete me please / 0 unassigned|11|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2818 Lordaeron Garrison / 0 unassigned|0|15 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2820 Eastern Kingdoms - Copy / 0 unassigned|0|121 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2821 Bilge Rat Corsair A / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2822 Bilge Rat Corsair B / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2823 Bilge Rat Corsair C / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2829 The Dauntless / 0 unassigned|10|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2834 11.1 Warband - General / 0 unassigned|10|2 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2837 11.1 Warband - Delve Select Screen - Nightfall Sanctum / 0 unassigned|10|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2847 11.1 Warband - Freywold / 0 unassigned|10|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2851 11.1 Warband - The Gallagio / 0 unassigned|10|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2857 Scarlet Halls / 0 unassigned|11|15 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2858 12.0 Quel'Danas Scenario / 0 unassigned|11|25 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2860 Blood Elf Frigate (Scenario) / 0 unassigned|11|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2872 Undermine / 0 unassigned|8|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2916 Vaults of Atal'Utek / 0 unassigned|11|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2927 Arathi Highlands RPE / 0 unassigned|9|30 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2954 11.2 Warband - K'aresh / 0 unassigned|10|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2982 DELETE ME / 0 unassigned|11|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|2984 12.0.0 Transport: Ethereal Skiff / 0 unassigned|0|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|3007 The Escape from Durnholde - Lorewalking / 0 unassigned|11|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|3019 12.0 Quel'Danas Intro Scenario Ship Map / 0 unassigned|11|0 / no|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|3026 12.0 Warband - Founder's Point / 0 unassigned|11|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|3027 12.0 Warband - Razorwind Shores / 0 unassigned|11|1 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|3040 Null Space / 0 unassigned|11|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|3047 Val / 0 unassigned|11|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|3053 12.1 Warband - Silvermoon / 0 unassigned|11|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
|3099 12.1 Warband - Silvermoon - Void / 0 unassigned|11|0 / yes|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0/0|0|0 / 0|
