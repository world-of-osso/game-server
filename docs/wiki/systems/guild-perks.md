# Guild perks

[Contract](../../specs/guild-perks.md). Shared `load_guild_perks` reads active GuildPerkSpells, SpellName and SpellMisc exports; server startup requires the real catalog. This is not the Trading Post Perks tables or a fabricated reward list.

## Knowledge and membership

`KnownSpells` contains three vectors: baseline learned IDs, PvP replacement/base pairs, and membership-derived guild IDs. Its iterator combines them without duplicate IDs. Guild IDs are not written into professions or persisted class progression. Class/spec evaluation preserves the guild vector so an ordinary rebuild does not transiently unlearn perks.

`GuildPerksPlugin` observes changed membership/knowledge and removed membership components after Update mutations. It replaces only the guild vector and publishes actual owner SpellsLearned/SpellsUnlearned deltas through the existing TalentChannel. The existing known-spell passive-aura reconciliation consumes the combined knowledge; listing/learning alone does not prove a perk effect works.

## Proof boundary

Persisted invitation/quit, changed specialization, existing-member login, forced/offline removal and reopen controls pass. Actual private UDP invitation acceptance→officer removal→rejoin→quit produces exact perk learn/unlearn deltas while retaining class knowledge; the bounded catalog/membership/UI slice landed. Client native five-tooltip/chat/caret and identical-binary fresh-process owner readback also pass. Individual effects remain separate from these knowledge proofs.

## Supported Mount Up:172/211

Actual78633 carries nonstacking run172 and flight211 at10%. The movement accumulator takes the strongest positive nonstacking amount and chooses the larger of that multiplier and the existing stacking multiplier, then applies the mounted speed aura. Thus expert-riding Golden Gryphon32235 changes run2→2.2 and flight3.2→3.52; actual Crusader Aura32223 at20% wins with2.4/3.84 rather than multiplying both bonuses. These two modifiers do not enter the existing swim normalization path. Revoking the guild-derived spell removes its passive and restores the ordinary mounted rates without unmounting.

Passive synchronization also observes actual InMap changes. Required installed MapStore metadata interprets Attr8 NOT_IN_BATTLEGROUND and Attr4 NOT_IN_ARENA_OR_RATED_BATTLEGROUND for the supported battleground/arena map types. Knowledge is retained; only an ineligible passive application is removed, then restored when the character returns to an allowed map. No missing-map fallback or spell-ID override is added.

Aura210 is **explicitly blocked**, per lead decision: TC Unit.cpp8833–8844 applies it to player-controlled NPC flight, not the mounted player. The local authored vehicle path is ground IoC Demolishers, where the perk is excluded by battleground rules. No flying vehicle or extra mounted multiplier is fabricated.

Real-data RED33ec603a recorded run2 instead of2.2; REDc0f608da recorded the passive surviving a move to installed WSG489. GREEN3/3 at6f27ebd6 covers the rates, Crusader nonstacking and WSG489/NagrandArena559→continent0 restoration. The earlier fixture-only missing SpellStore78633 failure is not a product RED. A functional both-skin native run measured real replicated run14→15.4→14 and flight22.4→24.64→22.4 through actual invitation acceptance/removal; final living/flat collector `alive-flat-1792192848` confirms seven coherent mounted/motion/speed samples in each skin. Fourteen full-HD originals inspected; ten selected mounted ground/flight/removal images published, with four wall/tree-occluded removed-ground/unmounted images rejected. Numeric unmounted/revoked-ground controls remain valid. The earlier cliff trial persisted fall death; actual32235/CasterDead receipts explained later mount failures, and private fixture setup now uses existing `.revive` authority without perk/speed injection.

Sources: pinned TC Unit.cpp8812–8854, SpellInfo.cpp2218, Spell.cpp7158 and Player.cpp26509; actual installed SpellEffect and Map rows.

## Hasty Hearth charge boundary

Installed8690 uses category2309 (one charge,1800000ms). Guild passive83944 has453/-900000ms for that category. TC SpellHistory.cpp982–995 adds the flat modifier when consuming a charge; SpellAuraEffects.cpp525 gives453 no immediate handler. Existing new-cast behavior already produced30/15/30-minute receipts for nonmember/member/removed controls. RED at7770d9c5 exposed a different defect: joining changed an already-consumed charge to898750ms remaining instead of preserving its roughly30-minute clock.

Actual Human native item6948 use exposed458619 (No Place Like Home):411/+1 charge and453/-300000ms, yielding two charges with25-minute nonmember recovery. Guild83944 makes new uses10 minutes; removing the racial to fit a30-minute fixture would be invalid. The real Human mixed-use test reproduced a second defect: after a25-minute ordinary use and a queued guild use, the second recovery was still25 minutes instead of10.

`ChargeState.pending` is now a VecDeque of `ChargeRecovery { recovery_ms, ready_at }`, selected at each actual consumption as in TC SpellHistory.cpp834–853. Each entry retains its flat-adjusted base before changeable rates. Flat453 changes future uses without changing consumed entries. Rate changes recompute from each stored base and preserve pending progress, avoiding accumulated integer-millisecond truncation; scripted reductions shift deadlines, and restoration removes the newest missing entries. Owner snapshots report the oldest pending entry, switching duration when that entry recovers; no wire extension is added. At4cb11c77,17/17 charge controls pass: four Hasty cases (including real Human mixed25/10/25-minute consumption), ten rate controls and three owner snapshot controls. First queue implementation reproduced two1ms rate-restoration truncations; recomputing each entry from its recorded base fixes both without changing expectations. Separate haste/ControlDream/script-restoration controls and final checks remain pending. Actual identical-binary Human item6948/8690 native proof passes both skins: two25-minute ordinary uses and two10-minute guild uses, current1→0/max2, four real invitation/removal cycles, exact114 non-guild IDs conserved. Both25/10-minute pending clocks survive guild changes without retiming. Twelve inspected full-HD originals show actual bag/item/cast context; numerical clocks are native owner receipts rather than rendered labels. Fresh processes separate consumption scenarios, not a pending-clock restart-persistence proof. Ledger: `/home/osso/.worktrees/logs/guild-hasty-hearth-2026-10-16-run1/proof-ledger.json`.

## Mobile Banking user policy — 2026-10-17

User decision2026-10-17 establishes MOBILE_BANKING_COOLDOWN_2H,7200000ms rather than installed70448 parent recovery3600000ms. The shared constant drives server cooldown start and client tooltip. Script83958 dispatches installed88304/88306 by established player faction; each child runs ordinary effect50 using installed206602/206603, display/bounds, destination and300000ms duration. Summoned type34 now uses the existing guild-bank interaction/runtime; it requires current membership plus CharacterReputation1168 Friendly on USE, and closes when eligibility disappears or the object expires. Static vault eligibility is unchanged. Existing bank permissions and storage remain authoritative. Neutral guild members may cast: no RoleplayCore Friendly CAST gate is adopted.

Behavioral RED: ordinary neutral-member cast succeeded but spawned zero chests; stale known spell allowed a nonmember cast; actual UseGameObject opened the mobile guild-bank role for an unguilded Exalted character; tooltip rendered1hour rather than user2hours. Initial AffectingCombat failures were fixture preconditions (generic combat harness defaults to combat), corrected before these production REDs. Initial server4/4 and tooltip1/1 GREEN. A separate real midcast-removal RED required rechecking membership at cast completion before cooldown/dispatch; final server5/5 GREEN after that change. First native Modern cast/chest, member bank, actual removal/nonmember refusal, rejoin cooldown and2hour tooltip passed. Five-minute expiry failed at330s: generic summoned lifetimes used capped virtual ticks. A301s-real/250ms-virtual-clamp regression reproduced this boundary. Summoned expiry now uses the existing real elapsed clock, matching TC GameObject.cpp1403–1404/2286 GameTime; authored durations stay unchanged. Final server6/6,18shared-object controls and independent425/425b/455e consumer gates pass; all code landed. [Native proof boundary](#native-mobile-banking-proof) remains distinct from those gates.

## Native Mobile Banking proof

Actual Alliance casts, rendered206602, member guild-bank contents, actual removal/physical nonmember refusal and rejoin cooldown refusal pass in both skins. Both skins observe approximately301s despawn from the authored300000ms. The final Forever source-event-anchored receipt is300435ms. Backend controls additionally prove Neutral-member casting,2999/3000 Friendly-use threshold, Horde206603 and the exact real-time clamp boundary; native earned reputation or native Horde appearance is not claimed.

After the Modern-origin cooldown, a cast following the same continuous client's switch to Forever was accepted: server timestamps1792264549753→1792271860976, gap7311223ms, same owner entity. The final Forever-only case uses1792289747363→1792296990063, gap7242700ms, same owner entity. Neither interval restarts the server/client or modifies clocks. Actual before-deadline requests are refused; the subsequent accepted casts establish readiness. Final native acceptance is complete in both skins, accepted as the retained Modern case plus the completed Forever case rather than relabeling an interrupted overall run.

Accepted-cast intervals use actual received CastSuccess timestamps. The final Forever lifetime observer maps that Unix timestamp to wall/monotonic clocks on the same host. Local float32 UI interpolation is not the timing oracle. Officer participants exist only for membership actions, then exit during long waits. Friendly3000 is an explicit private fixture prerequisite, not earned reputation. No production timeout changes, shortened source durations or cooldown resets are used.

Evidence: `/home/osso/.worktrees/logs/guild-mobile-banking-2026-10-17/proof-ledger.json`; Modern `phase-owned-officers-real-clock-1792264405`, completed Forever `quiet-owner-forever-1792289597`, identical extension/server hashes. Earlier transport/measurement-interrupted trial results remain unchanged and classified separately. Fifteen individually inspected, unchanged1920×1080 originals are published as `{modern,forever}-mobile-banking-*.png` under `/syncthing/AgentShared/2026-10-15/guild-completion/`; `final-native-proof.json` records each SHA256/source path. No pending-clock persistence across restart or UI countdown accuracy is claimed. Owned runtimes stopped and the owner-checked quiet window was cleared.

## Mobile and resurrection authority audit — 2026-10-17

Historical source inspection before the user decision above, not effect acceptance. Its Mobile policy gaps are superseded by that decision; implementation/native proof is tracked above. User clarification2026-10-17 excludes resurrection guild perks from World of Osso; Quick/Dead is not a pending gap.

| Boundary | Established evidence | Remaining gap |
|---|---|---|
| Reputation representation | Shared `CharacterReputation` stores faction values; authenticated `reputation_factions.rs` includes Guild1168 atNeutral0. TC a352b1fa `CriteriaHandler.cpp:2069` reads faction1168 directly. | Resolved by the user policy above: member/perk casting versus Friendly chest use. Do not substitute membership, rank or default Friendly for standing. |
| Mobile data | Active83958: script77 without trigger,3600000ms recovery. Child88304/88306: actual206602/206603, type34,300000ms duration. | User decision above authorizes faction dispatch; installed effect50/type34 runtime supplies the lifetime. |
| Contemporary fork candidate | RoleplayCore8604a936f3eaceb5034eb9e082e78640bf39fad6 `spell_generic.cpp:5920–5957` checks Friendly1168 at cast and dispatches Alliance88304/Horde88306. SQL binding was retrieved, not merely inferred from a search snippet. | User decision explicitly rejects the fork's Friendly CAST gate; only chest USERS require Friendly. |

RoleplayCore's March11,2026 commit9fc5c2e586c1fcf9ea3ebc6a5fce4ca85c5bcba2 added this script/binding and replaced an older hard-coded Alliance-only dispatch. That history proves fork authorship, not Blizzard correctness. The fork is not evidence that pinned TC implemented the missing effect.

Executed retrievals, pinned bytes/hashes and repository metadata: `/home/osso/.worktrees/logs/guild-perk-source-audit-2026-10-17/data/`. Raw source provenance identifies repository, commit and file; this directory is diagnostic evidence, not a runtime dependency. Earlier suggestions that a new guild-scoped reputation provider was required are superseded by the actual field representation above.

## Sources

Installed Mainline GuildPerks.lua/XML; active GuildPerkSpells rows2/7/10/14/16. TrinityCore a352b1fa Guild.cpp2322 grants the perk store's spells on guild login;3002 removes them on leaving/removal.
