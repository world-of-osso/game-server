# Toy Box server foundation

Account-wide toy learning/favourites are separate from the legacy per-character mount/pet collection. Toy identity comes from locally extracted Retail Toy.db2, not item names or invented lists. [Contract](../../specs/toys.md).

## Generic summon dependencies and source interaction markers (2026-10-21)

[Contract](../../specs/toys.md#generic-summon-dependency-closure): original206 effect28 blockers partition into151 missing-template toys and55 unsupported-control/title toys; no missing SummonProperties. The producer only imported one aura428 hop plus audited scenery. It now resolves every Toy on-use normal-difficulty TriggerSpell closure with cycle-safe UNION, preserving authentic TDB templates/models/addons; no world spawns or fabricated identities.

Global source markers keep interactions refused: NPC flags, spellclick, SmartAI, VehicleId, gameobject summon/use effects in a summoned toy's closure, and explicit existing toyfx contracts. Blank source script names never prove scriptlessness. Sole exemption is Katy156833/261602's exact mailbox flag, independently exercised by `toyfx66_katy_real_learning_summon_mailbox_service_and_source_expiry`; additional/drifted markers still refuse. Previously admitted46780 Ogre Pinata,118191 Vargoth and188694 Florist Pouch become named gaps (unproved SmartAI death/spawn actions or gossip).

Producer RED1→GREEN4 covers direct51337/recursive60383 source equality, idempotency, retained rows and rollback. Marker RED→GREEN3 includes actual186501 trophy use with both179325/180947 actors, source models/scales/owner/map/destination and natural60s expiry/item cooldown. Retained13-case census/mailbox scope initially had11PASS/two old-contract failures; preserving the five explicit toyfx11/toyfx18 dummy/seat/wormhole/invisible-display contracts gives four-case GREEN including both failures, updated census and canonical census. Earlier unaffected11 proofs remain scoped, not rerun.

Prepared-copy pipeline admission423→506/1173:86 added,three corrected removals. Baseline before both batches421→506 net+85. **Canonical without publication is420**, not506. Maintained importer inserts206 authentic templates/280 models/one addon, zero removals/updates to existing content; lead owns publication. Original151 cohort initially has13 marked/138 marker-free toys; five further explicit existing contracts stay refused. Four source/data gaps remain behind effect28:260074/265250/271783 absent,52697 still omitted by legacy identity selection;55 controls remain unimplemented. Template admission is not complete interaction/native parity. Evidence and exact per-item matrix: engine `data/diagnostics/toys-census-2026-10-21/`, including census.json,by-cause.md,summon-delta.json and publication manifest.

## Census-driven generic force casts (2026-10-21)

[Contract](../../specs/toys.md#census-driven-generic-effect160): the current Full1166+seven70448 catalog has421→423/1173 pipeline-admissible toys after generic effect160 (`FORCE_CAST_2`) dispatch. TC a352b1fa `SpellEffects.cpp:851–892` queues a normal cast by the hit unit at the original caster, after the parent SpellGo. Reuse the existing pending-cast queue, but preserve normal cast time, readiness and CC checks rather than effect64's costed-trigger policy. No per-toy script or dummy whitelist added.

Behavioral RED2/one control precedes implementation81721548d. Corrected native Hitching Post fixture starts outside combat (the shared combat harness otherwise rightly refuses peaceful mounting). Current49150da7f GREEN3 covers native reversed ownership/source1.5s cast, actual stun refusal and real Spore-Bound Essence203852 learning/use→source2s child402636→display88121→natural5min restoration with item cooldown retained. Nine existing native effect64 queue/order/cost/target controls pass. Census adds only140336 Brulfist Idol and203852, zero removals; Brulfist movement and native presentation remain unproved at this checkpoint.

Evidence: engine `data/diagnostics/toys-census-2026-10-21/{census.json,by-cause.md,force160-delta.json}` plus RED/GREEN/regression logs. Graphs preserve every raw all-difficulty DB2 TriggerSpell reference separately from runtime blocker classification; a script-owned/static reference is not automatically an executing edge. The752→750 remaining blockers include script/dummy, summon definitions/control, targets and other handlers; no full1166 lifecycle/native parity claim.

## Natural shared hearthstone charge coverage (2026-10-18)

[Current proof scope](../../toy-effects14-coverage.md#natural-shared-hearthstone-charge-coverage-green-first-execution-2026-10-18), [contract](../../specs/toys.md#natural-shared-hearthstone-charge-coverage): tt653211/0/check/fmt landed stationary timer coveragea989c0f3b. Final toy test/docs slice covers281136/281615 in both directions: ordinary30min shared2309 recovery allows sibling10s cast, spends its only charge and blocks original. Observed completion bounds/normal50ms updates, no private deadline/reset/clock edits. First execution7adfb7b03 GREEN4/0/78.45s covers both new directions and both retained initial/missing-bind controls. Existing machinery works; no runtime repair/fabricated RED. Lead integration/landing pending; accepted sourced boundary/gaps/native exclusions unchanged.

## Ordinary30-minute/hour coverage follow-up (2026-10-18)

[Current proof scope](../../toy-effects14-coverage.md#ordinary30-minutehour-coverage-follow-up-green-first-execution-2026-10-18), [contract](../../specs/toys.md#ordinary-lifetime-and-reuse-follow-up): accepted boundary docs landedec7df3660. Two existing stationary identities280820/280822 now have ordinary50ms-update tests for natural30min expiry and1h recovery/reuse using observed cast bounds, no deadline/cooldown/time-scale edits. First executione10b9562b GREEN4/0/79.19s includes both new lifecycles and existing same-ID cancellation controls. Timers already work; no behavioral RED/runtime repair was invented. Lead integration/landing pending. Other identities/native visuals and accepted three unpublished gaps remain unchanged.

## Accepted sourced boundary and three named gaps (2026-10-18)

[Authoritative source/code/asset matrix](../../toy-effects14-coverage.md#accepted-sourced-boundary-full1166-plus-seven-2026-10-18), [contract](../../specs/toys.md#accepted-sourced-boundary): approved current boundary complete at Full1166+seven published70448 extras. Batch3 landed69ed3c682 after ty639209/0/check0/fmt0; feature branch cleaned. Aqir275039 `AQIR_SCRIPT_BINDING`, Libram284159 `LIBRAM_LOOT189_REWARD_BINDING` and Quill286643 `QUILL_LABYRINTH_EXIT_ACTION` remain unpublished named gaps, not false-success cast admissions. Item143823 is not a sourced Glyph reward; Quill's real92 criterion does not establish its party exit/teleport action. Native visuals and full-duration reuse remain separate follow-ups; earlier pending stages below are historical.

## Expanded70448 footstep cosmetics (2026-10-18)

[Current source/publication/RED matrix](../../toy-effects14-coverage.md#expanded70448-footstep-cosmetics-canonical-cpu-green-2026-10-18): batch2 landed641cb2460 and its feature branch was deleted. Trail of Halazzi280821 and Jan'alai's Flame280825 actual learning RED0/2/54.36s precedes lead's exact twoToy-row/40file publication; canonical aura4 admission RED0/2/30.71s. Only two named IDs enter existing strict visual-only machinery, with their census triples appended beside five published extras. Canonical1e5e55ae0 GREEN22/0/76.90s covers pair/all13 census/five prior toys/retained2513 case/absent-visual negative. Lead broad gate/source landing pending; no inferred native footsteps/particles, generic dummy or scripted-gap change.

## Expanded70448 stationary Loa cosmetics (2026-10-18)

[Current source/RED/publication matrix](../../toy-effects14-coverage.md#expanded70448-stationary-cosmetics-canonical-cpu-green-2026-10-18): on rewrittenac36572bb, actual bag-learning RED0/3/78.01s→lead three-row publication→canonical aura4 admission RED0/3/59.41s. Only three named zero-payload cosmetics enter the existing strict whitelist; their published triples extend the census extras list to five while keeping Full1166 separate. Lead reproduced/published34 local-CASC files; direct row/SHA readback agrees, native remains unproved. Canonical81c554bb1 targeted20/0/151.90s covers three new cosmetics/all13 census/three batch1 cases/absent-visual negative. Broad toy/check/fmt gate and source landing remain lead-owned; no generic dummy admission or other-five change.

## Published extras and census cardinality (2026-10-18)

[Current regression/correction](../../toy-effects14-coverage.md#tg578-census-regression-named-baseline-and-published-extras-2026-10-18): tg578's12 census failures and st574's additional `toys_effect_census` share one stale1166 oracle after the two-row publication. Keep Full1166 as the baseline and append only named published70448 item/name/spell triples, asserting both identities and resulting count. Exact748a5ad78 merge preserves batch1; intentionally RED batch2 tests remain isolated. Thirteen census plus three batch1 targets pending; no runtime/data change or gate success claim.

## mc548c landing and expanded70448 hearthstone admission (2026-10-18)

[Current landing and batch1 matrix](../../toy-effects14-coverage.md#mc548c-maxcount-landed-expanded70448-hearthstone-batch-green-2026-10-18): lead confirms coordinated MaxCount landing and merged-engine1604/0; earlier pending labels below are historical. Expanded scope starts with two70448 hearthstones whose dependencies already exist but Toy membership is missing. Selected producer3808fe58f preserves unrelated rows; authentic copied-world diff has only twoToy INSERTs. Producer RED0/2→GREEN2/0; actual learned-use CPU RED0/2 before lead publication→canonical GREEN3/0/71.90s including retained three-item2513 case. Lead published the independently reproduced two-row world fragment and16 source-authenticated audio files; direct canonical rows/SHA readback agrees. Broad tg/toy gate, source landing and native proof remain pending. Historical Full1166 denominator and5efec9c6f families preserved.

## Post-guild coordinated merge target proof (2026-10-18)

[Current exact merge/codec/behavior matrix](../../toy-effects14-coverage.md#post-guild-merge-codec-and-bounded-maxcount-proof-green-2026-10-18): landed guild masters merged into MaxCount; protocola2b07a6 passes2 wire+2 discriminant targets, stable prior tags/new21+13; server33c9d2eb2 passes17/0 including all five regressions and MaxCount controls; engine6c00abbf4 dispatch1/0. Both engine wiki-index entries retained. Final broad mc548b/check/fmt, coordinated landing/native/full1166 remain lead-owned/pending.

## mc548b five-regression recovery targeted GREEN (2026-10-18)

[Current five-case proof/limits](../../toy-effects14-coverage.md#mc548b-five-regression-recovery-targeted-green-rerun-pending-2026-10-18), [root-cause matrix](inventory.md#mc548b-regression-recovery--targeted-only-2026-10-18): exactcfdfc3ee6/protocol999b52e reproduction0/5→f97c118e4 GREEN5/0/25.95s. Three failures were competing test-harness error consumers; two production fixes retain authentic quest metadata and avoid inventory work for empty grants. All original expected outcomes remain unchanged; real creation caps/typed errors/source identity remain enforced. The exact three-item Saptor/Idol/Badge cancellation/recovery test individually passed mc548b; this does not make that RED aggregate green. Lead combined rerun/landing/native/full1166 proof remains pending; no own broad rerun.

## Gate548 MaxCount repair targeted GREEN; coordinated gate pending (2026-10-18)

[Current Toy proof/limits](../../toy-effects14-coverage.md#gate548-maxcount-defect-coordinated-repair-targeted-green-integration-pending-2026-10-18), [inventory mechanism/source/proof](inventory.md#ownership-maxcount--implementation-and-targeted-proof-2026-10-18): genuine548 duplicate-product defect now has sourceMaxCount/shared-store/passive-bank/quest-bypass/typed-cast repair. Targeted model5/0, quest3/0, Doomsday4/0, shared-wire1/0 and native-account CPU-dispatch1/0. Real ProfessionData requires regular second-use refusal/typed ItemMaxCount; separate triggered-effect and genuine-full controls retain their distinct semantics. Three-item2513 batch retained/compiled but behavior-unexecuted. Lead combined three-repo gate, landing/native/full1166 remain pending; no source data/weights edit or own broad rerun.

## Three-item shared2513 cancellation/recovery batch pending (2026-10-18)

[Exact batch/limits](../../toy-effects14-coverage.md#three-item-batch-saptor685-idol702-badge710-cancellationrecovery-pending-after548-2026-10-18):604c35311 adds actual Saptor685/Idol702/Badge710 learned cancellation/restoration/no2513 bypass/ordinary600s Pango reuse. Source pools and weights untouched; prior fixtures unchanged. One future gate for three identities, uncompiled/unexecuted AFTER548 with format/manual evidence only. No Masquerade cancellation/hour-expiry/exact-wall/native/distribution certificate or runtime/data edit.

## Doomsday419 ordinary recovery/product cap acceptance pending (2026-10-18)

[Exact acceptance/limits](../../toy-effects14-coverage.md#doomsday419-ordinary-recovery-and-unique-product-repeat-pending-after543-2026-10-18):cb46a33f9 keeps the real first202310 product while learned202309 refuses near295s and completes again after300 ordinary seconds; source MaxCount1 must preserve original GUID/quantity rather than duplicate/replace. TC `SpellEffects.cpp:1426–1440` bounds created quantity, not necessarily root-cast admission. Old first-product/Offering fixtures unchanged. Format/manual only, uncompiled/unexecuted AFTER543; no product-use/native/bank/account-cap certificate or runtime/data/clock/inventory mutation.

## Gate543 Masquerade697 landed (2026-10-18)

[Exact proof/landing](../../toy-effects14-coverage.md#gate543-green-and-landed-masquerade697-cast-boundary-2026-10-18): lead receipt/ancestry confirmsa8b96f8d9 on master3accdf1a1;543server318/0/1ignored635.32s/EXIT0/FMT0/CHECK0 with exact697 test OK. Prior pending labels are historical; exact timing/interruption/hour-expiry/native/all-five/full1166 limits persist.

## Masquerade697 authored cast timing acceptance pending (2026-10-18)

[Exact acceptance/limits](../../toy-effects14-coverage.md#masquerade697-authored-cast-prepost-completion-acceptance-pending-after541-2026-10-18):35168319d extends existing276207/1298957 with learned-use pre/post source600ms cast boundary: no early SpellGo/appearance/category recovery, then145134/misc267626,8/one-hour aura/shared2513 and Idol refusal. Prior fixtures unchanged. Format/manual only; uncompiled/unexecuted AFTER541. No new CancelCast/runtime/data/protocol or native/exact-timing/hour-expiry/all-five claim.

## Gate541 Singed593 and Pango684 landed (2026-10-18)

[Current gate/landing proof](../../toy-effects14-coverage.md#gate541-green-and-landed-singed593-repair-plus-pango684-lifecycle-2026-10-18): explicit lead receipt/ancestry confirmsa6f8505f7 on server master312f5aebd;541server317/0/1ignored469.98s/EXIT0/FMT0/CHECK0 with exact593/Pango tests OK.535's genuine runtime admission failure repaired via named Singed slot, not blanketdummy. Earlier pending labels below are historical; native/full1166/all-five/hour-expiry/distribution limits persist.

## Singed593 and Pango684 owned CPU verification GREEN (2026-10-18)

[Exact proof/limits](../../toy-effects14-coverage.md#singed593-repair-and-pango684-cpu-verification-green-successor-landing-pending-2026-10-18): runtime5ea7efcaa at6b54512a0, canonical shared11a058cf; real535RED→focused5931/0/25.16s, own toys197/0/337.82s includes593/Pango684 and preserved Citizens/generic dummy refusals. Workspace all-targets check/fmt pass. Minimal audited Singed slot only; no blanketdummy or10186 edit.593/Pango earlier pending labels are historical; lead successor/landing receipt and native/full1166 proof remain absent.

## Gate535 RED and narrow Singed admission repair pending (2026-10-18)

[Exact failure/repair scope](../../toy-effects14-coverage.md#gate535-genuine-red-narrow-singed-admission-repair-pending-verification-2026-10-18): actual593 refused child dummy4;535server315/1/1ignored/EXIT101/FMT0/CHECK0, not fixture failure or landing. Authorized5ea7efcaa admits only audited1220867/slot0 through existing zero-payload visual-aura guard. TC `SpellAuraEffects.cpp:76,4836–5117` has spell-specific dummy handlers; generic dummy admission would falsely accept unresolved scripts. Citizens/Personal Anchor refusal retained. Own focused compile/test running; no GREEN/native certificate yet, Pango remains stacked.

## Pango684 cancellation and shared recovery pending (2026-10-18)

[Exact acceptance and limits](../../toy-effects14-coverage.md#pango684-cancellation-and-ordinary-shared-recovery-pending-after535-2026-10-18): test/spec223d0916c extends existing Pango identity with real CancelAura restoration/no shared2513 bypass, refusal near580 simulated seconds and learned Saptor success after600 ordinary seconds. No deadline/cooldown mutation; prior shared-five fixture unchanged. Format/manual only, uncompiled/unexecuted AFTER535; no all-five lifecycle, wall-duration/hour-expiry/native/weighted-frequency certificate or runtime/data edit.

## Mycomancer664 landed after532; Explosive593 queued535 (2026-10-18)

[Current proof and landing](../../toy-effects14-coverage.md#mycomancer664-gate532-passed-and-landed-explosive593-queued535-2026-10-18): feature8af454985 landed server masterd0e0d7fc3 per lead receipt and ancestry.532 server315/0/1ignored673.31s/EXIT0/FMT0/CHECK0 includes exact Mycomancer learned-root CPU test; native/cross-map/missing-bind/natural-recharge limits persist. Explosive38ced31bd is separately queued535, not covered by532. Earlier pending labels below are source-stage history.

## Explosive593 mixed hearthstone and Singed acceptance pending (2026-10-18)

[Exact acceptance and limits](../../toy-effects14-coverage.md#explosive593-mixed-hearthstone-and-singed-cpu-acceptance-pending-after532-2026-10-18): test/specbb325c6a4 preserves actual236687/root1220729 ten-second252 plus queued self64→1220867/Singed dummy4/5000ms, real learned bind/shared2309 and ordinary child expiry. Native explosion/Singed art remains unverified; no invented damage or simple-hearth substitution.25 direct cases/helper and Mycomancer unchanged. Format/manual only; uncompiled/unexecuted after532; candidatefb2bd5a88 is not landing. No runtime/data/native edit or own heavy job.

## Mycomancer664 triggered hearthstone acceptance pending (2026-10-18)

[Exact acceptance and limits](../../toy-effects14-coverage.md#mycomancer664-triggered-hearthstone-acceptance-pending-after-st526-2026-10-18): test/spec37a6c0598 learns264367 and uses actual ten-second root1299014→64→instant1286909/252, checking saved-bind return, root charge2309, unchanged health/model and Corewarden refusal. Separate fixture leaves25 direct-hearth cases/helper untouched. Format/manual readability only; uncompiled/unexecuted, lead successor afterst526 required. No missing-bind/cross-map/native/natural-recharge certificate or runtime/data edit.

## st526 Toy landing (2026-10-18)

[Landing and limits](../../toy-effects14-coverage.md#st526-toy-scope-landed-2026-10-18): lead-confirmed server master `ce0fec81c1caeaa20ff8d47b572cde1e49afbd65` contains feature38d087230 and docsfaab75f7e, independently confirmed by ancestry. Covered successor scope complete with194 Toy CPU passes/0 failures; eight server failures belong to other heads. Earlier pending labels below are historical for covered tests. Native/all1166 gameplay acceptance and whole-workspace green remain unclaimed.

## st526 Toy CPU results without whole-workspace success (2026-10-18)

[Exact current proof matrix](../../toy-effects14-coverage.md#st526-server-result-toy-cpu-acceptance-passes-full-workspace-remains-red-2026-10-18): candidate2ec2e80bc/feature38d087230/fix31f7e9216 compiled; server3909/8/124ignored/2770.80s/EXIT101/FMT0. Toy namespace194 passed/0 failed, including Royal misc0, shared-five baseline, old19/Oomgut,21 casters,13 objects,17 stationary and25 hearthstones. Akunda cursor/cooking/lifetime and drift controls pass with assigned queue fix. Seven missing-table startup/orientation failures plus Slagmaw damage failure keep the workspace RED; engine result pending. No own runtime/data/test job, native/full1166 gameplay or new landing certificate. Earlier pending labels retain source-stage history; this linked matrix governs current proof limits.

## Royal Visage618 misc0 acceptance pending after513 (2026-10-18)

[Exact pending acceptance](../../toy-effects14-coverage.md#royal-visage618-misc0-acceptance-pending-execution-after513-2026-10-18):0305a0e91/ce32419fb tests245631/1239480/236348/misc0/display121918 through real bag learning/use, exact ten-minute deadline, ordinary near-expiry/restoration/aura removal/health and thirty-minute recovery refusal. Old19 misc8 registry/helper, Oomgut and shared-five fixture unchanged. Format/manual readability/diffcheck pass; [st526 CPU result](#st526-toy-cpu-results-without-whole-workspace-success-2026-10-18) observes this exact test OK, with full server RED elsewhere and no local test job. No native/full-reuse/quest/selection certificate or runtime/data change; queued64 repair remains assigned elsewhere.

## Shared2513 five-identity learned-use baseline pending after513 (2026-10-18)

[Exact new baseline matrix](../../toy-effects14-coverage.md#shared2513-five-identity-baseline-new-acceptance-pending-after513-2026-10-18) adds acceptance-only1c143bea6 for Pango/Saptor/Masquerade/Idol/Badge: real learned source model/one-hour aura deadline/health, ten-minute2513 report and learned-sibling refusal without cast/model/deadline mutation. Old19 long-recovery registry/helper and Oomgut remain unchanged. Format/manual readability/diffcheck pass; [st526 CPU result](#st526-toy-cpu-results-without-whole-workspace-success-2026-10-18) observes this exact five-case baseline OK, not whole-workspace success. No cancellation/hour expiry/natural reuse/weighted-frequency/native certificate; mixed Memento excluded, no runtime/data/source-only admission change. Assigned owner retains queued64 repair.

## Full1166 metadata availability without failure inference (2026-10-18)

[Exact availability matrix](../../toy-effects14-coverage.md#full1166-metadata-availability-exact-absences-not-failure-verdicts-2026-10-18) checks all1166 runtime IDs against active70448:29 identical source-identity/runtime-content absences,15 without TriggerType0 roots, two named roots without SpellMisc plus unnamed264368's fully absent spell metadata, zero dangling ItemEffect links. Missing metadata is not automatically a code/import failure or permission for another trigger/default mechanic; retained Rezan behavior is not revoked by absent SpellMisc. No source-only admission/import/runtime change or behavior proof from this inventory; [current integration status](#st526-toy-cpu-results-without-whole-workspace-success-2026-10-18) remains RED.

## Gate513 queued-trigger destination regression (2026-10-18)

[Exact gate matrix](../../toy-effects14-coverage.md#gate513-acceptance-helpers-pass-queued-trigger-destination-regression-keeps-integration-red-2026-10-18): server311/1/1ignored/914.05s/EXIT101, client9/FMT0. All then-covered acceptance helpers completed OK, but Akunda's actual child279077 fire spawned at caster instead of parent cursor. Integrated Effect64's new queue drops parent `cast_destination`, retaining only item level before fresh-child reconstruction; old destination-preserving handler is bypassed. Explicit destination must survive queue/delivery, not a weaker fixture or caster fallback. Assigned fix31f7e9216 subsequently passes the exact Akunda case in [st526](#st526-toy-cpu-results-without-whole-workspace-success-2026-10-18); this thread made no runtime/data/native change. Whole workspace remains RED elsewhere, without native/full1166 completion or new landing certificate.

## Tail metadata reconciliation and unchanged Full1166 boundary (2026-10-18)

[Exact proof matrix and excluded names](../../toy-effects14-coverage.md#tail-reconciliation-and-source-only-names715720-metadata-proof-not-runtime-completion-2026-10-18) reconciles all151 authorized IDs after checkpoint559/228966 across31 active70448 source capsules, with no missing/unexpected IDs or provenance/order mismatches. Source1176/runtime1166 difference remains the same ten named extras, none admitted/imported/accepted; final six names are boundary-only. Toy hash and active symlink agree with checkpoint559. This establishes tail metadata accounting, not whole1166 review completeness or runtime behavior. [Gate513 status](#gate513-queued-trigger-destination-regression-2026-10-18) records the failed integration separately from passing individual helpers; no new toy landing or own heavy duplicate. Mixed/native gaps remain open.

## Last Full1166 IDs, weighted badge, Windwell and excluded source-only names (2026-10-18)

[Raw source710–714 matrix](../../toy-effects14-coverage.md#source710714-last-full1166-ids-weighted-badge-windwell-and-three-excluded-source-only-names-2026-10-18) reaches the last two Full1166 IDs280419/280541: Badge retains weights4/2/1/2/1 and one-hour appearance/shared2513 ten-minute recovery; missing Windwell271783 retains base0/both87 targets. Later Loa-Blessed Victory280820, Trail of Halazzi280821 and Nalorakk's Strength Charm280822 are named SOURCE-ONLY/OUTSIDE1166, with no import/admission/acceptance. Metadata-tail inspection is not runtime completion or landing. [Gate513 status](#gate513-queued-trigger-destination-regression-2026-10-18) supersedes queue-only status after separate master compile repair efd627da1; no new toy landing. No new acceptance/runtime/data/native change or own heavy work.

## Mixed Memento, mask, qualified swim, retained Venomous banner and story-view boundaries (2026-10-18)

[Source705–709 matrix](../../toy-effects14-coverage.md#source705709-mixed-memento-mask-qualified-swim-retained-venomous-banner-and-story-view-boundaries-2026-10-18) preserves Memento56+370/shared2513, unaudited Amani Mask, water/8993-qualified swim241+58/base1200+56 alongside descriptive300%, existing Venomous Banner146432/60s fixture and Storytime260/2565/7. No dropped370, invented curse, normalized speed formula, native banner or generic story attachment certificate. Metadata only; no runtime/data/native change or own heavy jobs, lead successor pending.

## Unnamed homebind metadata, current-camp recall, shared Idol, corpse haunt and targeted star boundaries (2026-10-18)

[Source700–704 matrix](../../toy-effects14-coverage.md#source700704-unnamed-homebind-metadata-current-camp-recall-shared-idol-corpse-haunt-and-targeted-star-boundaries-2026-10-18) preserves unnamed276370/1299513 homebind source with missing item identity, actual current-camp9076 recall, Idol shared2513 short recovery, corpse226/15000ms/trigger0 haunting and Otoola's targeted32→dummy4 badge. No fabricated name, fixed/homebind recall, long-recovery transform certificate, inert haunt or self-only star substitute. Metadata only; no runtime/data/native change or own heavy work, lead successor pending.

## Pending Corrosive Victory and mixed Effigy, short-category mask, stand, companion-command boundaries (2026-10-18)

[Source695–699 matrix](../../toy-effects14-coverage.md#source695699-pending-corrosive-victory-and-mixed-effigy-short-category-mask-stand-companion-command-boundaries-2026-10-18) adds acceptance-onlyad2f092f7:275988/1298433/30-minute aura via unchanged caster helper, now21 identities/old20 retained; behavior awaits lead successor. Effigy retains1250 trigger payload plus transform/200ms area chain; NPC scale0.6 is not player scale. Mask retains one-hour appearance/shared2513 ten-minute recovery; Stand retains47/138/base0; Companion Crystal retains150/critter kill authority. No flattened child, stationary-only stand or combat-pet shortcut. No runtime/data/native change or own heavy work.

## Pending Pearl disguise, throne, loot-bot, kill-rain and excluded source-only identity (2026-10-18)

[Raw source690–694 matrix](../../toy-effects14-coverage.md#source690694-pending-pearl-disguise-throne-loot-bot-kill-rain-and-one-excluded-source-only-identity-2026-10-18) adds acceptance-only83c3de3d2 for Full1166 Pearl274921/1295623/265732/ten displays/5min/1hr:19 disguise rows plus separate Oomgut20 identities, old18/helper unchanged; behavior pending lead successor. This raw block has four runtime identities plus named275039 Curse of the Aqir, explicitly SOURCE-ONLY/OUTSIDE Full1166 with no admission/import/acceptance. Missing target87/base0 Throne260074, resource-gated corpse loot bot and kill-proc enemy-player rain remain gaps. No static throne/free loot/inert rain substitute; no runtime/data/native change or own heavy jobs.

## Shared Saptor category, missing nap/tent, retained Umbral banner and scripted starfish boundaries (2026-10-18)

[Source685–689 matrix](../../toy-effects14-coverage.md#source685689-shared-saptor-category-missing-naptent-retained-umbral-banner-and-scripted-starfish-boundaries-2026-10-18) preserves Saptor's shared2513 ten-minute recovery/one-hour aura and122805 weight0.1, missing Nap Mat263791, existing Umbral Banner143906/60s fixture with explicit time-leap scope, missing target22/base0 Tent265250 and Starfish77 plus permanent child4. No equalized models, target87 tent, chair/fishing bonus or inert badge substitute. Metadata only; no runtime/data/native change or own heavy jobs, lead successor pending.

## Mixed props, staff image, camera guides and short-recovery Pango boundaries (2026-10-18)

[Source680–684 matrix](../../toy-effects14-coverage.md#source680684-mixed-props-staff-image-camera-guides-and-short-recovery-pango-boundaries-2026-10-18) preserves missing Hourglass/Bench mixed16 payloads and distinct aura/cancel contracts, source-instant missing original-owner staff image, camera293/60/321 with both mutual164 guide removals, and Pango's one-hour transform versus category2513 ten-minute recovery. Existing five Pango models establish metadata, not learned/restoration/category behavior; long-recovery helper cannot certify this timing without changing its contract. No runtime unsupported/RED claim, invented channel flag or generic camera substitute. No runtime/data/native change or own heavy jobs.

## Black-hole game, key inventory, rainbow chain, retained pillow and dual hologram boundaries (2026-10-18)

[Source675–679 matrix](../../toy-effects14-coverage.md#source675679-black-hole-game-key-inventory-rainbow-chain-retained-pillow-and-dual-hologram-boundaries-2026-10-18) preserves actual430 feeding game, all-shard inventory conversion, mixed rainbow241/56misc12/42/409 with proc2000ms and both164 removals, retained11-second Pillow fixture and both missing hologram259655/259656/base0/stacking contracts. No automatic win/free key, pure disguise, recast parachute, one static gnome or guessed holiday recovery. Metadata only; no runtime/data/native change or own heavy work, lead successor pending.

## Pending Corewarden hearthstone and corpse ritual, mail, viewer, current-location travel boundaries (2026-10-18)

[Source670–674 matrix](../../toy-effects14-coverage.md#source670674-pending-corewarden-hearthstone-and-corpse-ritual-mail-viewer-current-location-travel-boundaries-2026-10-18) adds acceptance-onlyf30ff0f2c:actual slot1 265100/1273401/source10s/shared2309 blocks Preyseeker263933 through unchanged helper, now25 hearthstones/old24 retained; behavior awaits lead successor. Corpse25 ritual/-100 scale/two9000 payloads, missing mailbox257683/base0/shared2066, Brann viewer233/base550/9049/251 and qualified current-Abundant3+77 travel remain gaps. No self-disguise, inert rift, stat buff or guessed destination substitute; no runtime/data/native change or own heavy work.

## Pending Dominating Victory and missing metadata, qualified flare, corpse flag/rod boundaries (2026-10-18)

[Source665–669 matrix](../../toy-effects14-coverage.md#source665669-pending-dominating-victory-and-missing-metadata-qualified-flare-corpse-flagrod-boundaries-2026-10-18) adds acceptance-only9470e4859:264413/1271973/30-minute aura via unchanged caster helper, now20 identities/old19 retained; behavior awaits lead successor. Unnamed264368/1271857 lacks item/spell metadata; missing257541 flare retains32/64/32 chain, distinct area groups and base0; corpse flag22/93 and Rod25/mixed395/shrink/two5000 payloads/permanent corpse transform remain gaps. No substituted metadata, static helper or caster disguise. No runtime/data/native change or own heavy work.

## Pending Preyseeker hearthstone and minipet leash, tap-state, mixed mushroom, triggered-homebind boundaries (2026-10-18)

[Source660–664 matrix](../../toy-effects14-coverage.md#source660664-pending-preyseeker-hearthstone-and-minipet-leash-tap-state-mixed-mushroom-triggered-homebind-boundaries-2026-10-18) adds acceptance-only3b3f52940:263933/1270814/source10s/shared2309 blocks Naaru263489 via unchanged bind helper, now24 hearthstones/old23 retained; behavior awaits lead successor. Permanent target90 leash, once/twice lens state, missing642496+42098 mixed76/179 mushroom and Mycomancer actual10s root1299014/64→instant252 child1286909 remain distinct gaps. No caster-only leash, fabricated tap window, ordinary mushroom summon or direct-child hearthstone substitution. No runtime/data/native change or own heavy jobs.

## Pending Naaru hearthstone and skip-rock, mixed mushroom, retained catch, periodic voidwalker boundaries (2026-10-18)

[Source655–659 matrix](../../toy-effects14-coverage.md#source655659-pending-naaru-hearthstone-and-skip-rock-mixed-mushroom-retained-catch-periodic-voidwalker-boundaries-2026-10-18) adds acceptance-only8ead527f5:263489/1270583/source10s/shared2309 blocks Lightcalled257736 through unchanged persisted-bind helper, now23 hearthstones/old22 retained; behavior awaits lead successor. Skip-rock nonzero trigger/base1, qualified50+179 mushroom with missing404325/40621, retained3300ms Valdekar fixture and100ms/trigger0/three-voidwalker contract remain distinct. No guessed count/impulse/body or inert periodic aura substitute; no runtime/data/native change or own heavy work.

## Mixed parachute, cycle-script, retained wine, photo and ash boundaries (2026-10-18)

[Source650–654 matrix](../../toy-effects14-coverage.md#source650654-mixed-parachute-cycle-script-retained-wine-photo-and-ash-boundaries-2026-10-18) preserves parachute's five root effects/negative98 payload/area8537/category1470, three distinct Cycle script base points, existing Wine fixture, nonzero-trigger proc Photo Finisher and both Ashen Horn140 edges including35/395 child. No global slow-fall, fabricated spirit, screenshot substitute or normal64 flattening. Metadata only; no new acceptance/runtime/data/native proof or own heavy work, lead successor pending after cancelled509.

## Pending Lightcalled hearthstone and jumpable-monument, gold-fountain boundaries (2026-10-18)

[Source645–649 matrix](../../toy-effects14-coverage.md#source645649-pending-lightcalled-hearthstone-and-jumpable-monument-gold-fountain-boundaries-2026-10-18) adds acceptance-only57aa5b0c5:257736/1261979/source10s/shared2309 blocks Cosmic246565 via unchanged saved-bind helper, now22 hearthstones/old21 retained; behavior awaits lead successor. Missing247940/254652/254655 monuments require actual shared jump collision with distinct casts/base0, not ordinary scenery. Missing255051 fountain retains mixed64→425841/effect65 and actual gold-offering authority; no guessed cost/reward. No runtime/data/native change or own heavy jobs.

## Pending Dredger disguise and linked-companion, mixed406 boundaries (2026-10-18)

[Source640–644 matrix](../../toy-effects14-coverage.md#source640644-pending-dredger-disguise-and-linked-companion-mixed406-boundaries-2026-10-18) adds acceptance-only2cc3d9ec3 for256893/1260864/254071/four exact displays/10-minute appearance/15-minute recovery:18 disguise rows plus separate Oomgut19 identities, old17/helper unchanged; behavior awaits lead successor. Exodar's two495→1258382 edges and linked428 chain, missing traveling cat253791, Rutaani406/misc20/target0 and Steward406/misc294/self remain distinct gaps. No static companion or pure-transform substitute for mixed406. No runtime/data/native change or own heavy work.

## Pending Potatoad disguise and pet-command, bundle, egg, Arcantina boundaries (2026-10-18)

[Source635–639 matrix](../../toy-effects14-coverage.md#source635639-pending-potatoad-disguise-and-pet-command-bundle-egg-arcantina-boundaries-2026-10-18) adds acceptance-onlye29972e08 for252265/1254376/251887/five exact source displays/10-minute appearance/20-minute recovery:17 disguise rows plus separate Oomgut18 identities, old16/helper unchanged. Behavior pending lead successor;509 cancelled before start for unrelated master compile interaction. Exact vanity-pet90/87 move/stay, missing bundle251069/base0/target32, unaudited Potatoad Egg and two distinct Arcantina script base-point payloads remain gaps. No combat-pet substitute, assumed summon count or invented travel destination; no runtime/data/native change.

## Broker disguise, shared grafts, mixed stall and updraft boundaries (2026-10-18)

[Source630–634 matrix](../../toy-effects14-coverage.md#source630634-broker-disguise-shared-grafts-mixed-stall-and-updraft-boundaries-2026-10-18) preserves Cartel Xy transform56/misc0 without invented creature, distinct Void/Light marks sharing category2433/50-minute recovery, missing outdoor stall249056 plus effect164→carried-crate removal, and full updraft105/104/106/506 movement contract. No zero-dummy cosmetic admission, independent graft timer, static stall or flight substitute. Metadata only; lead509 queued without result/landing, no own heavy work.

## Goggles, distinct crates, wormhole and strategy-game boundaries (2026-10-18)

[Source625–629 matrix](../../toy-effects14-coverage.md#source625629-goggles-distinct-carried-crates-wormhole-and-strategy-game-boundaries-2026-10-18) preserves goggles233/3569/247 viewer presentation, distinct carried-crate3393/3392 poses, missing wormhole actor248783 with actual random Midnight travel, and permanent strategy-game430/3707 plus separate two-second298624 trigger. Metadata only; no generic aura, static portal or automatic-win substitute. Gate507 failed only Cold Ones234951 at the five-yard fixture cursor; source/runtime range169 is0–3 yards. Test-only172cb1782 uses a nonzero in-range cursor, other12 rows/assertions unchanged. Lead509 queued at that revision; no behavioral success or landing inferred.

## Pending POST/Cosmic hearthstones and balloon/scaled-spire/phase-guise boundaries (2026-10-17)

[Source620–624 matrix](../../toy-effects14-coverage.md#source620624-pending-postcosmic-hearthstones-and-balloonscaled-spirephase-guise-boundaries-2026-10-17) adds acceptance-onlyd981a866d:POST245970/1240219 and actual slot1 Cosmic246565/1242509 via unchanged source10s/saved-bind/shared2309 helper, now21 identities/old19 retained; behavior AFTER507. Missing controlled slot6 Brewer balloon246042 and actual Phase Diver state remain gaps; existing Lightning Spire display131280/scale0.5 fixture retained. No mail service from Title5, shortened POST cast, substituted Cosmic slot0, invented phase immunity or native/recharge certificate. No runtime/data/new native or own heavy duplicate.

## Pending Sea-Blessed Shrine and kill-proc/memory/rolling/Royal Visage boundaries (2026-10-17)

[Source615–619 matrix](../../toy-effects14-coverage.md#source615619-pending-sea-blessed-shrine-and-kill-procscripted-memoryrolling-snowballroyal-visage-boundaries-2026-10-17) adds acceptance-only4a0133a68:245942/1240082/246037/display129958/cursor/180s as17th stationary actor, old16/helper unchanged; behavior AFTER507. Existing explicit time-leap expiry is not ordinary50ms/wall proof. Kill-proc42/trigger0 voice, script-only K'areshi appearance, missing controlled snowball245763/holiday recovery and actual Royal Visage misc0/display121918 remain gaps; no inert proc, guessed appearance, stationary rolling prop or borrowed misc8 certificate. No runtime/data/new native or own heavy duplicate.

## Pending Etheric Victory and portal/void/snowman/transmorpher boundaries (2026-10-17)

[Source610–614 matrix](../../toy-effects14-coverage.md#source610614-pending-etheric-victory-and-portalvoidsnowmanmixed-transmorpher-boundaries-2026-10-17) adds acceptance-onlyb7df42b22:244470/1237584/1800s as19th audited caster identity, old18/helper unchanged; behavior AFTER507. Missing any-player Dornogal portal244139, unaudited Void pose, mixed snowman293/321/27/56/holiday timer and mixed Brann245374/65/77 service remain gaps. NPC scale2.2 is not player scale; description-reference1237986 is not a root trigger. No runtime/data/new native or own heavy duplicate.

## Courier-mail/finger-ribbon/outdoor-balloon/skiff/pennant boundaries (2026-10-17)

[Source605–609 matrix](../../toy-effects14-coverage.md#source605609-courier-mailfinger-ribbonoutdoor-balloonskiffpennant-boundaries-2026-10-17) preserves missing actual Courier Lynx242136 mail/category2066, finger-ribbon art, outdoor riding balloon243246, one-hour skiff104+250ms226/jump paddling and nonzero Astral pennant gaps. No inert courier/balloon, gameplay ring, flat130% speed or copied sibling art.507 runtime still lacks final result/landing;603 thirteenth object remains newer. No runtime/data/native proof or own heavy duplicate.

## Pending Dastardly Banner and crate/juggling/scaled-banner-podium boundaries (2026-10-17)

[Source600–604 matrix](../../toy-effects14-coverage.md#source600604-pending-dastardly-banner-and-cratejugglingretained-scaled-banner-podium-boundaries-2026-10-17) adds acceptance-only3eabc6183:239007/1227030/527088/type5/display102616/size1/cursor/180s as13th object, old12/helper unchanged; behavior awaits head AFTER507. Distinct carried crate3185 and exact juggling removal203/reapplication64/mixed293/406 remain gaps. Existing Flame's Radiance scale0.75 case and authentic grounded static Podium20min CPU/historical native-source receipts retained, not new wall-duration/recharge/qualification proof. No runtime/data/new native or own heavy duplicate.

## Flarendo/fountain and paired bobber overrides (2026-10-17)

[Source595–599 matrix](../../toy-effects14-coverage.md#source595599-flarendofountain-and-paired-fishing-bobber-override-boundaries-2026-10-17) retains exact Flarendo pose/fountain36404 and three paired258 fishing overrides: Rocket520304/520305, Goblet520306/520311, Wellington520308/520309 against35591, each referencing131474. Only replacement520305 exists at runtime, type17/display101984/size2; no generic35591 art substitute, deduplicated pair or two immediate Fishing casts. Actual bobber selection/fishing/cleanup remains gap; no catch bonus from flavor.507 queued, no new proof/runtime/data/native change or own heavy duplicate.

## Ally-return/paddle/outdoor-stall/mixed-homebind/Torq-pose boundaries (2026-10-17)

[Source590–594 matrix](../../toy-effects14-coverage.md#source590594-ally-returnpaddleoutdoor-stallmixed-homebindtorq-pose-boundaries-2026-10-17) preserves friendly-player sawblade return, raft waterwalk plus250ms periodic/jump paddling, missing controlled outdoor stall237883, homebind252 plus unaudited explosive child1220867 and Torq's5000ms pose. No caster projectile substitute, fixed130% buff, generic vendor, child-dropping twentieth hearthstone or invented photo-file output.507 queued at76a8426bf; no new proof/runtime/data/native change or own heavy duplicate.

## Prized pennant and four missing cartel banners (2026-10-17)

[Source585–589 matrix](../../toy-effects14-coverage.md#source585589-nonzero-prized-pennant-and-four-missing-cartel-banners-2026-10-17) retains nonzero permanent Prized pennant and exact Steamwheedle508828/Bilgewater508826/Blackwater508827/VentureCo508830 cursor-banner gaps. All four runtime objects absent; explicit shared item category1246/180s is not four independent timers. No guessed object art/type/scale, per-use cartel eligibility or sibling substitution.507 queued at76a8426bf; no result, runtime/data/native change or own heavy duplicate.

## Shard/permanent poses/repeated jump proc/tea boundaries and505 type correction (2026-10-17)

[Source580–584 matrix](../../toy-effects14-coverage.md#source580584-mixed-shardpermanent-posesrepeated-jump-procinteractive-tea-boundaries-2026-10-17) preserves no-template transform plus scale, permanent circlet/reading presentation, both Hat42→1216102 jump edges and missing interactive tea506612. No guessed model/scale, ring stats, spawned desk, deduplicated proc or Tea149865 copy.505 stopped at compile E0277/EXIT101;76a8426bf fixes only registry creature typeu32→i32, all16 rows/assertions unchanged, format/manual readability/diffcheck pass. Next lead proof must include577 twelfth object; no runtime result, data/native change or own heavy duplicate.

## Pending Cold Ones rack and mixed chair/device/crate/homebind boundaries (2026-10-17)

[Source575–579 matrix](../../toy-effects14-coverage.md#source575579-pending-cold-ones-rack-acceptance-and-mixed-chairdevicecratehomebind-boundaries-2026-10-17) adds acceptance-only0e4c5ed9b:234951/1215364/505415/type22/display97591/scale0.75/actual cursor/180s as12th object, old11/helper unchanged; format/manual readability/diffcheck pass, behavior needs head AFTER505. Rack's Data0 can aura is not automatic trigger/item yield. Missing controlled chair232549/mixed aura, step-on goblination area36000, distinct carried crate3141 and homebind252 plus cancellation298 remain gaps; no ordinary chair/self transform/20th simple hearthstone substitution. Lead505 queued through Golem573; no result/landing inferred or own heavy duplicate.

## Pending Golem self-transform and banners/unnamed-season/carried-crate boundaries (2026-10-17)

[Source570–574 matrix](../../toy-effects14-coverage.md#source570574-pending-golem-self-transform-and-retained-bannersunnamed-seasoncarried-crate-boundaries-2026-10-17) adds acceptance-only06d44ef20:233202/473407/234479/six valid models/instant30min/1h recovery as16th disguise plus Oomgut17IDs; old15/helper unchanged, behavior awaits lead covering head. Existing Prized/Astral source-banner cases retained;232308/233151 remains missing item/model metadata with literal `[Season]`. Hallowfall self293/misc3118/1 carried-crate appearance remains gap, not inventory supply yield. No separate possession actor inferred from Golem self56, runtime/data/native proof or own heavy duplicate.

## Seasonal banners and unnamed season-four metadata gap (2026-10-17)

[Source565–569 matrix](../../toy-effects14-coverage.md#source565569-existing-seasonal-banners-and-unnamed-season-four-metadata-gap-2026-10-17) retains existing Tempered/Prized/Unbound/Forged exact display/owner/map/faction0/front2yd/stationary60s behavior cases.232304 remains a Toy identity with literal `[Season]` description but missing70448 ItemSparse/runtime item/233147 creature-model; no guessed seasonal name or sibling art. All source roots use property64/600s recovery. No new pass/native/reward-eligibility/recharge certificate, runtime/data or own heavy duplicate.

## Retained balloons/victory and random-delve/recipient-spotlight/local-disguise boundaries (2026-10-17)

[Source560–564 matrix](../../toy-effects14-coverage.md#source560564-retained-balloonsvictory-and-random-delverecipient-spotlightlocal-disguise-boundaries-2026-10-17) records exact local70448 metadata. Balloon source actor and Explosive Victory caster cases already exist; no duplicate fixture or expanded caster count. Random Bountiful Delve travel/completion reset, actual target57 party-member Spotlight and Undermine-only aura243/misc35 consequences remain gaps, not static bot/self aura/body transform. Source232267 model absent;231372 retains both model weights/scales.498 remains landed; no new behavioral/native proof, runtime/data or own heavy duplicate.

## Trial/bling/observer-spiders/qualified-return-home/screen boundaries (2026-10-17)

[Source555–559 matrix](../../toy-effects14-coverage.md#source555559-trialblingobserver-spidersqualified-return-homescreen-boundaries-2026-10-17) preserves missing avoid-hit Trial actor, unaudited permanent Coldflame attachment, observer-only233/7825/240 spider perception, qualified Threads homebind and actual2243 screen presentation. Threads description requires qualification while item faction/reputation/on-use condition fields are0; no invented threshold/account scope/per-use gate or generic Human-helper full certificate. No global NPC/screen mutation, inert trial or flavor-derived veteran rule.498 landed/master8707ac0ea; no further runtime/data/new native/own heavy duplicate.

## Pending Arachnoserum acceptance and pennant/linked-carry/stone-metadata boundaries (2026-10-17)

[Source550–554 matrix](../../toy-effects14-coverage.md#source550554-pending-arachnoserum-acceptance-and-pennantlinked-carrystone-metadata-boundaries-2026-10-17) adds acceptance-onlyc3fe0958c:228705/462934/229772/sixteen valid models/instant10min/1h recovery as15th disguise plus Oomgut16IDs, unchanged registry phase/old14 data.119763 NPC scale0.4 is metadata, not player scaling. Format/manual readability/diffcheck pass; newer covering head required. Nonzero pennant, actual outdoors Lil Wick linked285/Control4 carry and stone slow90/periodic action remain gaps; root462924 SpellMisc absence is70391-specific, not fabricated default timing or all-profile absence. Lampyridae exact attachment CPU scope retained.498 landed/master8707ac0ea; no further runtime/data/own heavy duplicate.

## Pending Shroud acceptance;498 landed and selected-pet/paid-elixir/guard-radius boundaries (2026-10-17)

[Source545–549 matrix](../../toy-effects14-coverage.md#source545549-pending-shroud-acceptance-and-selected-petpaid-elixirguard-radiuspennant-boundaries-2026-10-17) adds acceptance-only7463a1090:226810/459531/228081/three models/source1.5s/20min/1h recovery as14th disguise plus Oomgut15IDs; existing cast_ms phase/old13 data unchanged. Format/manual readability/diffcheck pass, newer head required. Selected minipet90 leash, actual50silver Noggenfogger outcomes, City-of-Threads guard-radius visualization and nonzero pennant remain gaps, not caster/own-critter substitution/free aura/guessed overlay/stealth. Lead confirms498 LANDED/master8707ac0ea; local c013 ancestry exits0. Its older11disguise/10object/16caster scope does not certify new14/11/18 expansions; no further runtime/data/own heavy duplicate.

## Lead498 PASS and pending Kobyss/Pileus acceptance (2026-10-17)

[Current498 scope and source540–544 matrix](../../toy-effects14-coverage.md#lead498-scoped-passnewer-expansions-still-pending-2026-10-17):498c013 PASS306/0failed/1ignored545.19s/client9/EXIT0/FMT0,5m37s/no warnings, landing pending lead confirmation. Corrected11-disguise/473Stasis,19 hearthstones,16 stationary/16 caster/10 object scopes pass—not newer13/18/11 expansions.

f44658cf6 adds Kobyss225641/455426/226283/41 choices/source1.5s cast/30min and Pileus225910/456572/226964/29 choices/10min, both1h recovery, now13 disguise rows plus Oomgut14IDs. Count-free identity slice retains old11 values/order/verified cast0; data-driven pending-halfcast phase preserves readable behavior and source timing. Format/manual readability/diffcheck pass, future covering head required. Permanent reading/nonzero pennant and exact enemy-player/frontal corpse flag remain gaps; no scale/count/model-pool/native extrapolation or runtime/data/own heavy duplicate.

## Pending Toxic Victory acceptance and owned-pet-cooking/finery/dyeing/role-dummy boundaries (2026-10-17)

[Source535–539 matrix](../../toy-effects14-coverage.md#source535539-pending-toxic-victory-acceptance-and-owned-pet-cookingfinerydyeingrole-dummy-boundaries-2026-10-17) adds acceptance-only896be829e:225547/443689/source1800s as18th existing learned/cancel caster identity, pending newer head AFTER498c013. Owned non-combat pet target150/candle/395 cooking area, missing royal finery chest, periodic five-second dyeing wheel and outdoors opposing-role damage-and-heal Icarus dummy remain exact gaps, not caster light/inert props/invented items/damage-only dummy. Format/manual readability/diffcheck pass; no further runtime/data/new native/own heavy duplicate.498 corrected Claw/Stasis/Stone individually ok, aggregate still pending.

## Layered-spin/self-memory/torch/mixed-levitation/moving-hologram boundaries (2026-10-17)

[Source530–534 matrix](../../toy-effects14-coverage.md#source530534-layered-spinself-memorytorchmixed-levitationmoving-hologram-boundaries-2026-10-17) retains Ravager slow33 and nested5s/10s/30s periodic chain ending7s stun/parent removal, personalized Memory Chord with literal11686/weight1/22903/weight0, unaudited permanent torch, all hover/featherfall/waterwalk/animation1151 siblings and missing forward-racing horse225414. No cosmetic-only spin, removal-as-cast, generic player-copy body, cooking torch guess, free flight or stationary horse shortcut. No further runtime/data/new proof/own heavy duplicate;498 in progress.

## Permanent-pod/random-wormhole/seed/recipient-gift boundaries and retained Camera (2026-10-17)

[Source525–529 matrix](../../toy-effects14-coverage.md#source525529-permanent-podrandom-wormholeseedrecipient-gift-boundaries-and-retained-camera-2026-10-17) preserves permanent three-model Escape Pod, RequiredSkill2875 random Khaz Algar wormhole/missing actor, mixed seed/area33467 and friendly missile Hat/60s aura plus actual recipient223316 product. Product shares the toy's name but has distinct ID/DurationInInventory60/MaxCount1; caster-only creation is not the contract. Existing Filmless Camera attachment/cancel CPU proof retained, not native photography output. No further runtime/data/new proof/own heavy duplicate;498 lead-owned/in progress.

## Pending Cherry Trail acceptance and puntables/opt-in-fireworks/cooking/water-blaster boundaries (2026-10-17)

[Source520–524 matrix](../../toy-effects14-coverage.md#source520524-pending-cherry-trail-acceptance-and-puntablesopt-in-fireworkscookingwater-blaster-boundaries-2026-10-17) adds acceptance-only4b21ddb7e:220777/447446/source600s as17th existing learned/cancel caster identity, pending newer head AFTER498c013. Missing puntable-box creature237586 is not same-number Bonfire GO; fireworks require attack-to-opt-in viewers/event recovery, missing452051 Lantern retains cooking service, and existing wild type22 water display still refuses its unscripted445930. No inert/shared-number/service shortcut or new native/runtime/data proof. Format/manual readability/diffcheck pass; no own heavy duplicate.

## Pending colorful-chair acceptance and wing/two-person-cracker boundaries (2026-10-17)

[Source515–519 matrix](../../toy-effects14-coverage.md#source515519-pending-colorful-chair-acceptance-and-wingtwo-person-cracker-boundaries-2026-10-17) adds acceptance-only66ca57ee0:218112/441839→434317/type7/89277/size1.5/source2yd/300s, now11 transmitted-object identities, old ten unchanged. Format/manual readability/diffcheck pass; future head AFTER498c013 required, no seated/native/recharge proof. Distinct Yu'lon/Niuzao occasional eight-hour/20s-period wings and paired Cracker/one surprise recipient remain gaps; no guaranteed proc, name-derived combat buff or solo/both-recipient reward. Draconic Banner exact seasonal CPU witness retained. No further runtime/data/own heavy duplicate.

## Gate496 ordering corrections and source510–514 boundaries (2026-10-17)

[Current496/498 proof and source matrix](../../toy-effects14-coverage.md#gate496-fixture-order-failures-corrected498-pending-2026-10-17) supersedes earlier running496 entries.496 FAILED303/2/1ignored536.82s/client9/EXIT101/FMT0,2m16s/no warnings:450 checked longer recovery during GCD,473 checked item recovery after restored Frostbolt started combat. Lead-approved test-onlyc013eafdd waits ordinary1500ms and reorders Stasis check before Frostbolt; no reset/clear/weakened assertion, corrected behavior pending498 including503.16 stationary/16 caster/10 object/18 hearthstone fixtures have scoped individual successes, not whole-gate/landing proof.

[Source510–514](../../toy-effects14-coverage.md#source510514-critter-beamduck-eggfaction-chestperiodic-wing-boundaries-2026-10-17) retains critter128/beam/figurine, missing interactive duck egg, unnamed216887 faction equipment chest and distinct Xuen/Chi-Ji occasional eight-hour/20s-period/category2272 wing contracts; no generic stun, inert egg, invented item or guaranteed proc. No further runtime/data/new native/own heavy duplicate.

## Mount-randomization/quest-credit/colored-parasol/fallen-player burial boundaries (2026-10-17)

[Source505–509 matrix](../../toy-effects14-coverage.md#source505509-mount-randomizationquest-creditcolored-parasolfallen-player-burial-boundaries-2026-10-17) preserves per-mount customization plus quest80201 completion, three distinct colored conditional Parasols and fallen-player Gravestone transformation. All colored child remove164 fields literally reference white parent431949; do not silently rewrite to each color, treat removal as recast or drop105/409/312/431982 siblings. Existing burial model117236 does not certify dead-player eligibility/lifecycle or allow a living-target substitute. No further runtime/data/new proof/own heavy duplicate.496 marks disguise/Stasis failed, final diagnostics pending.

## Pending Stone-of-Hearth acceptance and pennant/table/conditional-parasol boundaries (2026-10-17)

[Source500–504 matrix](../../toy-effects14-coverage.md#source500504-pending-stone-of-hearth-acceptance-and-pennanttableconditional-parasol-boundaries-2026-10-17) adds acceptance-onlyd68256325:212337/401802/source10s/shared2309 as19th saved-bind case with Flame refusal, pending a head after4964df7af2e6. Permanent nonzero pennant, missing interactive Hearthstone table and next-fall Parasol remain gaps. Parasol child mixes featherfall/can-turn409/animation1133/removal164 plus dummy child; parent removal is not recursive recast or always-on slow fall. Abyss Horn existing visual-only/no gameplay water/actor CPU scope retained. Format/manual readability/diffcheck pass; no further runtime/data/new native/own heavy duplicate.

## Observer/party-private perspective/fixed-Gilneas/water-boat boundaries and retained banner (2026-10-17)

[Source495–499 matrix](../../toy-effects14-coverage.md#source495499-observerparty-private-perspectivefixed-gilneaswater-boat-boundaries-and-retained-banner-2026-10-17) retains Eyes' caster-and-summons/periodic/two perception payloads, outdoors party-or-raid-private Date Modulator, fixed Gilneas252+77 travel and permanent controlled four-seat water cruise/radius20yd with literal base430489. No global/owner-only audience swap, saved-bind substitution or executable base/count guess. Dreaming Banner211424 retains exact seasonal fixture CPU scope, not new pixels/recharge. No further runtime/data/new proof/own heavy duplicate;496 missile-expanded gate running.

## Monument/raid-aura/pennant/moving-trail/owl-rest/sleep boundaries (2026-10-17)

[Source490–494 matrix](../../toy-effects14-coverage.md#source490494-monumentraid-aurapennantmoving-trailowl-restsleep-boundaries-2026-10-17) preserves Murkmorpher missing body/interaction plus raid-area65 child;425843 description reference is not a root trigger. Permanent nonzero Verdant pennant, Socks500ms periodic→1s persistent-area27 trail, Owl Post nearby rest and missing Leafbed usable sleep/pose remain unaudited. No automatic shape/one-aura trail/inert bed substitution or guessed multiplicity. No further runtime/data/new proof/own heavy duplicate;496 lead-owned/pending.

## Festive friend/growth/racial-homebind boundaries and retained interactive banner (2026-10-17)

[Source485–489 matrix](../../toy-effects14-coverage.md#source485489-festive-friendgrowthracial-homebind-boundaries-and-retained-interactive-banner-2026-10-17) preserves linked Festive Pepe114425/scale0.2/probability0, unaudited three-friend and rapid-growth presentation, and explicitly Draenei-only Hologem despite source ItemSparse race halves-1/on-use condition0. Human homebind acceptance cannot close racial qualification. Chilling Banner is type22/Data0→424023, with retained actual object-use/3600s caster aura/unchanged-health/model CPU witness, not inert type5 or native/access/expiry proof. No further runtime/data/own heavy duplicate;4964df7af2e6 queued with missile filters/fmt.

## Approved cursor-flight fix; complete16-row local GREEN (2026-10-17)

[Current approved fix/proof matrix](../../toy-effects14-coverage.md#approved-cursor-flight-fix-complete16-row-local-green-2026-10-17) supersedes pending approval/ignored492 state below. Lead-approved8687f84c5 adds only87 to the existing destination whitelist/comment and removes honest ignore; every assertion/16 identities retained. Authorized native helper PASS1/0failed/0ignored47.72s, compile3m43s/no warnings: all seven minis preserve source flight/pre-actor-zero/post-actor-one plus existing actor/placement/state/expiry semantics, alongside old nine scenery rows. Format/fmtcheck/manual readability/diffcheck and exact pushed readback pass. Other new tests compiled but filtered, not behaviorally certified. Successor492 aggregate/landing remain lead-owned/pending; no deployment/native/full-height/wall-time/recovery certificate or further own heavy duplicate. Only explicitly approved runtime exception; data/producer/engine/protocol unchanged.

## Pending Flame return-home acceptance and unnamed hat/ride/private-weather boundaries (2026-10-17)

[Source480–484 matrix](../../toy-effects14-coverage.md#source480484-pending-flame-return-home-acceptance-and-unnamed-hatrideprivate-weather-boundaries-2026-10-17) adds acceptance-only2302a9e1b:209035/422284/source10s/shared2309 as18th saved-bind case, with Deepdweller sibling refusal; newer head beyond492 required. Missing ItemSparse208880 stays unnamed despite periodic drake-hat/Brewfest description. Gryphon Control5 and rolling Barrel Control0 still require actual ride/control, not scenery; actors absent. Dreamsurge331 is target-only weather488/density1 plus screen2078/7 and zone-weather restoration, not phase or screen-only behavior. Format/manual readability/diffcheck pass; runtime cursor87 proposal awaits approval, no production/data/native/own heavy duplicate.

## Gate492 cursor-flight RED; runtime proposal awaiting approval (2026-10-17)

[Current RED/proposal matrix](../../toy-effects14-coverage.md#gate492-cursor-flight-red-runtime-proposal-awaiting-approval-2026-10-17) supersedes earlier492 queued/running entries:286passed/1failed/1ignored544.23s/client9/EXIT101/FMT0,2m14s/no warnings. Maiev actually appears before source flight,1/expected0. Lead-approvedde2724abe honestly ignores the whole16-row fixture with every assertion retained; later mini rows were not reached, no fix/aggregate/landing certificate.387 corrected regen,393Ohn'ir,402Soulshape and419Doomsday individually pass. Proposal awaiting approval adds87 to existing destination whitelist, backed by70448 effect32/87 roots/speeds and pinnedTCa352b1fa destination distance/speed/HasDst logic. New445/450/454/457/473/477 tests remain after492 and unverified; no production/data/new native change or own heavy duplicate.

## Pending Deepdweller return-home acceptance and mounted-pennant/mirror/memory/ring boundaries (2026-10-17)

[Source475–479 matrix](../../toy-effects14-coverage.md#source475479-pending-deepdweller-return-home-acceptance-and-mounted-pennantmirrormemoryring-boundaries-2026-10-17) adds acceptance-onlyeea1ff744:208704/420418/source10s/shared2309 as17th saved-bind helper case, with actual Path-of-Naaru sibling refusal; newer head beyond4920c77 required. Racer pennant is mounted78 plus400ms periodic/keybound406 and area8051, not a back-aura shortcut. Zero-entry man'ari Mirror, group8008-qualified Tyrhold/Valdrakken memory disc and periodic party rings/spacing/five-minute child lifetime/holiday recovery retain gaps. Local format/manual readability/diffcheck pass; no runtime/data/model guesses/native proof or own heavy duplicate.

## Pending Stasis acceptance and torch/ritual/follower/random-wolf boundaries (2026-10-17)

[Source470–474 matrix](../../toy-effects14-coverage.md#source470474-pending-stasis-acceptance-and-torchritualfollowerrandom-wolf-boundaries-2026-10-17) adds acceptance-only52e9f7c80: actual Stasis208415/419271 self-stun60s/300s recovery, real Frostbolt Stunned refusal, ordinary expiry/removal/restored cast and retained cooldown; exact intentional replies preserved. Retail item comes from content_item/item_sparse joins, not legacy item_template. Torch's mixed kill-credit229063/device actions, seven-second incomplete ritual, Boffins actual follower and random-volume giant wolf remain gaps. Format/manual readability/diffcheck pass; future covering head beyond4920c77 required, no native movement/time-freeze/runtime/data admission or own heavy duplicate.

## Platter/tiny-rocks/vehicle-detection/hourglass-action boundaries and retained banner (2026-10-17)

[Source465–469 matrix](../../toy-effects14-coverage.md#source465469-plattertiny-rocksvehicle-detectionhourglass-action-boundaries-and-retained-banner-2026-10-17) preserves mixed platter/script/service and base4/special-flag rock multiplicity with missing bodies. Idol296 sets vehicle8196 and500ms periodic child retains all four ordered repeated script recipients; not decorative tracking. Minute Glass override2600 grants six distinct60/120/300/600/1800/3600s mixed aura/area actions, not one root timer. Smoldering Banner208057 retains exact seasonal-fixture CPU witness. No runtime/data/new proof/native/own heavy duplicate;492 running.

## Pennant/mannequin/grain/relative-coordinate/pocketwatch boundaries (2026-10-17)

[Source460–464 matrix](../../toy-effects14-coverage.md#source460464-pennantmannequingrainrelative-coordinatepocketwatch-boundaries-2026-10-17) preserves nonzero permanent Crimson pennant, script-driven mannequin selection, missing403749 grain crate, permanent1500ms periodic Thinking Cap and nonzero hour-long Pocketwatch. Source cap swaps w2/w1 with50000 offsets and re-equips to reset its coordinate origin; actual projection/thought behavior remains gap, not cosmetic or guessed map axes. No infection from grain's name or server-clock shift from pocketwatch. No runtime/data/new proof or own heavy duplicate;4920c77 compiling.

## Pending Naaru return-home acceptance and explosive-show/critter/pennant/service boundaries (2026-10-17)

[Source455–459 matrix](../../toy-effects14-coverage.md#source455459-pending-naaru-return-home-acceptance-and-explosive-showcritterpennantservice-boundaries-2026-10-17) adds acceptance-only1ce4b65a0: Path of the Naaru206195/412555/source10s/shared2309 in unchanged actual saved-bind helper, now16 hearthstones; exact one-charge/1800000ms and Ohn'ir refusal, pending newer head beyond4920c77. Flamin' Ring preserves three distinct area edges plus child script; Fyrakk's actual critter action, permanent nonzero pennant and missing Warpweaver/transmog service remain gaps. Format/manual readability/diffcheck pass; no runtime/data/native/service or cosmetic-only critter admission, no own heavy duplicate.

## Pending Claw/Nightmare-banner acceptance and wisdom/audio/attraction boundaries (2026-10-17)

[Source450–454 matrix](../../toy-effects14-coverage.md#source450454-pending-clawnightmare-banner-acceptance-and-wisdomaudioattraction-boundaries-2026-10-17) adds acceptance-only1c5afda02: Claw205904/410931/205296/four valid models/source600s/category2177 one-hour recovery as11th disguise; Nightmare206008/411653→402852/type5/82612/size1/180s as tenth cursor object. Existing helper assertions retained; new behavioral head beyond4920c77 required. Senegos yes/no body/action, week-long listener-specific sniff audio suppression and friend-attraction area trigger remain gaps, not inert bodies/generic visual auras/teleport guesses. Local format/manual readability/diffcheck pass; no runtime/data/native/pool-sampling/full-recovery or own heavy duplicate.

## Pending Cinder acceptance and drum/ritual/glitter/lava-pack boundaries (2026-10-17)

[Source445–449 matrix](../../toy-effects14-coverage.md#source445449-pending-cinder-acceptance-and-drumritualglitterlava-pack-boundaries-2026-10-17) adds acceptance-only184f9874d: exact205418/409891/source600s in unchanged learned/cancel caster fixture, now16 identities, pending later head beyond4920c77. Mixed45s drum/property3911/flags67241992 and10s ritual/property5664 retain missing actor/behavior gaps. Friendly glitter child395 creates an area trigger, not generic visual aura. Lava Pack's override2525 grants410609→410641 through actual actionbar metadata; source PowerDisplay127 absent, full grant/toss/restoration remains gap. Format/manual readability/diffcheck pass; no runtime/data/producer/native/new behavioral certificate or own heavy duplicate.

## Mixed prison/horn/Loamm-travel boundaries and retained friend effects (2026-10-17)

[Source440–444 matrix](../../toy-effects14-coverage.md#source440444-mixed-prisonhornloamm-travel-boundaries-and-retained-friend-effects-2026-10-17) retains Titan prison mixed105/base10000/104/191/106 control, unaudited half-second horn and actual Zaralek→Loamm transfer/script gaps. RequiredAreas7894 is a group containing Loamm14520/Zaralek14022/map2454, not Dalaran Island's same-number area. Existing Mortar selected-friend cast/delivery and Fist selected-friend/compressed-day aura witnesses stay bounded, not full hostile/native/facing/impact/recharge certificates. No runtime/data/new acceptance or own heavy duplicate;492 pending.

## Linked-hologram/stone-smash/holiday-basket boundaries and retained Bear (2026-10-17)

[Source435–439 matrix](../../toy-effects14-coverage.md#source435439-linked-hologramstone-smashholiday-basket-boundaries-and-retained-bear-2026-10-17) preserves Lady of Dreams360s versus Timeless One300s linked428 roots, distinct stand identities and mixed permanent child aura; shared111698/scale0.35 does not certify either hologram. Stone Breaker's two root children, literal misc1300/700 and repeated28357 area edges remain distinct unresolved actions. Bear204405 retains exact CPU/earlier both-skin model evidence, not fresh pixels/recovery. Drake Basket retains caster aura/cancel proof, with mounted-drake and Noblegarden recovery gaps explicit. No runtime/data/admission/new proof or own heavy duplicate;492 queued0c77bfdaa.

## Cooking/forced-form/clan/linked-hologram boundaries and retained Will (2026-10-17)

[Source430–434 matrix](../../toy-effects14-coverage.md#source430434-cookingforced-formclanlinked-hologram-boundaries-and-retained-will-2026-10-17) preserves missing runtime386682 cooking-service scope, self ForceCast2→402636/202340 form, chosen-clan nonzero dummy and linked hologram428→403712 with mixed self aura/Visage Stand. Root Alexstrasza versus child King Mechagon text does not authorize model substitution; exact cast and item-recovery overrides retained. Hraxian204220 is already an exact15-caster484 row, not new native/natural-expiry/reuse proof. No runtime/data/admission changes or own heavy duplicate. Lead492 queued0c77bfdaa owns corrected regen/actor-phase proof;489 failure remains explicit.

## Gate489 assertion corrections; behavioral proof pending (2026-10-17)

[Current correction/proof matrix](../../toy-effects14-coverage.md#gate489-assertion-corrections-behavioral-proof-pending-2026-10-17) supersedes queued/live489 statements below. Saved489c043 FAILED284/2/1ignored577.08s/EXIT101/FMT0.0c77bfdaa replaces387's frozen-health expectation with a same-clock untargeted health/level/combat/regen-phase control, requiring positive natural regen; destination-child SpellGo expectations become actual pre/post-flight actor assertions in the16-row mini fixture. Source `DEST_TARGETS` lacks87, an unverified immediate-flight hypothesis, not an excuse to weaken ordering or claim production GREEN.393/402 individually ok;419 remains after the gate. Format/manual readability/diffcheck pass; corrected behavior/newer lead gate pending, no runtime/data change or own heavy duplicate.

## Pending Murloc/egg acceptance and retained banner/caber/snow boundaries (2026-10-17)

[Source425–429 matrix](../../toy-effects14-coverage.md#source425429-pending-murlocegg-acceptance-and-retained-bannercabersnow-boundaries-2026-10-17) adds acceptance-only10e8faaad: Murloc202862→200972/112325 and Whelp Egg202865→200975/112328 through independent speed30/18yd cursor roots; stationary-source fixture now16 exact identities. Local format/manual readability/diffcheck pass, expanded behavior pending corrected fixture/newer head. Exact Thundering Banner row retained in seasonal fixture; caber area-trigger/script/nonzero dummy and snow transform/191/ground area/370 contracts remain gaps. Inspected489c043 FAILED284/2/1ignored577.08s/EXIT101/FMT0:387 ordinary regeneration52 versus37 assertion,408 nonexistent child SpellGo expectation;393/402 individual ok, no whole-gate pass. No runtime/data/producer/native/tier/hatching/full-height/recovery certificate or own heavy duplicate.

## Pending three-mini acceptance and fishing/compass boundaries (2026-10-17)

[Source420–424 matrix](../../toy-effects14-coverage.md#source420424-pending-three-mini-acceptance-and-fishingcompass-boundaries-2026-10-17) adds acceptance-only53f4ae444: three independent speed30/18yd cursor missiles for Huntress202851→200961/112316, Tauren202856→200966/112319 and Ghoul202859→200969/112322. Existing stationary-source fixture now14 identities, original11 unchanged; parent-before-child/one actor/horizontal placement/owner/map/non-pet/stationary/compressed300s expiry covered by assertions, not yet a behavioral result. Child399250 Speed1 is preserved metadata, not a second projectile. Dented Can retains fishing destination39/mixed self aura/permanent/channel/cleanup contract despite existing385913; Lost Compass hold-out pose remains unaudited, not facing/scouting gameplay. Local format/manual readability/diffcheck pass; newer covering head required, no own heavy duplicate/runtime/data/native admission. Live489 marks387 failed,393/402 ok; final result/details pending.

## Pending Sneed/product acceptance and bobber/Furbolg/screen-effect boundaries (2026-10-17)

[Source415–419 matrix](../../toy-effects14-coverage.md#source415419-pending-sneedproduct-acceptance-and-bobberfurbolgscreen-effect-boundaries-2026-10-17) adds acceptance-onlyd3bed536f: Sneed's own speed30/18yd→398246→200350/114420 as eleventh stationary-source case, with Maiev12yd preserved via one root/distance choice.50f7b11fc adds learned202309/398572/source3s→one persisted202310,300000ms recovery/exact immediate refusal; old279 assertions retain187342 through an item-ID counter. Format/manual readability/diffcheck pass, later head beyond489c043 required for11-row and two-creator fixtures. Bobber compatibility258, nonzero Furbolg dummy, screen-effect260 and product MaxCount1/repeat/use remain gaps; no runtime/data/producer/native/full-height/tier/cap certificate or wrong two-product expectation.

## Riches-trail/ally-storm/enemy-corpse-flag/kite/shades boundaries (2026-10-17)

[Source410–414 matrix](../../toy-effects14-coverage.md#source410414-riches-trailally-stormenemy-corpse-flagkiteshades-boundaries-2026-10-17) preserves periodic riches trail plus minimum-speed437/base3/set1072, selected-friendly chasing storm, source-centered enemy-player corpse/front flag, unaudited kite and shades attachments. No guessed cadence/currency/flag identity, caster-only substitution or zero-dummy admission. [Current integration receipt](../../toy-effects14-coverage.md#lead-integration-receipt-2026-10-17) has inspected484 PASS283/0failed/1ignored408.90s/client9/EXIT0/FMT0,4m12s/no warnings, landed/masterec0863223;15caster/nineobjects covered. New387/393/402/408 are queued489 throughc043f0f8c, not passed. No runtime/data/producer/new native proof or own heavy duplicate.

## Pending Maiev missile-child acceptance and cloak/crystal/BG-flare/challenge-dummy boundaries (2026-10-17)

[Source405–409 matrix](../../toy-effects14-coverage.md#source405409-pending-maiev-missile-child-acceptance-and-cloakcrystalbg-flarechallenge-dummy-boundaries-2026-10-17) adds acceptance-only58a5fabd1:201931/398485 speed12 to valid12yd cursor, parent-before-child395720 event, one199047/112284/scale1 actor, horizontal destination/owner/map/non-pet/non-following/stationary/compressed300s expiry. Ten stationary-source cases now await covering489 throughc043f0f8c; format/manual readability/diffcheck pass, no full-height/native-tier/wall-lifetime claim. Targeted fading cloak and unaudited crystal, BG-only faction/area180s flare lock versus120s ordinary recovery, and outdoor opposing-role damage/heal dummy remain exact gaps; no generic script/costume/dummy substitutes or runtime/data admission.

## Pending Soulshape recovery/refresh acceptance and control/music/gang/holiday boundaries (2026-10-17)

[Source400–404 matrix](../../toy-effects14-coverage.md#source400404-pending-soulshape-recoveryrefresh-acceptance-and-controlmusicgangholiday-boundaries-2026-10-17) adds acceptance-only1f028916a: actual200960/392700/100634/source10-minute aura with five-minute recovery, near-threshold refusal/natural reuse while active/refresh/ordinary restoration and exact intentional replies. AURA_UNIQUE is clear and existing stacking refreshes duration; no manual reset. Format/manual readability/diffcheck pass, covering489 throughc043f0f8c is queued, result pending. Personal Soulshape selection/native behavior stays separate. Control3 boat plus player restrictions, mixed song/no-actions/mount state, linked gang follow/attack and holiday-specific dance recovery remain gaps; no scenery, music-only or static-actor substitutes/runtime/data/new admission.

## Invocation/eggs/cloak-service/random-talisman/alcohol boundaries (2026-10-17)

[Source395–399 matrix](../../toy-effects14-coverage.md#source395399-invocationeggscloak-servicerandom-talismanalcohol-boundaries-2026-10-17) preserves elemental invocation/base1200, three named egg hatchlings versus literal script base4, linked cloak collection with weighted visible/unused-invisible models, literal Talisman base references versus actual trigger edges, and alcohol effect100/base5. Existing bodies/child transforms do not authorize inert props, random-range guesses, dropped animation/removal or visual-only alcohol. No runtime/data/producer/new native proof or own heavy duplicate;387/393 still require a later head than484.

## Pending Ohn'ir return-home acceptance and pelt/catch/orb/eight-trigger boundaries (2026-10-17)

[Source390–394 matrix](../../toy-effects14-coverage.md#source390394-pending-ohnir-return-home-acceptance-and-peltcatchorbeight-trigger-boundaries-2026-10-17) adds acceptance-only00ee4d3e3:200630/391042/source10s252/savedbind/shared2309one1800000mscharge, now15hearthstone variants. Format/manual readability/diffcheck pass; covering489 throughc043f0f8c is queued, result pending. Missing pile/orb runtime identities, bouquet missile→cursor-centered1yd friendly catch and eight distinct Palooza area-trigger children remain exact action/lifecycle gaps. Orb availability outside Cobalt Assembly is not a new must-be-outside gate; preserve both distinct misc300 children and index order without guessed timing. No runtime/data/new native or full-recharge certificate.

## Pending mixed Chewed Wand acceptance and zombie/cage/rod/pelt boundaries (2026-10-17)

[Source385–389 matrix](../../toy-effects14-coverage.md#source385389-pending-mixed-chewed-wand-acceptance-and-zombiecagerodpelt-boundaries-2026-10-17) adds acceptance-only27772d6ad: learned200249/389324/source1700ms self56+confusion5/10000ms, damaged37 health, cast blocking, ordinary expiry/restoration and1800000ms recovery refusal. Format/manual readability/diffcheck pass; covering489 throughc043f0f8c is queued, result pending. Runtime sheep weights1:1/build12340 versus pinned2:1/build69497 remain an explicit profile gap, no producer fix or sampling certificate. Mixed zombie speed/keybound406, Control4 prison, selected-player Dust Pile/misc9 and absent-runtime lounging pelt remain separate action/eligibility/lifecycle gaps. No plain-disguise shortcut, runtime/data/new native proof or own heavy duplicate.

## Compass/lava/unnamed-money/speech/leap boundaries (2026-10-17)

[Source380–384 matrix](../../toy-effects14-coverage.md#source380384-compasslavaunnamed-moneyspeechleap-boundaries-2026-10-17) preserves north-pointing compass guidance, Dragon-Isles/outdoors lava immunity, unnamed200142's20g/payment and once-plus-three missile graph, mixed base7/script/aura speech, and caster-back leap/area-trigger odor actions.200142 has Toy/Item/on-use rows but no ItemSparse; coin GO identities absent from runtime TDB, no invented name or dropped cost/duplicate child. No audited-cosmetic shortcut, runtime/data/new positive proof or own heavy duplicate;484's exact older-head scope stays pending.

## Pending Survey Tools acceptance and folly/dummy/seat/sled boundaries (2026-10-17)

[Source375–379 matrix](../../toy-effects14-coverage.md#source375379-pending-survey-tools-acceptance-and-follydummyseatsled-boundaries-2026-10-17) adds acceptance-onlyafad56176: existing Survey206590/type5/10103/1/cursor87/300s is ninth transmitted-object case; format/manual readability/diffcheck pass, lead484 passes through this exact head with15casterIDs. Folly retains bounded group-private actor/visibility/expiry proof, not its friends' challenge result. Rubbery Fish Head outdoors/combat, Blue Beanbag78/radius1yd seating and linked Tug Sled's392313/392328 rider/control/cleanup remain gaps; base173426 is not a guessed count. No runtime/data/new native or producer admission.

## Pending Blue/Bronze/Green acceptance, training gap and retained soup proof (2026-10-17)

[Source370–374 matrix](../../toy-effects14-coverage.md#source370374-pending-bluebronzegreen-acceptance-training-gap-and-retained-soup-proof-2026-10-17) adds acceptance-only73494c432: distinct Blue381321/77021, Bronze381319/76419, Green381320/77020, alltype5/scale0.5/cursor87/180s. These eight rows are unchanged inside the nine-object fixture passing484 throughafad56176 with15caster; format/manual readability/diffcheck pass. Training196394/outdoors/combat behavior remains a gap, while [Soup's exact linked-cooking CPU witness](../../toy-effects14-coverage.md#linked-cooking-traps-2026-10-12) is retained without Soup-native certification or a guessed stat coefficient. No runtime/data/new admission or own heavy duplicate.

## Pending Black Dragon acceptance and seat/tea/Red-banner boundaries (2026-10-17)

[Source365–369 matrix](../../toy-effects14-coverage.md#source365369-pending-black-dragon-acceptance-and-seatteared-banner-boundaries-2026-10-17) adds acceptance-onlyb53bd5e5b: Black199768/388089→existing381318/type5/71263/0.5/cursor87/180s as fifth transmitted-object case, preserving previous four rows. Format/manual readability/diffcheck pass; these five rows are unchanged inside the nine-object fixture passing484 with15caster. Actual Red381317 refusal remains distinct. S.E.A.T. caster-ground/type7 seating and two type10 tea props with separate394315/394316 seated-drinking85/226 actions retain full interaction/effect/lifecycle gaps. No inert tea, copied banner, guessed mana/seat rule, runtime/data/new native certificate or149865 producer change.

## Pending Reliquary/Duck acceptance and banner/sled/vacuum boundaries (2026-10-17)

[Source360–364 matrix](../../toy-effects14-coverage.md#source360364-pending-reliquaryduck-acceptance-and-bannersledvacuum-gaps-2026-10-17) adds361 acceptance-only431949735: existing Reliquary380639/type5/76510/0.6/cursor87/180s becomes fourth transmitted-object case, including concrete owner assertions. Existing audited335 Duck Lovie383065 also gains actual194056/permanent0/cancel acceptance49e9c2d95, now15casterIDs. Format/manual readability/diffcheck pass; lead484 throughafad56176 has explicit nine-object/15-caster fixture ok lines inspected. Missing Explorers'380638, Control4 sled, distinct Lucky Duck385941 and frontal rectangular critter-vacuum child remain gaps; no blanket dummy/banner/control admission. [Contract](../../specs/toys.md).

## Taivan buff/pillow boundaries and retained statue proof (2026-10-17)

[Source355–359 matrix](../../toy-effects14-coverage.md#source355359-taivan-buffpillow-boundaries-and-retained-statue-proof-2026-10-17) preserves Taivan195387/description-only384543/scaled189 nearby Versatility action gap, exact retained [Ornate statue CPU/both-skin pixels](../../toy-effects14-coverage.md#placed-statues-and-the-missing-template-audit-2026-10-12), and three distinct pillow placement/interaction identities. Source78/radius1yd is not zero “at feet”; Purple376979/type7/74543/size3 exists in actual runtime TDB loading, while380634/380635 do not. No copied geometry, invented buff amount or decorative-only seating admission; no new runtime/data/native certificate or own heavy duplicate.

## Private weather/shell/raft/easel boundaries and retained cooking proof (2026-10-17)

[Source350–354 matrix](../../toy-effects14-coverage.md#source350354-private-weathershellrafteasel-boundaries-and-retained-cooking-proof-2026-10-17) keeps Title55 outdoor/party-private weather, mixed shell105964/action26/set1048, water/jump-paddle Dinghy104/226 and Easel373/191/base2.5 action/movement gaps.351 retains exact [source16 cooking CPU scope](../../toy-effects14-coverage.md#cooking-focus-batch16-2026-10-12); shared Grim native fire is not Maruuk per-toy pixels. **329 correction:** runtime `tdb_gameobject_template` is authoritative for summoned objects and contains offering377204/74995; earlier absence inference from empty base/content GO tables is withdrawn, interaction gap unchanged. No runtime/data/new admission or own heavy duplicate.

## Slide/wyrmhole/recalibration/private-environment/perception boundaries (2026-10-17)

[Source345–349 matrix](../../toy-effects14-coverage.md#source345349-slidewyrmholerecalibrationprivate-environmentperception-gaps-2026-10-17) preserves mixed slide373/104/312set1047/periodic23-child/263, engineering2827/rank1 Wyrmhole access/travel/triangulation, recalibrator area-trigger179/27706, outdoors party/raid-private Title55 environment and observer-scoped233 gnoll perception. Wyrmhole195667/environment194993 runtime templates are absent, and the existing Wyrmhole census remains blocked. No guessed cadence/destinations/race pools, privacy reduction, inert imports or new positive/native proof; lead-owned gates are not duplicated.

## Mailbox/whelps/holiday/feast boundaries and retained sign proof (2026-10-17)

[Source340–344 matrix](../../toy-effects14-coverage.md#source340344-mailboxwhelpsholidayfeast-boundaries-and-retained-sign-proof-2026-10-17) keeps Ohuna Control1/Title5 mail/recall, five-whelp mixed106/105/104/506 movement/presentation, Murglasses233/misc6379/169 holiday recovery and four-model appreciative-critter feast actions unresolved. Source341 retains exact earlier sign193182/108105/2s/5min/ground-placement/owner/map/expiry CPU scope, not a new native certificate. Ohuna/critter TDB identities exist but runtime templates do not; no inert admission or guessed service/physics/holiday/product behavior.

## Lead integration receipt (2026-10-17)

[Current coverage receipt](../../toy-effects14-coverage.md#lead-integration-receipt-2026-10-17) records inspected477 server280passed/0failed/1ignored, client9passed, EXIT0/FMT0/master15bef300d and478 through28f1ce365 server283passed/0failed/1ignored672.55s, client9passed, EXIT0/FMT0,1m24s compile/no warnings. Exact three tents/all14hearthstones/ten-disguise+Oomgut fixtures have explicit ok lines; ignored case is the named non-toy invisible stalker. Lead confirms478 LANDED/mastera2ddee2cb. Inspected484 throughafad56176 has283passed/0failed/1ignored408.90s/client9/EXIT0/FMT0/4m12s/no warnings; exact15-caster/nine-object fixtures pass, lead confirms LANDED/masterec0863223. New387/393/402/408 await489 throughc043f0f8c, not borrowed484 proof. No native/full-recovery/full-toy certificate.

## Pending linked-tent acceptance and nap/seat boundaries (2026-10-17)

[Source335–339 matrix](../../toy-effects14-coverage.md#source335339-pending-linked-tent-acceptance-and-napseat-boundaries-2026-10-17) adds acceptance-only28f1ce365: distinct Market194059/398329→398328/111097 and Expedition194060/398334→398333/111099 through the existing Gnoll source-cast/placement/rotation/refresh/owner/cancel/death fixture. All three tests pass in explicitly covering lead478 through28f1ce365; format/manual readability/diffcheck pass. No native/rest/natural15-minute-expiry or commerce certificate. Duck Lovie's existing audited383065 route has15-ID caster acceptance49e9c2d95 passing in484; native nap/pose remains separate. Cushions195980/109046,196013/109053 retain exact presentation/interaction gaps; runtime cushion templates absent, no inert imports or invented mechanics. [Contract](../../specs/toys.md).

## Pending Timewalker acceptance and prism/tent/seat/pall boundaries (2026-10-17)

[Source330–334 matrix](../../toy-effects14-coverage.md#source330334-pending-timewalker-acceptance-and-prismtentseatpall-boundaries-2026-10-17) adds acceptance-only4ee545b9f:193588/375357/source10s252/savedbind/shared2309one1800000mscharge, now14hearthstone variants. Format/manual readability/diffcheck pass; all14hearthstone cases pass in478 through28f1ce365;477 throughd4 had only13. Existing prism activation/action-block/cancel and linked-tent source placement/refresh/owner/cancel/death witnesses remain bounded; guided party beam/native/natural linked expiry remain separate. Beanbag seating stays blocked despite TDB108950, and permanent dead-recipient Pall108530 needs actual recipient/corpse lifecycle. No runtime/data/native admission or own heavy duplicate.

## Audio/launch/wisdom/animation/offering boundaries (2026-10-17)

[Source325–329 matrix](../../toy-effects14-coverage.md#source325329-audiolaunchwisdomanimationoffering-boundaries-2026-10-17) preserves509 audio-filtering scope, mixed98/base1200 launch,189951/properties5348/Title27 wisdom actor,312/set61 animation replacement and377204/type3 offering interaction. Wisdom/display107044 exists in source TDB but lacks runtime content; offering377204/display74995 is present in the runtime TDB table actually loaded for summoned objects (earlier absence inference corrected). Availability does not prove advice or acceptance action. Source cast/duration/item-versus-inherited recovery remains exact. No new positive/native proof, inert import, guessed stealth/launch/loot behavior or own heavy duplicate.

## Pending Harpy acceptance and fruit/core/star-mode boundaries (2026-10-17)

[Source320–324 matrix](../../toy-effects14-coverage.md#source320324-pending-harpy-acceptance-and-fruitcorestar-mode-gaps-2026-10-17) records acceptance-only eb02827f3:191891/371470→existing187854/104851/source10-minute transform/30-minute item recovery. Ten disguises plus Oomgut make11IDs; format/manual readability/diffcheck pass, exact disguise/Oomgut fixture ok lines inspected in478 through28f1ce365. Lead477 throughd4 contains the older nine-disguise fixture, not Harpy. NPC model1.5/probability1 stays metadata, not an automatic player-scale requirement. Fruit periodic child, living-core back attachment/positioner and paired star-flinger/catcher modes/actions remain explicit gaps; no runtime/data/native admission.

## Pending Cube acceptance and cache/song/mines/specimen boundaries (2026-10-17)

[Source315–319 matrix](../../toy-effects14-coverage.md#source315319-pending-cube-acceptance-and-cachesongminesspecimen-gaps-2026-10-17) adds317 test/spec d4a4a716c: existing185567/105515 with source15-minute expiry and1h item recovery, so duration is per-case while old eight10-minute values/windows stay unchanged. Nine disguises plus Oomgut make10IDs; format/readability/diffcheck pass, covering lead477 is landed; exact fixture ok lines are inspected in the linked integration receipt. The handler writes only `ModelDisplay { display_id }`, matching pinned TC's selected-ID handoff; NPC model scale1.25/probability0 is retained metadata, not an invented player-scale requirement. Chest/trap, mixed Jiro song, six step-trigger mines and specimen placement remain gaps; no runtime/data/native admission or own heavy duplicate.

## Pending Batons/return-home acceptance and seated-orb boundary (2026-10-17)

[Source310–314 matrix](../../toy-effects14-coverage.md#source310314-pending-batonsreturn-home-acceptance-and-seated-orb-gap-2026-10-17) adds test/spec01de71c2e: Batons exact900000ms caster row makes14IDs, while Dominated/Enlightened/Broker Matrix real learned saved-bind/source10s/shared2309 cases make13 hearthstone variants. Local format/readability/diffcheck pass; lead477 throughd4a4a716c is reported landed, not borrowed from earlier counts; see the linked integration receipt for exact scope.312's seated/orb action remains unproved despite zero dummy shape. No runtime/data admission, native/ordinary recovery/cross-map certificate or own heavy duplicate.

## Seasonal/flower/cooking boundaries and pending Insoles acceptance (2026-10-17)

[Source305–309 matrix](../../toy-effects14-coverage.md#source305309-seasonalflowercooking-boundaries-and-pending-insoles-acceptance-2026-10-17) retains Winter Veil/285-child music, observed Wild Flower legacy8.82-versus-Retail1 model state plus scripted Grow, and mixed caster-hosted cooking gaps.308 keeps [earlier actual-area/native scope](../../toy-effects14-coverage.md#source-requiredareasid-lifecycle-2026-10-13) without a new calendar certificate.309 test/spec8a986da00 adds exact permanent188699/45416 to the existing caster fixture (13IDs at8a986da00;14 after01de71c2e,309 unchanged); local format/readability/diffcheck pass, exact fixture included in lead477 throughd4a4a716c, landed with the exact caster fixture ok line inspected in the integration receipt. Separate RequiredTransmogHoliday341 is not an invented use-time rule; no fresh native/recovery/calendar proof or runtime/data admission.

## Source scouting/slime-trail boundaries and retained seasonal banners (2026-10-17)

[Source300–304 matrix](../../toy-effects14-coverage.md#source300304-scoutingslime-trail-boundaries-and-retained-seasonal-banners-2026-10-17) keeps Outland/two-island flight-map discovery and source10ms-period/one-second slime-child moving-trail/pledge-OR-renown80 gaps distinct. Encrypted/Shrouded retain their exact [historical source9 banner CPU rows](#toy-effects9-stationary-banners-and-sign-2026-10-10), source models/2yd placement/expiry, not fresh native certificates. No invented discovery set, unconditional trail schedule, caster visual shortcut or new runtime/data admission.

## Source scouting-map discovery boundaries (2026-10-17)

[Source295–299 matrix](../../toy-effects14-coverage.md#source295299-exact-scouting-map-discovery-boundaries-2026-10-17) retains five distinct source flight-point/map-reveal contracts: Broken Isles, Draenor, Pandaria, Cataclysm-established maps and Northrend. Shared zero-payload self script3/source2s cast does not authorize cosmetic success, broad geography-to-node guesses or copied eligibility/progress. Actual authored discovery/fog/persistence remains unproved for each; no player-progress/data admission or new positive proof.

## Source banner/wings/ride/metadata/scouting boundaries (2026-10-17)

[Source290–294 matrix](../../toy-effects14-coverage.md#source290294-retained-banner-proof-and-wingsridemetadatascouting-gaps-2026-10-17) retains290's [earlier exact banner CPU proof](#toy-effects9-stationary-banners-and-sign-2026-10-10), not new native certification. Night Fae-OR-renown80 wings, outdoor Control4 rolling ride, unresolved187861 item metadata/seed action and flight-node/map-reveal script remain gaps.187861 has Toy/Item/on-use rows but no current ItemSparse/runtime item; no invented name, fallback definition or Flags2 meaning. No runtime/data admission or new positive proof.

## Source critter/cooldown/stage/friends/containment boundaries (2026-10-17)

[Source285–289 matrix](../../toy-effects14-coverage.md#source285289-crittercooldownstagefriendscontainment-boundaries-2026-10-17) retains Venthyr-OR-renown80 critter atonement, Nightborne-specific reduced recovery, controlled dance stage, three separate fuzzy-friend child triggers and Control4 self imprisonment gaps. Large base359469/173426 fields are not executable trigger records or guessed counts; existing containment103058 is not control/cleanup proof. No qualification/cooldown/payload substitution, runtime/data admission or new positive certificate.

## Source controlled-cage/steward/realm/customizer boundaries (2026-10-17)

[Source280–284 matrix](../../toy-effects14-coverage.md#source280284-controlled-cagestewardrealmcustomizer-boundaries-2026-10-17) keeps Control4 cage entry/link cleanup, pledged steward-companion transformation, invisible-body linked realm and complete Racer customization/control restoration gaps. Existing cage/steward/realm bodies are not new admission; source-selected invisible11686 must not be replaced with zero-weight15092. No ordinary self costume, inert racer or dropped child/control payload, runtime/data write or new positive proof. Lead472 owns240's package gate.

## Offering-kit targeted acceptance and vesper/transport boundaries (2026-10-17)

[Source275–279 matrix](../../toy-effects14-coverage.md#source275279-offering-kit-targeted-acceptance-and-vespertransport-boundaries-2026-10-17) records three distinct mixed actor/caster Vesper identities/scales and Whistle fast/tired transport/area gaps.279 has an exact existing create-item product187342/count1/source2s/item30s contract; Test/spec04a47a43 and fixture fix62db4212 pass actual learned use, persisted quantity, cast and ordinary recovery/reuse:1/1 zero ignored31.60s after1m30s compile/no compiler warnings, fmt/readability/diffcheck pass. The initial fixture mistakenly treated its two intended cooldown refusals as unexpected failures; final exact-message assertions handle both. Lead476 landed277 passed/master0b17f866f; no own package-check repeat. Source naming differs from runtime item name, so retain the exact ID. No shared Vesper/transport shortcut, wrapping/gifting claim, new runtime/data admission or native certificate.

## Source mirror/fate/mixed-guise/shadow/dead-party boundaries (2026-10-17)

[Source270–274 matrix](../../toy-effects14-coverage.md#source270274-mirrorfatemixed-guiseshadowdead-party-boundaries-2026-10-17) retains action-copy mirror/movement removal, authentic rune fate output, mixed Changeling transform/-45/312 payload, selected-player shadow child and recently-deceased party judgment gaps. Existing176673/two-model appearance is not ordinary-disguise acceptance for a mixed graph; target35 is party, not a generic enemy corpse. No actor/model/payload substitution, runtime/data admission or new positive proof.

## Source pet-chase/flight/destruction/drag/mixed-shade boundaries (2026-10-17)

[Source265–269 matrix](../../toy-effects14-coverage.md#source265269-pet-chaseflightdestructiondragmixed-shade-boundaries-2026-10-17) keeps companion/golem chasing, permanent minipet flight payload, Minigrue toy destruction, controlled linked dragging and mixed shade/screen/periodic child contracts unproved. Existing Ball179941/39353 and Shade180048/97411 are not missing publication or complete gameplay proof. Shade-child180054's repeated101526 display has distinct scales/weights that must not be collapsed. No runtime/data admission, source-payload discard or new positive certificate.

## Source paired-trophy/oracle/dance/leash/anima-service boundaries (2026-10-17)

[Source260–264 matrix](../../toy-effects14-coverage.md#source260264-paired-trophyoracledanceleashanima-service-boundaries-2026-10-17) retains paired Doomwalker actor placement/lifecycle, authentic oracle fortunes, nearby haunting-melody dancing, permanent companion leash and outside-Torghast anima service gaps.264's selected source model is invisible11686/scale2, not zero-weight visible52318; no source slot/model/recipient substitution or metadata-only admission. No runtime/data write or new positive proof.

## Source return/music/disturbance/minipet-leash boundaries (2026-10-17)

[Source255–259 matrix](../../toy-effects14-coverage.md#source255259-returnmusicdisturbanceminipet-leash-boundaries-2026-10-17) keeps returning gormling destination/movement, ten-minute rooted harp music, pipe script/sound, disturbable casing and permanent minipet leash contracts unproved. Exact TDB actor presence/root support/zero dummy shape is not those combined actions or native sound. No inert actor/caster attachment/script no-op admission, runtime/data write or new positive proof.

## Source stacking/death-view/minipet/chase/multi-slime boundaries (2026-10-17)

[Source250–254 matrix](../../toy-effects14-coverage.md#source250254-stackingdeath-viewminipetchasemulti-slime-boundaries-2026-10-17) retains coordinated steward stacking, death-only viewer spirit-healer replacement, slime-minipet party appearance, ground-laser chasing and exact four-slime source-pool lifecycle gaps. Authentic source models and generic aura/summon support do not establish these combined contracts. No actor/viewer/minipet/script admission, runtime/data write or new positive proof.

## Source critter/passenger/reflection boundaries and retained hearth/vesper proof (2026-10-17)

[Source245–249 matrix](../../toy-effects14-coverage.md#source245249-critterpassengerreflection-gaps-and-retained-hearthvesper-proof-2026-10-17) retains authentic half-critter destruction, wounded-ally carrying and chained reflection-summon gaps. Stretcher properties5075 is Control0, with one zero-probability source model; no fabricated vehicle or arbitrary visible selection.246 linked cooking and249 stationary Vesper retain earlier exact CPU witnesses, not new per-toy native/audio proof. No runtime/data admission or repeated unchanged gate.

## Dominated helm targeted acceptance and coin/launch/linked/covenant boundaries (2026-10-17)

[Source240–244 matrix](../../toy-effects14-coverage.md#source240244-dominated-helm-targeted-acceptance-and-coinlaunchlinkedcovenant-gaps-2026-10-17) records240 test/spec c8feec2e actual175221/nine-display learning/ten-minute restoration/inherited recovery: two disguise/Oomgut tests/nine IDs pass91.61s after21m25s compile, with no compiler warnings and fmt/readability/diffcheck pass. Lead gate472 landed276 passed/masterca715f278; no own package-check repeat. Selected-target coin action, Maw-advertised health-sacrifice/self-launch, linked Krexus child/cleanup and Kyrian pledge acceptance remain gaps. Existing175442/display99668 does not prove linked child behavior. No runtime/data admission or new native certificate.

## Source advice/riding/sentry/atonement boundaries (2026-10-17)

[Source235–239 matrix](../../toy-effects14-coverage.md#source235239-adviceridingsentryatonement-boundaries-2026-10-17) retains minute-period dummy advice with no named child, wheelbarrow riding/eligibility, unwanted-entity warning and allied atonement gaps. Three authentic TDB actors exist but are absent from runtime content; Racer's VehicleId0 must not be replaced with an invented vehicle. No generic cosmetic/actor admission, new runtime/data write or positive simulation/native proof.

## Smelly Jelly and hover/mail/tea/moving-area boundaries (2026-10-17)

[Source230–234 matrix](../../toy-effects14-coverage.md#source230234-smelly-jelly-acceptance-and-hovermailteamoving-area-gaps-2026-10-17) adds234's actual authentic five-model transform/ordinary ten-minute restoration/one-hour recovery acceptance. Seven disguises plus Oomgut pass two tests/eight IDs99.85s; shared225/234 check2m25s/fmt/readability pass. Hover meditation, persistent friend-mail, goober tea-serving and moving Bonestorm area-trigger actions remain gaps. Source Sinvyr358569/type10/display67389 exists but is not runtime-published or interaction-proved; no inert-prop admission or new native certificate.

## Necrolord and retained cooking/quill boundaries (2026-10-17)

[Source225–229 matrix](../../toy-effects14-coverage.md#source225229-necrolord-source-acceptance-and-retained-cookingquill-scope-2026-10-17) adds225 test/spec e2d1b991 for the actual learned saved-bind/shared-charge path: ten variants pass52.90s; one shared final package check2m25s/fmt/readability pass.226 linked cooking/versatility and227 quill retain earlier exact CPU scope, not new native certificates.228 Venthyr pledge and229 Necrolord/Maldraxxus reduced-recovery acceptance remain gaps despite existing229 runtime model94055. No runtime/data admission or invented covenant rule.

## Source conditional-fall and reagent/random-action boundaries (2026-10-17)

[Source220–224 matrix](../../toy-effects14-coverage.md#source220224-conditional-fall-and-reagentrandom-action-boundaries-2026-10-17) retains black-parasol negative periodic-tick/refusal proof, not actual falling acceptance. Purple/green readiness consumption, Plume's combined slow-fall/turn/area contract and Necronom-i-nom's one-vellum/random-effect action remain gaps. No unconditional trigger, invented random pool, new runtime/data admission or repeated unchanged gate.

## Source stack/bobber/area/vehicle boundaries and vessel proof (2026-10-17)

[Source215–219 matrix](../../toy-effects14-coverage.md#source215219-retained-vessel-proof-and-stackbobberareavehicle-gaps-2026-10-17) keeps215's stack-conditioned shoulders/proc,217's actual bobber override,218's nearby forced-cry area action and219's mixed vehicle/motion/party-devouring graph open. Existing166086/display100400 content is not a missing-template fix.216 retains Batch3's exact learning/use/aura/cancel CPU scope without a fresh per-toy native certificate. No new admission or repeated unchanged gate; Tea Set149865 remains lead-required.

## Source visual/motion/return/class/covenant boundaries (2026-10-17)

[Source210–214 matrix](../../toy-effects14-coverage.md#source210214-visualmotionreturnclasscovenant-boundaries-2026-10-17) retains multi-visual eyes, water-walk/hover, player throw-return, class-armor payload and advertised Night Fae eligibility gaps. Shared2309/homebind metadata or ItemEffect condition0 is not complete Night Fae proof; no invented covenant rule or borrowed earlier hearthstone certificate. No runtime/data admission or new native result.

## Source area/vision/companion boundaries and Aqir proof (2026-10-17)

[Source205–209 matrix](../../toy-effects14-coverage.md#source205209-areavisioncompanion-boundaries-and-retained-aqir-proof-2026-10-17) distinguishes an area-trigger totem, void-perception payload, Control1/Title2 scale2 apple and minipet-target leash from generic scenery/caster cosmetics. Aqir retains earlier actual missile/selected-target/15s-aura CPU proof without new native certification. No runtime/data admission or repeated gate.

## Mogu and companion/equipment/game boundaries (2026-10-17)

[Source200–204 matrix](../../toy-effects14-coverage.md#source200204-mogu-source-acceptance-and-companionequipmentgame-boundaries-2026-10-17) records201's actual exact-source four-model transform/lifecycle/recovery acceptance: two affected tests/seven IDs, native check and fmt/readability pass. Companion possession, headgear procedural, shaving action and skull-game state remain gaps; existing half-scale shaving-kit runtime content is not that interaction. No runtime/data admission, generic substitute or new native certificate.

## Source motion/faction/appearance/action boundaries (2026-10-17)

[Source195–199 matrix](../../toy-effects14-coverage.md#source195199-motionfactionappearanceaction-boundaries-2026-10-17) retains self-launch/safe-fall, language/faction/self-sacrifice, soul appearance, cocoon action and nonzero shadow-disguise payload gaps. Scroll314988 is the actual TriggerType0 on-use; auxiliary314407/TriggerType7 must not replace it. No cosmetic/actor/transform substitutes, runtime/data admission or new native proof.

## Eternal Traveler and mount/control/portal boundaries (2026-10-17)

[Source190–194 matrix](../../toy-effects14-coverage.md#source190194-eternal-traveler-and-mountcontrolportal-boundaries-2026-10-17) adds Eternal Traveler actual learned-use/saved-bind/shared-charge acceptance: nine variants/check/fmt pass without runtime/data changes. Wild Holly mount/holiday, Control3 RC toys and Shadowlands portal service remain unresolved; source actor/visual metadata is not those actions or native certification.

## Midday source acceptance and interactive boundaries (2026-10-17)

[Source185–189 matrix](../../toy-effects14-coverage.md#source185189-midday-source-acceptance-and-interactive-boundaries-2026-10-17) records Jar187's actual-source day/night aura/cancellation/recovery acceptance: four affected tests/check/fmt pass after one cold-slot compile. Fish-kicking, Gral blessing and memento interaction remain gaps; Sealamp retains earlier exact body/scale/state-kit/natural-expiry witnesses without a new native run. No runtime/data admission or generic interactive substitutes.

## Midnight/sign proof and equipment boundaries (2026-10-17)

[Source180–184 matrix](../../toy-effects14-coverage.md#source180184-retained-midnightsign-proof-and-equipment-gaps-2026-10-17) keeps earlier Shadescale aura/cancellation and both warning-sign actor/native expiry scopes distinct from unproved midnight pixels, message-carving/extended visibility and dining/naga-equipment procedural behavior. No new runtime/data admission, invented interaction or repeated code gate.

## Memory transforms and collection boundary (2026-10-17)

[Source175–179 matrix](../../toy-effects14-coverage.md#source175179-retained-props-collection-gap-and-memory-transforms-2026-10-17) adds actual Unshackled/Ankoan learning, authentic pool membership, ordinary ten-minute restoration and inherited one-hour recovery acceptance. Eight tests/12IDs and check/fmt pass; no runtime/data change. Heart/Trophy retain earlier bounded proof, while the Mechagon figure collection/action remains unresolved despite an existing runtime base. Native bodies/weighted sampling/full recovery passage remain separate.

## Source standard/insignia/equipment/linked-hologram boundaries (2026-10-17)

[Source170–174 matrix](../../toy-effects14-coverage.md#source170174-standardinsigniaequipmentlinked-hologram-boundaries-2026-10-17) retains mixed Standard action, sanctuary script destinations, arm-equipment procedural and linked-hologram child gaps. Judgment154818/92155 already exists in the content tables WorldData actually loads; absent base-table rows are not missing runtime publication. Its linked child/lifecycle behavior remains unproved. No new admission, invented coordinates or borrowed War Banner/hearthstone proof.

## Holographic Hearthstone and banner boundaries (2026-10-17)

[Source165–169 matrix](../../toy-effects14-coverage.md#source165169-holographic-hearthstone-and-banner-boundaries-2026-10-17) records eight actual learned-use/saved-bind/shared-charge cases including Holographic165; native package/tests check and fmt/readability pass. Rustbolt333298 publication/placement and mixed Horde War Standard action remain gaps. Earlier source20 War Banner native/combat-removal witnesses retain their original scope; no new runtime/data admission or native certificate.

## Source robot/teleport/whistle boundaries (2026-10-17)

[Source160–164 matrix](../../toy-effects14-coverage.md#source160164-robotteleportwhistle-boundaries-and-retained-ocean-proof-2026-10-17) separates missing Blingtron7000/service behavior, unsourced Kul Tiras/Zandalar destinations and Flight Master's Whistle action from existing Pandaria code. Ocean Simulator retains its earlier revision-scoped simulation/native/PCM witnesses; no new acceptance, physical-output claim or confusion with Hyper-Compressed Ocean. No runtime/data admission.

## Source equipment/light/ocean/gift boundaries (2026-10-17)

[Source155–159 matrix](../../toy-effects14-coverage.md#source155159-equipmentlightoceangift-boundaries-2026-10-17) preserves exact equipment/gift procedural fields, paired light behavior and the Burning Blade banner presentation gap. Hyper-Compressed Ocean152121 remains unpublished; its actual source body is an invisible stalker, not proof of a fishable pool. No generic substitutes, new admissions or native acceptance.

## Azerite real-ground recovery and interaction gaps (2026-10-17)

Already-admitted Azerite151 now has actual Northshire ground/learning and ordinary600s item recovery/reuse acceptance, without new policy. [Exact four-test/check proof and150–154 matrix](../../toy-effects14-coverage.md#source150154-azerite-real-ground-recovery-and-interaction-gaps-2026-10-17) separate server events from native skyward presentation and retain matching-game, rally, jukebox and secret-fish interaction gaps. No database publication.

## Procedural/control/script and Tea Set gaps (2026-10-17)

[Source145–149 matrix](../../toy-effects14-coverage.md#source145149-proceduralcontrolscript-and-tea-table-boundaries-2026-10-17) retains the blight-coating procedural payload, Control4 spinning toys, ambiguous Highborne possession action and unpublished149865 Tea Set. No generic visual/actor substitute or new admission; authentic Tea Set model metadata is not successful gameplay/native proof.135 remains a separate established source contract.

## Brewfest hearthstone and corpse/appearance boundaries (2026-10-17)

Brewfest Reveler140 now has actual saved-bind/shared2309 acceptance alongside six prior holiday roots. [Seven-case proof and exact141–144 boundaries](../../toy-effects14-coverage.md#source140144-brewfest-hearthstone-and-corpseappearance-boundaries-2026-10-17) distinguish enemy-corpse fireworks, the mixed Transmorpher child/service chain and dead-target soul trapping from generic handlers. Runtime/data unchanged; native Brewfest presentation and full recharge remain separate.

## Goldtusk table and Fire Eater source contracts (2026-10-17)

Source135's maintained import and actual-use omissions were reproduced before adding exactly149876 with its authentic90570/half-scale model. New source139 follows the existing saved-bind/shared2309 path;136 bowl retains its Group28 contract, while137–138 RC possession stays unimplemented. [Exact nine-test/importer proof, source/native exclusions and copy-only publication receipt](../../toy-effects14-coverage.md#source135139-goldtusk-table-retained-bowl-rc-gaps-and-fire-eater-2026-10-17) distinguish the added data from Rust handler changes, food interaction and native presentation. The lead completed canonical publication after landing; the linked coverage owns the exact receipt and read-only publication acknowledgment.

## Source companion/healer/corpse/music boundaries (2026-10-17)

[Source130–134 matrix](../../toy-effects14-coverage.md#source130134-companionweaponhealercorpsemusic-boundaries-2026-10-17) preserves the owned-critter riding target, procedural anchor weapon, Brynja viewer payload versus healer service, fallen-ally forced-child semantics and missing music-box publication/behavior. Actual TDB music-box body resolves to the existing chest01b model; that does not authorize invented presentation/audio. No runtime/data changes or new acceptance claims.

## Noble Gardener and source anchor boundary (2026-10-17)

Noble Gardener125 now has the actual saved-bind/shared2309 contract alongside four prior holiday roots. Citizens126 remains refused at the source child's permanent Personal Anchor dummy; its new actual-learning refusal test proves no partial actor/effect or recovery spending, not a successful summon. [Exact source124–129 matrix and retained proof scopes](../../toy-effects14-coverage.md#source124129-garmentanchoraction-gaps-and-noble-gardener-2026-10-17) distinguish these from native garment/surfing/zero-entry/script appearance gaps. No production admission.

## Scarab target32 grounded oracle (2026-10-18)

Native92146 is SUMMON28, implicit32+0 and Attr9=0. TC Spell1614/1640–1641/9286–9292 selects MovePositionToFirstCollision, whose Object2904 calls UpdateAllowedPositionZ. Object709–748 samples height once from the computed pre-clamp elevation. Therefore the first Scarab retains the post-grounding destination, not raw `effect_dest` Y. After hi609, the old pre-grounding oracle became stale; production grounding is correctly scoped.

The corrected test independently samples the same selectedXZ/pre-clampY ray, requires this actual slope to distinguish old/new height, retains exact horizontal coordinates and strict floor bits, and leaves count/scatter/follow/lifetime checks unchanged. Ignored REDbe5cd97ae reproduces the original first-destination failure39.32s; corrected strict oracle and retained controls pass17/17 144.23s at206b095ed (master641cb2460/protocolf9a9175), including10 Herald, four grounding, two Scarab and the toyfx393 prefix-matched Hearthstone. Independent full-Toy gate remains LEAD-owned. No production code changes. [Contract](../../specs/toys.md), [target32 grounding](../../specs/ramparts-herald.md#grounding-only-target32-prerequisite).

## Scarab source scatter metric (2026-10-17)

The first guardian retains the exact selected destination after target32 grounding (see above); secondary placements use a horizontal5yd disk with independent ground-projected elevation. A3D-distance assertion incorrectly rejected valid sloped-ground points. [Merged-artifact30-repeat reproduction, deterministic actual-terrain RED and exact scoped GREEN](../../toy-effects14-coverage.md#gate412-scarab-source-ground-metric-2026-10-17) retain5.001 unchanged and strengthen the height/first-point checks. Runtime unchanged; merged verification remains lead-owned.

## Dragon costume simulation lifecycle (2026-10-17)

Six already-authentic dragon costumes now have actual learned-use/source-display acceptance through ordinary fifteen-minute expiry/restoration and natural recovery/reuse. [Exact118–123 identities, eight-test bounded proof and fixture-failure distinction](../../toy-effects14-coverage.md#source118123-dragon-costume-simulation-lifecycle-2026-10-17) separate simulation state from unproved native costume/group presentation. Production and canonical data unchanged.

## Oomgut source transform lifecycle (2026-10-17)

Actual163795 learned use already works with132902; new test proves source display membership, ten-minute ordinary-frame expiry/model restoration and reported two-hour recovery remaining active afterward. No production patch or invented RED. [Exact scoped14-case proof and exclusions](../../toy-effects14-coverage.md#oomgut104-existing-transform-lifecycle-2026-10-17) distinguish simulation acceptance from all17 native appearances and wall-clock recovery. [105–108 boundaries](../../toy-effects14-coverage.md#source105108-controlheadwearbanner-boundaries-2026-10-17) retain RC possession/headwear gaps and prior Hero Banner proof.

## Source disguise recovery precedence (2026-10-17)

Trogg164373, Hozen164374 and Skeletal Monkey164375 already have authentic transform entries. New actual-use tests prove ten-minute ordinary expiry/restoration and item override versus inherited spell recovery, without production changes. [Exact four-ID final comparison, source pools and exclusions](../../toy-effects14-coverage.md#source109113-bannerscript-boundary-and-disguise-lifecycles-2026-10-17) separate server state from weighted/native appearance certification; Glorious Pennant110 remains an unresolved script.

## Lunar/Peddlefeet hearthstone learned use (2026-10-17)

Two additional holiday roots use the same persisted-bind/shared2309-charge path: missing bind refuses before spending, source10s cast returns to the saved same-map location, spending blocks its sibling. [Exact5-test/seven-ID gate and source visual boundaries](../../toy-effects14-coverage.md#source114117-hidinggift-gaps-and-holiday-hearthstones-2026-10-17) retain procedural/native/cross-map/recharge gaps; no production patch. Rhan'ka hiding114/Akunda gift115 remain unresolved actions.

## Forlorn Toll destination-only flight (2026-10-16)

163463/279223 triggers279222 after source1.5s cast+3s fixed missile flight, then one143222 source soul. Genuine importer RED precedes exact one-entry/ten-model admission; actual learned-use RED finds early arrival because front47 was unrecognized and unit-only delay storage discarded unitless destination flight. The existing scheduler now retains that delay without a fake target or toy-ID special case. Real cast/flight, source display membership/front,30s recovery/reuse and coexisting independent60s lifetimes pass. Whole-Toy150/150, affected49/49, importer3/3 and check/fmt pass. [Exact revisions, current-copy publication receipt and boundaries](../../toy-effects14-coverage.md#forlorn-toll85-source-summon-2026-10-16) distinguish server timing from all-ten native appearances, ghost/wandering semantics and complete-toy parity; canonical template publication remains lead-owned.

## Akunda source child and visible trap (2026-10-16)

The queued64 cursor regression and captured-context fix are tracked in [spell casting](spell-casting.md#queued-effect64-captured-destinations--2026-10-18); its current proof limits apply to the destination chain below.

163211/279076 first refuses linked trap302746; exact source type6/display16675 profile now joins31442/display0 without generic trap admission. A separate genuine child-position RED finds279077 at the caster rather than clicked point; existing effect64/32 unit-trigger routing now retains the parent's explicit destination/item level through its depth-checked loader/launcher. Actual learned-use root/child/visible-source trap, ordinary recipe consumption/output, environmental CozyFire refresh, five source-drift refusals and ordinary300s cleanup pass. Whole-Toy149/149, affected29/29 and check/fmt/manual readability pass. [Exact proofs and source boundaries](../../toy-effects14-coverage.md#akunda84-visible-linked-trap-boundary-2026-10-16) separate server behavior from native pixels/audio, broad trigger completeness and full-retail parity;82 missing ItemSparse/83 specialized title27 remain named gaps.

## Gnoll Barrel source pair (2026-10-16)

163201/279063 source templates143119/143120 were absent after maintained import: genuine importer RED precedes the exact two-entry source admission. Idempotence/rollback controls and actual learned-use pass: real area5759 eligibility, authentic map1643/MCNK9042 cursor pair38854/38857, unchanged faction35/nonpet/nonfollow, ordinary300s simulation expiry and30min recovery. Current whole-Toy147/147 and check/fmt/manual readability pass. [Exact source, revisions, copy-only canonical receipt and claim boundaries](../../toy-effects14-coverage.md#gnoll-barrel81-authentic-template-admission-2026-10-16) distinguish data/lifecycle acceptance from missing native533501 body, indoor/WMO eligibility and unproved target-dummy combat. Canonical publication remains lead-owned; Cursed Vision80 remains a named remote-view gap.

## Holiday hearthstone learned use (2026-10-16)

Greatfather Winter162973/278244 and Headless Horseman163045/278559 use persisted HomeBind, source10s cast and shared charge2309; missing bind refuses before spending and either toy blocks its sibling after spending the sole charge. Two independent actual learned-use tests pass; current whole-Toy146/146, check/fmt/manual readability pass. [Exact evidence and exclusions](../../toy-effects14-coverage.md#holiday-hearthstones7879-learned-use-boundary-2026-10-16) separate same-map server acceptance from source type17 procedural appearance, native wire/render/audio, cross-map transfer and natural30min recharge.

## Corgi and Rezan source boundaries (2026-10-16)

Corgi158149/264091 learned-use/cancel passes with literal viewer233 payload[3569,53], not a global ModelDisplay change; native viewer interpretation/rendering remains absent. Roar160509/272071 genuine learned-use effect3 refusal precedes exact audited cosmetic admission; focused cast/source90s recovery/reuse and unchanged health/model/no aura/summon pass. Native catalog resolves the authored one-shot caster animation74/sound107006; authenticated1777497 bytes are not audible playback. [Exact coverage and proof boundaries](../../toy-effects14-coverage.md#roar71-source-one-shot-cosmetic-2026-10-16) retain revisions, passing144/144 whole-Toy gate and named Chili69/Enthralling70 gaps; no invented fire/critter-following implementation or full-toy claim.

## Stampwhistle source assistant and mailbox (2026-10-16)

156833/261602 summons source132969/display68512 through property4203 Control1/Assistant35. Native default TempSummon routing preserves source flags/faction/non-follow identity,3yd front placement and600s duration; source item/category2066 recovery is3h. Authentic mailbox flag opens ordinary mail service directly, not an invented gossip option. Separate genuine importer/journal/mailbox REDs and source-state diagnostics are retained; focused real service/learned-use/source600s lifecycle now passes2/2 after fixture per-frame reply collection. Whole-Toy143/143, affected mail/NPC/summon49/49, importer controls3/3, binary/tests check and fmt pass at recorded66 revisions. Canonical classification370 still refuses missing132969; prepared392 admits Katy but is not complete-toy coverage. [Exact coverage](../../toy-effects14-coverage.md#stampwhistle66-source-assistant-and-mailbox-2026-10-16) distinguishes full simulated expiry/live service from native assets/rendering and pending lead-owned canonical publication.

## Sightless detection consumer (2026-10-16)

Sightless Eye153293/254568 already has its target aura152 consumer in native `AggroDistance`. Real hostile-acquisition controls prove ordinary12yd acquisition versus protected12yd non-acquisition and protected7yd acquisition before registry admission. Actual journal RED rejects152;574d45f5 recognizes that existing consumer without changing aggro math. Actual bag learning/UseToy and native aggro pass2/2; source group5454/instant10min aura/5min recovery and ordinary expiry remain unchanged. Whole-Toy142/142, creature-aggro30/30, current binary/tests check and fmt pass at recorded65 revisions; canonical classification adds only153293 to370, not370 complete toys. [Exact revisions and proof boundaries](../../toy-effects14-coverage.md#sightless65-native-detection-consumer-2026-10-16) distinguish server behavior from native rendering/wire and preceding unimplemented remote-view/possession roots.

## Waywatcher source lifecycle (2026-10-16)

130249/195806 consumes the currently published source99572 template, choosing63609or63606 without a new importer admission. Actual bag learning/UseToy acceptance passes1/1: terrain-backed Val'sharah7558 admission/Northshire refusal, source instant cast/15s duration/+200% speed/241 and900s recovery, then ordinary expiry restores native display/health/flags/speed. [Exact proof and remaining boundaries](../../toy-effects14-coverage.md#waywatcher64-source-lifecycle-2026-10-16) separate this existing-handler acceptance from eighteen authenticated privately published body assets/four aliases. Actual private Rust loader, whole-Toy141/141, server binary/tests check and fmt pass at recorded64 revisions. Native rendering remains separate; no manufactured RED or canonical publication claim.

## Grizzlesnout source lifecycle (2026-10-16)

129965/194807 uses existing transform27864/display9287, aura241 and+250% run speed for15000ms, with600000ms item recovery. Actual bag learning/UseToy acceptance passes1/1 after62: real Northshire terrain refuses group4446 without spending; real Northrend571/3537 accepts the1500ms cast. Ordinary50ms simulation expiry restores native display/health/flags/speed without clearing recovery. This is a new acceptance test of existing behavior, not an invented runtime RED. [Exact proof and remaining boundaries](../../toy-effects14-coverage.md#grizzlesnout63-source-lifecycle-2026-10-16) separate server lifecycle from twelve already-owned authenticated canonical body assets/two aliases. Actual canonical Rust loader, whole-Toy140/140, server binary/tests check and fmt pass at the recorded63 revisions. No fresh wire/pixels or complete-toy claim.

## Forced-forward flag boundary (2026-10-16)

TC `HandleForceMoveForward` reconciles aura241 into `UnitFlags2::FORCE_MOVEMENT` (0x40), preserving every unrelated bit and retaining the flag until the last241 aura leaves. Genuine source-spell148626/194807 RED observes1 instead of65; the committed handler and shared constant pass the overlap/final-removal control1/1 in6.08s at serverb0a24e7d/protocol07a8faa. Native input now passes35/35 gameplay controls after genuine idle-input RED, and whole-Toy passes139/139 at server2d8fc58e/engined6adbe444/protocol07a8faa. [Exact packet scope and named gaps](../../toy-effects14-coverage.md#forced-forward62-generic-input-proof-2026-10-16) separate generic flag/input support from missing104329 mount template73689 and unresolved113543 mount action; no fresh wire, native-pixel or complete-toy claim. [Contract](../../specs/toys.md#contract). Exact stage ledger: `/home/osso/.worktrees/logs/toy-effects14-continuation/force-forward62-proof-ledger.json`.

## Ysera source summon boundary (2026-10-16)

Ysera's Tear130170/195708 summons one99518/display66207 with property3054: private to its owner, ordinary non-pet wild actor, source47's8yd front offset and120000ms expiry. Authentic template publication was missing: real importer RED finds zero99518; actual learned UseToy RED refuses missing99518. The maintained importer now admits only that source entry; existing placement/privacy/level/lifetime handlers are unchanged. [Contract](../../specs/toys.md#contract). Six importer controls and the real learned-use lifecycle pass; whole-Toy138/138 and current check/fmt pass after a module-order-only correction. [Exact proof](../../toy-effects14-coverage.md#ysera-source-summon-proof-2026-10-16) records literal revisions, private seventeen-asset body closure and prepared classification385/1166, not complete-toy coverage. Native state kit60600→procedural8605/type22 is currently ignored by the renderer; body asset closure and server acceptance do not establish that appearance or complete-toy parity.

## Enchanted Stone Whistle source summon (2026-10-16)

129279/193687 has one source wild/default summon of Petrified Sand Piper98300/display71056. The maintained additive importer copies its authentic template/model; existing63→86 destination selection, ownership, ordinary wild initialization and60000ms expiry remain unchanged. No AI/addon/spell/click/root-script binding supplies another action. [Exact server proof and native gaps](../../toy-effects14-coverage.md#enchanted-stone-whistle-server-proof-2026-10-16) distinguish server acceptance from native assets/pixels; guardian final HP remains read-only.

## Shrine server per-target stat conversion (2026-10-16)

A replicated viewer range alone is insufficient for server `UnitScaling`, which pairs it with per-level `ScaledStats`. Wild initialization now builds that existing table from final summon tuning and source difficulty modifiers; raw DamageModifier is retained for regionally tuned templates too. Actual learned Shrine RED observes server level30/armor155 against viewer20; nineteen focused controls now pass level20/armor135 and source damage/health conversion outputs while Podium stays unscaled. [Exact output matrix and proof scope](../../toy-effects14-coverage.md#shrine-server-per-target-stat-conversion-2026-10-16) distinguish this actor-state CPU proof from actual combat hits, client HP display, guardian stats and native rendering.

## Shrine viewer-level projection (2026-10-15)

UseCreatureLevel retains the initial source regional `LevelScaling`; wild initialization carries its range and once-rolled delta instead of dropping them after native selection. Unflagged player-owned Podium inherits zero tuning and stays native for every viewer. Actual learned Shrine use reproduces viewer20→30, then passes viewer1/20/80→1/20/30 while preserving native30 and initialized stats. [Claim-specific evidence and open server-combat/HP-projection boundaries](../../toy-effects14-coverage.md#shrine-viewer-level-projection-2026-10-15) own verification scope; no complete combat or new native-rendering certificate.

## Wild summon initial level and stats (2026-10-15)

Pinned TC a352b1fa initializes actual terrain area in Creature::Create before CreateFromProto→UpdateEntry→SelectLevel; TempSummon::InitStats then clamps all unflagged summons, not only guardians. Wild E28 reads the same strict destination-level projection before normal zone spawning, then initializes selected UnitLevel/HP/armor/mana before spell summon bookkeeping. Missing regional or required mana-source rows refuse spawning.

UseCreatureLevel retains source template/map/area tuning for initial stats; unflagged TempSummon inherits its owner's tuning (source zero-initialized when absent). Owner level comes from player Level or non-player UnitLevel; absent state is not a guessed1. This also corrects the existing learned-use group chain's temporary NPC child. Ordinary zone spawning, guardian stat formulas, AI, placement, privacy and lifetime paths are not redesigned.

[Source rows, genuine REDs, exact whole-Toy gate, current build/checks, private initialized-field wire proof and binary hashes](../../toy-effects14-coverage.md#wild-summon-initial-state-proof-2026-10-15) own acceptance evidence. Initial-state CPU and fresh Podium/Shrine/default-Scarab wire now pass. [Shrine client-level projection](#shrine-viewer-level-projection-2026-10-15) is separately proved; inherited server-combat scaling/modifiers, attack power and full creature-stat parity remain open; previous native pixels/lifetimes are not recertified. [Contract](../../specs/toys.md).

## Winner's Podium source placement (2026-10-15)

239018/1227063 selects static240220/display127527 through47→138. Actual-source RED reproduces1.25yd instead of native player reach1.5yd, airborne82.5378 instead of terrain80.52305, and real bag-learning/UseToy refusal for missing240220. The additive source importer now includes240220; player-front47 clamps to the existing sourced scaled reach, and the mutable cast-plan ground step resolves138 alongside62. Non-player front reach is not redesigned. Source TDB has no AI/script/spell/click/vehicle binding and properties64 has no flags; no interaction is inferred.

Initial six controls pass66.51s at3c2d0bb6; post-master packet20/20 passes40.01s, including Guardian51 and prepared census. Final pure clamp extraction377d5139 passes the affected six again58.66s plus server check/format/maintained build; unchanged engine/protocol gates retain their exact scope. Actual Podium learning/use, owner/map/body/health/non-pet/static placement and1200000ms simulation expiry use the existing50ms harness unchanged. Three player scales, terrain grounding and retained62/Bashiok/Shrine87 are covered. No cooldown/max-delta/runtime-clock reset.

Both native skins pass fresh learning/journal activation, source127527/6392183/two meshes/scale1/alpha255, unchanged native owner and46500 health, no pet frame and static actor after real14yd owner movement. Original1920×1080 frames show the podium at the authored Northshire spawn, without changing gameplay's camera or positioning actors for the photograph. [Exact native revisions, hashes, private8-asset provenance,383-row prepared census and remaining boundaries](../../toy-effects14-coverage.md#winners-podium-source-placement-and-native-proof-2026-10-15) distinguish these observations from an unproved20min native wall lifetime or complete1166. [Contract](../../specs/toys.md).

## Runtime generated casting data (2026-10-13)

[Runtime data guide](../../toy-runtime-data.md) owns producer commands, source-build provenance and deployment sync. Startup loads both required JSON artifacts into an ECS resource and refuses missing data. Process tests reject each missing file and load two controlled builds with the same binary; fresh-app tests verify changed facing behavior. Production generated data is no longer embedded in Rust source.

## Authored script dispatch regression (2026-10-13)

[Coin ordering regression](../../toy-effects14-coverage.md#coin-dispatch-regression-recheck-2026-10-13) records the clean-master counterexample, introducing commit and retained source-specific fix.

## Prismatic Bauble source boundary (2026-10-13)

Exact223143 cosmetic admission has authoritative lifetime/recovery proof, not native rainbow certification. Source ribbon parsing and actual native delivery/lifecycle gaps remain separate. [Claim-specific coverage](../../toy-effects14-coverage.md#prismatic-bauble--source-admission-native-gaps-open-2026-10-13).

## Source facing boundary (2026-10-13)

Two source-facing roots use selected-unit180° arc validation and TC's strict exact3D target-radius exemption before spending. No CombatReach extension; ordinary start GCD is retained on completion refusal. [Sources and claim-specific proof](../../toy-effects14-coverage.md#source-toy-facing--bounded-gate-2026-10-13); [contract](../../specs/toys.md).

## Source area-group lifecycle (2026-10-13)

Cast start and completion consume the pinned Toy root manifest; actual terrain-area lookup and source ParentAreaID ancestry precede cooldown/effect spending. Player-owned restricted auras use ordinary interrupted removal on exit; within-group travel retains them and category history is unchanged. [Source, exact boundaries and proof](../../toy-effects14-coverage.md#source-requiredareasid-lifecycle-2026-10-13); [contract](../../specs/toys.md#source-area-group-closure--active2026-10-13).

## Current PvP faction and War Banner proof (2026-10-13)

`CurrentPvpFaction` carries the authoritative source condition value independently of faction reactions. Login derives the known racial value; accepted BG/arena entry assigns team after successful teleport; successful return restores it. Invitations and refused source transport entry do not assign the team prematurely. Client parser/codec consumes this scalar for36349/36348. Exact sources, coverage matrix, census and private asset boundaries live in [War Banner coverage](../../toy-effects14-coverage.md#current-pvp-faction-and-war-banners-2026-10-13); [match contract](../../specs/battleground-lifecycle.md#current-pvp-faction) owns lifecycle requirements.

## Source summon flag import (2026-10-12)

Canonical read-only inspection found3610 SummonProperties rows with only ID/Control/Faction/Title/Slot, despite the maintained DB2 schema already naming `Flags_0`/`Flags_1`. Local CASC FDID1345276/layoutA4CA5ECF and pinned TC DB2LoadInfo supply both words for3610 readable records; three encrypted records are explicitly omitted. `scripts/import_summon_property_flags.py` validates every existing source identity, transactionally adds the missing columns and populates flags without rewriting identities/unrelated data. Legacy3001 Title4294967295 equals the source signed32 -1 bit pattern; validation normalizes bits, not stored data.

Private visibility is a separate runtime step: TC `SpellEffects.cpp::EffectSummonType` uses OnlyVisibleToSummoner0x10 and OnlyVisibleToSummonerGroup0x10000 to set a private owner; `Object.cpp::CheckPrivateObjectOwnerVisibility` filters seers. Source3077 used by the four warning signs carries0x10, while the existing summon/interest path ignores it. Importing flags alone is not a visibility fix or a new working-toy claim. The owner-only runtime path reads `Flags_0`, attaches an unreplicated `PrivateObjectOwner` before interest registration, and caches that owner in the existing spatial interest index. Normal visibility transitions therefore omit private replicas for other player connections and remove cached ownership with the target. This does not mark other flag semantics implemented; group-private graphs formerly retained a preflight gap; the [group-private runtime](#source-group-private-runtime-2026-10-13) now handles this visibility flag. Real two-peer UDP RED receives source-private73781 in both replicas while public66768 is visible to both. The first change missed both target-dirty visibility and the separate wild/default `zones::summon_creature` path; missing spawn traces falsified the assumed path. Wild spawns attach the same private ownership marker, and its Add observer updates/invalidates interest after the zone spawn's initial registration. Actual UDP GREEN with source-private Lilian88280 now has owner=true/nearby observer=false while public66768 remains true for both. Source owner model92597/Shirakess170196 also preserves four meshes,.6 scale/no pet frame and natural60s expiry in fresh Modern/Forever native processes (observed59405/59742ms after model readiness). Requested framed owner PNGs inspected; no two-rendered-client claim. Lead canonical command, once requested:

```text
python3 scripts/import_summon_property_flags.py --db2 /home/osso/.worktrees/logs/toy-effects14-continuation/runtime/data/summon-properties-source/1345276.db2 --db data/world.db
```

## Linked cooking traps (2026-10-12)

Source-limited `game_objects::cooking_traps` loads31442 plus addon35, spawns a separate unowned trap at its summoned focus and routes7353 through ordinary triggered spell selection. Environmental activation searches living same-map players within diameter/2 plus scaled combat reach; the spell independently selects10yd allies. Charges0 repeat at the source TC default4s. Parent expiry/removal despawns the linked trap before another refresh. The snapshotted versatility aura keeps its60s duration even after its GO caster disappears; rating-only snapshot retention fixes the observed unconditional casterless-removal failure without claiming other aura/source combinations.

[Exact seven identities, source capsule, RED/GREEN, census337 and representative both-skin proof](../../toy-effects14-coverage.md#linked-cooking-traps-2026-10-12). Leylight253080 missing source metadata and Akunda's Firesticks163211 multi-object chain remain separate; no generic trap or extra focus-field admission.

## Toy-effects14 ordinary caster visual kits (2026-10-11)

Largest remaining group is aura4(273 distinct toys). Complete source audit separates96 zero-shaped bound roots from27 ordinary-model native kit chains, then12 explicit caster-only cosmetic contracts. The unchanged complete-zero predicate admits those12 source-bound spells; it does not accept arbitrary dummy auras. Models/animations/sounds use existing native kit types2/5/6, with no procedural, positioner or conditional payload asserted. Real bag learning/UseToy, exact replicated lifetimes, cancellation and unchanged health/global models are tested. [Exact12 identities/source/gaps](../../toy-effects14-coverage.md#batch3-highest-count-aura4-complete-ordinary-model-caster-kits).

## Toy-effects14 zero-entry costumes (2026-10-11)

TC a352b1fa `HandleAuraTransform` supplies explicit mappings, not creature-entry joins, for Murloc42365/display21723, Gnomeregan75531/displays31654–31655 and Darkspear75532/displays31737–31738. The native player sex selects the two Pride displays even while another transform is active. Existing transform precedence and native-display restoration apply. Toy preflight accepts only those implemented zero-entry cases; other zero-entry costumes remain explicit refusals. [Source and proof](../../toy-effects14-coverage.md).

## Toy-effects14 item selection (2026-10-11)

Toy.db2 membership was absent from the general item-selection union, so 843 source-resolved toy items never reached `content_item`. Of these,156 already pass spell-graph preflight; that is not native-effect certification. General content rebuilding now retains source-resolved Toy members. The additive `scripts/add_toy_summon_templates.py` uses the same `content_policy.ITEM_COLUMNS` conversion, preserves existing runtime rows and still refuses missing required scenery sources. Unresolved Item/ItemSparse rows remain unresolved. No spell, script, payload or coordinate is manufactured.

Prepare only a disposable/approved copy: `python3 scripts/add_toy_summon_templates.py --db data/world.db.new`. This task does not promote canonical world.db. Real-ID tests cover source equality, idempotency, missing-metadata preservation, full item rebuild and Bones of Transformation64646/91760 learning/use/display32473/cancellation restoration. Proof and remaining mechanics: [coverage](../../toy-effects14-coverage.md).

## Toy-effects12 non-summon cosmetics (2026-10-11)

Non-summon runtime-item-eligible ranking starts with aura4=35, effect3=31, effect77=16, effect50=11 and aura56=8 (overlapping counts). Extend the existing shape-checked cosmetic audit, not a blanket dummy executor. Heartbreaker71909 has animation1684/3924 and heart attachment3014; Death's Door193333 has ghost-state model1249985; B.B.F. Fist408067 has animations398982/398986 and hand model978213; Abyss Caller430760 has seven authored water/ritual attachments plus animation kits. Source lifetimes are60000/1800000/86400000ms for the three auras; the horn has no aura duration, gameplay water or summoned actor. No timer is manufactured. All four have complete zero-payload dummy shapes, source visual chains, and no selected spell_script_names bindings in the actual TDB1210 archive.

The Heartbreaker previously deferred in toy-effects10 is now source-audited against its complete animation/attachment chain. Procedural Frosh Hat, zero-entry transforms, missing Black Ice/ring visuals, regional/seasonal scripts, returning balls and interaction objects remain unavailable. Wayfinder's Compass requires Type2 GenericID110576/positioner1937 rendering; it is not an ordinary model attachment. [Coverage and exact gaps](../../toy-effects12-coverage.md). Native rendering is not certified by server-preflight/CPU tests.

## Toy-effects10 cosmetic target and growth auras (2026-10-10)

Ranked prepared-copy runtime blockers: summon28=43, aura4=42, effect3=31. Missing summon templates do not establish interactions/control scripts. Three source-complete aura4 actions extend the unchanged complete zero-valued cosmetic predicate in `spell_cast/cosmetic.rs`; the root Aqir missile remains ordinary trigger32, not an audited dummy.

|Item / root|Audited aura|Source lifetime / target|Local visual evidence|
|---|---|---|---|
|Iron Belly Spirits88587/128327|128327|1000ms cast;60000ms aura / friendly21|visual27023, cast kit23617 and impact9579, animation829 and sound101465; no resource restoration|
|Darkshard Fragment131812/196783|196783|5000ms / caster1|visual51069, crystal model1249984 via kit61005/positioner36; cast392109 via61004|
|Aqir Egg Cluster175063/318453|318452|15000ms / target25|authored missile32 triggers318452/visual94332; kit123945 models1249925,1733424,165723 plus sound147889; no creature spawning/damage|

Local Retail12.1.0.69933 DB2 rows and the actual TDB1210 archive have no selected spell_script_names binding. Descriptions and full visual chains, not absence of bindings alone, establish these cosmetics. Heartbreaker/emote actions, procedural Frosh Hat, Wormhole menus, Gokk'lok's Shell/nonzero dummy, scouting maps and scaled Flame's Radiance stay blocked. No new importer/content changes: prepare toy-effects9 templates on a disposable copy with its existing importer. Behavioral tests exercise real bag learning, UseToy, target-vs-caster routing, missile-triggered aura, unchanged health/model, source lifetimes and retained refusals. Native rendering/audio/positioners are not certified by server proof.

Sources: server `data/diagnostics/toy-effects10-2026-10-10/{selected-source.json,visual-source.json,spell-script-names.sql,before.json}`; [contract](../../specs/toys.md), `crates/server/src/toys_tests.rs`. Prepared-copy census125→128/1166, added only88587/131812/175063 and removed none. Behavioral RED4 aura4 refusals; final GREEN4 including source1000ms cast/8yd missile delay, near-expiry existence and removal. First post-master5e858c8 gates passed349 spell_cast/47 toy tests. Before-push fetch then found source-changing master3953eb3 (class-spell batch15); merged featuref838340 passes360 spell_cast/47 toy tests and format/check warning-free. The second gate is required by that new spell-execution scope, not a repeat on unchanged code. Canonical data was also prepared externally during the task: initial scratch import added16 toy-effects9 templates; final read-only canonical census is128. This branch never wrote canonical world.db. Durable receipts and exact revisions: `/home/osso/.worktrees/handoff-toy-effects10.md` and the linked diagnostics.

## Toy-effects9 stationary banners and sign (2026-10-10)

Missing runtime items and effect28 templates, not absent generic summon execution, blocked this source-complete group. `scripts/add_toy_summon_templates.py` now adds15 Retail banner item definitions using `content_policy.ITEM_COLUMNS` and16 authentic TDB1210 templates/models (15 banners plus Artisan's Sign). Full content-policy rebuilding retains those selected item/template identities. No world.db mutation is part of this branch: run `python3 scripts/add_toy_summon_templates.py --db data/world.db.new` on an approved copy, then let the owner promote it separately.

|Item|Spell|Creature|Display|
|---|---|---|---|
|163986 Orgrimmar Hero's War Banner|280827|144025|88125|
|163987 Stormwind Champion's War Banner|280828|144026|88126|
|187834 Tormented Banner|360123|181826|102568|
|187957 Encrypted Banner|361085|182459|103372|
|187958 Shrouded Banner|361090|182461|103384|
|197719 Artisan's Sign|381262|193182|108105|
|203716 Thundering Banner|401253|201798|111503|
|208057 Smoldering Banner|417591|208477|113602|
|211424 Dreaming Banner|429364|214220|115934|
|218128 Draconic Banner|442240|220620|118153|
|232301 Tempered Banner|469612|233144|123650|
|232302 Prized Banner|469613|233145|123654|
|232303 Unbound Banner|469614|233146|123655|
|232305 Forged Champion's Banner|469616|233148|123656|
|232306 Prized Champion's Banner|469617|233149|123658|
|232307 Astral Champion's Banner|469618|233150|123659|

All selected spells have exactly one summon effect, Control0/Title0/Slot0, no creature AIName/ScriptName/npcflags/gossip/SmartAI action or spell_script_names binding. Complete reference tuples match the actual local `TDB_full_world_1210.26091_2026_09_09.sql`:16 templates,16 models,21 difficulty rows and0 addons/SmartAI rows. Retail descriptions promise only planting a banner or placing this sign. This does not justify importing Lilian's editable sign, drinks, seats, wormholes or firework opt-in actors. Flame's Radiance Banner238852 has model DisplayScale0.75 and stays deferred; no scaled-render proof is inferred.

Banner target47 preserves radius16=1yd for war banners and radius7=2yd for seasonal banners. Target138 uses the current destination; UseToy has no ground-pick field, so Artisan's Sign lands at the caster after its2000ms cast. Banners last60000ms, sign300000ms. Wild summons keep source faction35 for the two war banners and0 for the other selected scenery, not caster faction. War-banner ItemSparse faction/race masks remain intact.

After merging current origin/master615fb90 into the feature branch, once-only gates pass: spell_cast349/349, toy43/43, importer3/3, format/check with no warnings. The real `tdb_item_ids`→`build_items` rebuild on a disposable copy retains all15 banner items (`content-rebuild.json`). Imported-copy census109→125/1166, +16 and no removals; unchanged shared database109. Behavioral fixtures exercise actual bag learning/UseToy, faction restrictions, SpellGo, one authentic actor/model, destination, ownership/map, stationary position and expiry. Importer tests cover source-row preservation, repeated CLI idempotency and atomic rollback for missing source models/items. Evidence/proof ledger: engine `data/diagnostics/toy-effects9-2026-10-10/` and `/home/osso/.worktrees/handoff-toy-effects9.md`. Server-preflight/CPU proof is not rendered Retail-client parity. [Contract](../../specs/toys.md).

## Catalog preparation

Extract FDID988200 with `casc-local` from the local WoW install. Store the DB2 under the engine's persistent data tree. Then run:

```text
python3 scripts/export-toys.py <988200.db2> <engine>/data/db2/12.1.0.69933/Toy.csv --exporter <engine>/scripts/export_db2_csv.py
python3 scripts/import-toys.py <Toy.csv> data/world.db --report <joins.json>
```

The wrapper registers WoWDBDefs layout821B5D20 with the engine's maintained WDC5 decoder; unknown encrypted records are dropped by that decoder. The importer atomically replaces only the `toy` reference table. It preserves every decrypted identity and writes unresolved joins to the requested report. Runtime joins Item/ItemSparse and the first on-use ItemXItemEffect→ItemEffect ordered by LegacySlotIndex/ID, matching item-use catalog selection. Missing runtime content_item definitions remain unavailable rather than receiving substitute templates. Startup fails explicitly if the reference table has not been prepared.

## Transmitted toy objects and viewer-local models (2026-10-10)

Effect50 had a ritual-only handler: nonritual toys could report success and spawn nothing. `game_objects::summoned` now loads authentic spell-referenced TDB1210 templates/DB2 display bounds (not the Wrath-derived content templates: Stormwind banner194274 uses Retail display10483, not8623) at world-data startup and handles non-scripted generic decoration, chair and mailbox objects. Other template types/scripts and absent content remain explicit preflight failures. Existing ritual channel ownership is unchanged. Template scale, chair seats, destination/facing and map copy are preserved; positive spell duration expires in whole seconds, while nonpositive duration does not set a despawn timer (TC a352b1fa EffectTransmitted).

Target87 is DEST_DEST, preserving the clicked/selected destination. Toy Box now enters the native cursor and carries that point in UseToy; missing/nonfinite/out-of-range points reject before casting instead of silently using the caster. Target47 still uses source forward radius. [Current transport](#toy-effects14-authentic-input-and-rendering-boundaries-2026-10-11).

Aura233 is TC HandleUnused/client-side-only appearance, not aura56's global transform. Server stores/replicates the authentic aura and removes it normally. Native client viewer-local humanoid presentation still needs implementation; no rendered parity claimed. Source: pinned TC a352b1fa SpellAuraDefines.h320/SpellAuraEffects.cpp305; concrete local DB2 spells289277 and384585. No synthetic IDs or database writes. Required missing object imports and census receipts live in the toy-effects4 handoff.

Measured server-preflight census75→95/1166:21 newly available object toys plus2 aura233 toys, minus3 former false positives (Little Wickerman70722, Steamworks Sausage Grill116757 and Jeweled Offering193032 still require spell-focus/chest behavior). Two previously admitted chair/mailbox toys now actually spawn; Moonkin chair70161 uses its available authentic TDB template.31 toy tests,19 ritual regressions,12 teleport regressions and7 summon regressions pass (some overlap); format/check pass without warnings. Exact receipts: engine `data/diagnostics/toyfx4-2026-10-10/`. No rendered-client parity claimed.

## Audited attachment and sound toys (2026-10-10)

The largest remaining blocker among toys with runtime item definitions was aura4 (53 candidates), but dummy metadata still does not establish a script's action. The visual-only audit now admits eight more authentic items: Spitzy156871, Worn Cloak165791, Filmless Camera221964, Lampyridae Lure228413, Valdekar's Special263198, Lil' Scoots' Pillow267456, Ken-Ken's Mask88580 and Mallard Mortar204818. Each has a nonzero local DifficultyID-0 SpellXSpellVisual binding, zero-valued dummy-only effects and no selected-spell binding in the local TDB1210 spell_script_names table. Local descriptions promise attachments or a targeted squeak, not damage, teleport, discovery or creature spawning. The existing complete-effect audit remains unchanged; no blanket aura4/effect3 support was added.

Actual bag learning and UseToy prove caster/target SpellGo, aura replication/cancellation and no global transform or damage. Worn Cloak preserves its Horde-only race restriction (Orc use/Human refusal). Ken-Ken's Mask lasts60000ms on the selected friendly player, not the caster. Mallard Mortar has a2000ms cast and a minimum range; the valid20yd target receives the spell without damage. Valdekar's Special3300ms and Lil' Scoots' Pillow11000ms expire normally. Spitzy and Scoots retain their authentic conditional visual bindings; client condition evaluation/asset presentation are not proven by server tests.

Black Ice133997/203533 and Eternal Black Diamond Ring134007/203820 have no local SpellXSpellVisual binding and remain blocked despite attachment descriptions. Negative actual-use tests retain refusal without SpellGo/cooldown. No database import is needed for the eight admitted items; unknown scripts, zero-entry costumes, other aura types/object interactions and viewer-local233 remain outside this audit.

Census95→103/1166 on unchanged reference data. Proof: four new behavioral tests pass; one broad toy run passed34/35, then correcting the Human-only fixture for Worn Cloak made all four new tests pass (35 distinct toy tests covered across receipts). Required `cargo test -p server --bin game-server spell_cast` passed193. Evidence: engine `data/diagnostics/toyfx5-2026-10-10/{before,after,source-records}.json` and final logs. This is server-preflight/CPU proof, not rendered Retail-client parity. [Contract](../../specs/toys.md).

## Toy-effects6 follow-up (2026-10-10)

Duck-Stuffed Duck Lovie194056/383065 is the additional aura4 admission: local description promises a nap, complete effects are zero-valued dummy-only, visual118994 supplies kit162763's duck attachment/AnimKit3404 and kit167706's cast sound. Ordinary bag learning, UseToy, aura replication and cancellation apply without invented healing/transform/gameplay sleep. Behavioral RED was an actual aura4 refusal; GREEN proves learning/use/SpellGo/aura/cancel and unchanged health. No other aura4/effect3 gameplay scripts were inferred from zero-valued metadata.

After merging origin/master2ab5e5e into the feature branch, spell_cast261/261 and toy36/36 pass. Census103→104/1166, only gain194056, no removals. Receipts: engine `data/diagnostics/toy-effects6-2026-10-10/{before,after,delta}.json`, `final-spell-cast.log`, `final-toys.log`. These are server-preflight candidates, not complete Retail visual parity.

[Client follow-up](../../../../game-engine/docs/wiki/systems/toy-box.md#toy-effects6-conditional-selection-and-summon-coverage-2026-10-10) implements Spitzy's single inverted aura condition and imports authentic ordinary train-wrecker appearance coverage. Both configured skins produced native Spitzy/train/Duck Lovie assets through actual Toy Box input. Valid-range mortar remains CPU-proven but live-unproved: final Modern reached19.82yd with hostile faction14 Invader42937 but selection was lost (`NoTarget`); final Forever's selected model left during post-selection repositioning. The final fixture removes that repositioning and learns before selection; this correction is not itself rendered acceptance.

Haunting Memento170950 has DB2period180000 but trigger0; no tick action can be derived. Scoots needs absent ModifierTree303980 and cloak Type17 export lacks procedural payload. Largest remaining groups are aura4=285, summon28=263 and effect3=157; custom scripts/control categories and missing content are not closed by this one admission. Aura233 stays skipped. No world.db mutation, shared-realm deployment or master merge.

## Toy-effects7 visual-only audit (2026-10-10)

The rerun starts at104/1166. Shared blocker incidence remains aura4=285, summon28=263, effect3=157, effect77=61 and effect50=52; those counts overlap. Restricting to actual runtime-item eligible entries gives summon28=46, aura4=45, effect3=31, effect77=18 and effect50=12. Guardian actors do not supply missing creature templates, puppet/vehicle control, click interactions or scripted event actions. For example, importing the wormhole creature alone would not implement its travel menu, and spawning a cushion would not implement sitting on it. Those groups remain blocked rather than receiving invented actions or broad imports.

Three complete zero-valued dummy-aura spells have source-backed visual-only actions: Akazamzarak's Spare Hat147838/239560, Barrier Generator153183/254513 and A Drake's Big Basket of Eggs204675/405634. The existing complete-effect audit remains strict; no generic aura4 admission. Local Retail12.1.0.69933 SpellXSpellVisual→SpellVisualEvent→SpellVisualKitEffect→ModelAttach rows establish hat/rabbit models876205/1244081 with AnimKit9950, barrier model1607602 with hand effects166009, and the basket's unconditional and six caster-conditioned visual choices. No selected-spell binding occurs in the local TDB1210 spell_script_names dump; that absence is not itself proof of cosmetics.

Hat casting takes2000ms and its aura lasts5000ms; no NPC rabbit is synthesized. Barrier is a10000ms cosmetic channel, not an absorb or collidable object. Basket retains the authentic405634 aura and all visual conditions, not a forced player/mount model change. Client mount-condition selection, rendered attachments and sounds are not certified by server admission. All three runtime item definitions and required spell rows already exist; no importer or database write is needed.

Census104→107/1166 (+3, no removals) on unchanged reference data. Behavioral RED3 refusals became GREEN3; post-origin/master toy39/39, format/check pass without warnings. The required one-shot spell_cast gate passed314/315: unchanged guardian sacrifice fixture `guardians_sacrifice_splits_spell_damage_and_stops_on_native_health_floor` failed its random-damage transfer expectation at guardian_tests.rs113. That unrelated failure is retained, not hidden by a rerun or changed toy semantics. Source receipt: canonical engine `data/diagnostics/toy-effects7-2026-10-10/selected-source.json`; full baseline census and ranked candidates are beside it. Haunt tick action, cloak Type17 payload, Scoots ModifierTree303980 and aura233 remain excluded. [Contract](../../specs/toys.md).

## Toy-effects8 stationary scenery preparation (2026-10-10)

Runtime-eligible effect28 is the largest remaining group (46 blocked toys, versus42 aura4). Two entries require only authentic stationary scenery: Silversage Incense88381/127771→65298 and Totem of Harmony88584/128180→65387. Local Retail DB2 gives ally control1/title0/slot0, destination87 and20000/60000ms durations. TDB1210 supplies incense displays41007/41008 and totem41838, with no npcflags, AIName, ScriptName, addon or selected smart-script/spell-script binding. Source model identities are preserved; no client light/burning effect or rendered parity is certified by CPU tests.

[Importer](../../../scripts/add_toy_summon_templates.py) now prepares these two alongside the existing linked summons, using the same TDB conversion as a full content rebuild. Run `python3 scripts/add_toy_summon_templates.py --db <disposable-or-approved-db-copy>`; it imports source models/addons and fails explicitly when the required scenery templates/models are missing. Canonical world.db is not modified by this work. Availability gains depend on applying this data preparation; an unchanged shared database stays at107 candidates. Tests learn/use both toys, check source display membership, ownership/map/faction, stationary position and duration-based despawn on a disposable imported copy. A separate imported-copy census retains scripted seating, drinks, wormholes and fireworks as unavailable.

Sources: local12.1.0.69933 Spell/SpellEffect/SpellMisc/SpellDuration/SummonProperties and world.db's authentic TDB1210 creature template/model/difficulty tables; archive `data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql` spell_script_names. [Contract](../../specs/toys.md). Durable source/proof receipts: engine `data/diagnostics/toy-effects8-2026-10-10/`. Measured107→109/1166 on the imported copy (+88381,+88584,zero removals); unmodified shared database remains107. One post-origin/master gate passed325 spell_cast and41 toy tests; importer2/2 passes source-row preservation, repeat CLI idempotency and rollback on missing authentic models. Format/check pass without warnings. Selected template/model/difficulty/addon/smart-script rows match the actual local archive value-for-value; shared DB SHA256 is unchanged. No native/rendered verification or deployment performed.

## Account state and transport

`account_toys` stores a bitcode map `item_id -> favourite` keyed by account. Learning on normal UseItem writes both ownership and the consumed inventory record in one redb transaction. A duplicate returns an error without consumption. Favourite updates reread the durable account row inside their write transaction.

ToyCollectionUpdate on CollectionChannel carries the full catalog and learned/favourite flags. Initial world entry and each learning/favourite change publish it; updates broadcast to every in-world connection authenticated to that account. Other accounts remain independent. SetToyFavourite and UseToy are client-to-server requests on the same channel. ToyResult identifies Learn/Favourite/Use and carries an optional error; learning also retains the normal InventoryError refusal path.

## Activation boundary

UseToy checks ownership, raw metadata, runtime item eligibility and complete reachable spell preflight before dispatch. Registry-unhandled effects/auras and script-only auras fail explicitly. Aura56 requires a real creature template or an implemented source-defined zero-entry mapping; the three costume mappings above are supported, other special cases remain blocked. Summon28 requires available properties, supported control and a creature template. Area-trigger179 without a trigger binding/radius is lifecycle-only and unavailable as a toy action. Unsupported implicit targets and trigger depth/cycles are reported too.

### Data-backed toy effects (2026-10-10)

The original 300 summon blockers were primarily reference-data absence, not a stale effect registry: effect28 was already registered. Import authentic local `SummonProperties.csv` through `scripts/db2.py`; a private test database with the table raised the unchanged classifier from38 to43 candidates. Creature templates are still required; do not invent newer Retail creatures from spell names. Preflight now reports property/creature IDs and unsupported control/title pairs separately from effect28.

Wild control0/title0 summons keep their template faction. Ally/guardian/companion summons inherit their caster's faction. Title5 companions join FollowOwner and are immune to PCs/NPCs; positive slots replace previous occupants. Control3 puppets, control4/5 vehicles and specialized titles are not generic temporary creatures. Green Balloon69895 references missing creature53453: the companion test uses existing template25305 as an explicitly labelled mechanism fixture, not a replacement catalog definition.

Visual-only support uses the audited IDs in [cosmetic.rs](../../../crates/server/src/spell_cast/cosmetic.rs), plus runtime checks: a nonzero DifficultyID-0 SpellXSpellVisual and exclusively zero-valued effect3/effect77/aura4, no misc payload, trigger, item or mechanic. Descriptions promise only attachments/sounds/visuals: Touch of the Naaru119144, Fruit Basket119083, Seafarer's Slidewhistle126931 and Heart of a Champion169768 have concrete loopback use tests. SpellGo already broadcasts caster, spell and hit targets; aura4 uses existing ActiveAuras/UnitAuras apply, duration and cancellation. No new spell-specific gameplay script is installed. Visual presence cannot prove that periodic dummy ticks do nothing; the data-backed226 bridge and retained script gaps are described below.

Counterexamples to blanket acceptance: Gadgetzan18986/23453 teleports and scouting-map150743/360598 unlocks flight points despite single zero-valued dummy effects plus visuals. Both stay unavailable. Metadata-drift tests remove a visual binding or append another effect to audited176594 and assert refusal without cooldown/SpellGo.

[Forlorn/charge master integration](../../toy-effects14-coverage.md#forlorncharge-master-integration-2026-10-17) retains both delayed-hit behaviors; merged55-case proof and template publication receipt are separate from historical whole-Toy/native claims.

[Source boundaries89–93](../../toy-effects14-coverage.md#source-scriptcompanionheadwear-boundaries8993-2026-10-17) retain Poking's dummy child, owned-critter150 and conditional procedural headwear gaps; [94–98 source actions](../../toy-effects14-coverage.md#source-actionprocedural-boundaries9498-2026-10-17) retain nearby reactions, spectral/headwear operations, corpse ritual and return/catch gaps. No generic cosmetic admission. [99–103 matrix](../../toy-effects14-coverage.md#source99103-existing-contracts-versus-missing-actions-2026-10-17) separates retained Cooler/Gong contracts from missing procedural/Kovork identities.

[Flashback88 source screen-effect boundary](../../toy-effects14-coverage.md#flashback88-screen-effect-boundary-2026-10-17) separates source metadata, source-area acceptance and missing native presentation; no world-phase conclusion.

Retail type meanings (TC a352b1fa SpellAuraDefines/SpellAuraEffects):293 override abilities,321 no actions,428 linked summons,37 effect immunity,105 feather fall,258 override summoned object,260 screen effect,61 model scale,56 transform. They are not interchangeable cosmetic auras; puppet/vehicle controls remain blocked; generic61/105/37/293 and data-qualified linked summons are described below. Aura56 with an existing template was already handled; zero-entry scripted costumes and missing templates stay blocked. Time-Lost Figurine32782 now explicitly proves ModelDisplay changes to a creature23423 display and cancellation removes it to restore the native race/sex model. Aura260 is implemented as ClientVisual: TC's registry labels it `HandleNoImmediateEffect` with no server code required. Nonzero ScreenEffect IDs use standard aura replication/cancellation. Shadescale170187/304370 proves authentic screen1720 state, but the item stays unavailable because content_item is absent; no runtime item is synthesized.

References: TrinityCore a352b1fa `Spell::EffectSummonType` companion immunity/category dispatch; `AuraEffect::HandleAuraDummy` and `Spell::EffectScriptEffect` retain script hooks. Local Retail12.1.0.69933 Spell descriptions, SpellEffect and SpellXSpellVisual provide the per-ID audit, preserved in the toyfx evidence directory.

Supported casts use the existing start_cast pipeline. A per-cast spell clone applies ItemEffect recovery/category overrides; negative values retain SpellCooldowns. SpellCooldownUpdate reports the actual category and durations, including item recovery. ToyResult success means **cast started**, not completed; SpellGo/CastFailed remain the later outcome. Existing character SpellHistory lifetime is unchanged; server-restart cooldown persistence is not added.

### Generic scale, fall, effect immunity and override sets (2026-10-10)

Aura61 sums per-stack percentage points into replicated UnitScale (player minimum0.1, NPC minimum0.01); unit visuals restore native scale on removal. Aura105 replicates FeatherFall, caps downward prediction/server gravity at7yd/s, caps client-reported descent per server tick, and negates landing damage. Removing the last aura restarts fall accounting at current height. TC a352b1fa Unit::RecalculateObjectScale, HandleAuraFeatherFall and MovementUtil.cpp:30 ground these rules.

Aura37 uses MiscValue as a SpellEffects ID, checked during target filtering, per-target effect dispatch and direct aura application, including friendly summon offers. Spell/effect NoImmunity bypass it (TC Unit.cpp:7934, SpellInfo.cpp:3555/3804, DBCEnums.h:2418). Existing UnitAuras and Immune combat logs provide client presentation.

Aura293 reads ordered10-slot OverrideSpellData (local FDID1002186/layout297CBDC8); existing AuraView.overrides carries SpellSet alongside aura332 pairs. Active set spells receive temporary cast permission without modifying KnownSpells or action persistence. Retail OverrideActionBar.lua:42/Setup displays six slots; the native main bar uses that order, preserving zero/empty slots. Dedicated override-bar chrome and the abilities' individual scripts are separate from this generic source mechanism.

Prepare OverrideSpellData with the existing scripts/db2.py local extract/import workflow, now aware of its inline10-element array. The private census database contains3050 authentic records, none encrypted; canonical world.db was not modified. On missing records/spells the toy remains unavailable, not a successful empty override. Missing content_item still blocks Whispers of Rai'Vosh113542: its authentic167273 proves the fall mechanism, not UseToy. Puppet46709 similarly isolates authentic aura65451/37 for a real meeting-stone85 rejection/removal test; its unsupported puppet behavior remains blocked; aura321 itself is now handled below.

Prior census:71/1166 preflight candidates versus historical58. This branch adds3 (18660,90427,119211);10 other gains came from already-landed master teleport/animation support. Supported reachable aura-toy sets:61=15,105=19,37=20,293=30; sets overlap and are not total usable toys. Source/proof receipts: private toyfx2 diagnostics and /home/osso/.worktrees/handoff-toy-effects2.md. See [client presentation](../../../../game-engine/docs/wiki/systems/toy-box.md).

### Periodic dummy, no-actions and linked summons (2026-10-10)

Census **71→75/1166** with the prepared private database; no previous candidate loses support. New items: Convergent Prism193033 and Gnoll/Market/Expedition Tent193476/194059/194060. This is server-preflight availability, not a complete toy-script or rendered-client claim. Without the linked template import the code yields72 candidates.

Aura226:36 toy roots,8 with explicit DB2 triggers. Seven parasol variants trigger a falling aura that consumes another aura; the falling predicate is absent from DB2, so they remain explicit conditional-script gaps. The remaining data bridge is Gin-Ji Knife Set90175:131568 ticks every330ms and triggers131569 at the channel's original target. Its final corpse-chopping dummy131571 and destination-random72 still block the actual toy. Other28 roots have no trigger and require scripts; no periodic dummy is accepted as a cosmetic no-op. TC a352b1fa PeriodicTick leaves226 to scripts; the explicit trigger bridge is derived from these Retail DB2 records, not a claim that TC automatically executes226 triggers.

Aura321 uses SpellCategories.PreventionType bit4 (NO_ACTIONS), **not aura MiscValue**: real toy auras contain0 or85. Application interrupts a matching current cast; eligible categories remain cast-blocked until the last321 leaves. Existing CastBlock dispatch and textual failure detail carry the rejection without a protocol ABI change. TC a352b1fa HandleAuraModNoActions and SharedDefines.h ground the4 bit. All29 type blockers are removed; other toy effects/puppets/items may still block use.

Aura428 applies its trigger from the aura target once, never again on refresh. Removal despawns one owned matching creature per SUMMON effect, preserving other owners. SummonProperties/data gates remain shared with ordinary summons; four vehicle-controlled toys remain blocked. Tents require target pair18/79: the caster destination followed by a backward offset using the authentic DB2 radius7=2yd (TC SelectImplicitDestDestTargets). Other destination-back forms remain unsupported.20/24 linked-aura type blockers are removed; underlying scripts/targets/items still qualify each toy.

Preparation: all24 linked creatures were absent from content_creature_template but existed in local TDB data with models/difficulty rows. `retail_content.TOY_LINKED_SUMMON_ENTRIES` retains them during normal content preparation. For an existing database, run `python3 scripts/add_toy_summon_templates.py --db <private-copy>` before swapping the prepared database through the normal deployment workflow. The additive importer inserted24 templates and36 model rows, exactly matching the existing Retail conversion; no synthetic templates, item definitions or values. Canonical world.db was not changed.

Aura258 remains explicitly unavailable for all17 toys. Its bindings replace gameobject35591 with DB2 MiscValue1's bobber entry, not a creature summon (TC EffectTransmitted). All33 replacement identities are absent from runtime content_gameobject_template; only18 have TDB source rows. Transmitted-gameobject/fishing support is absent, so no data-complete case exists in this dataset. Classifier reasons name the original/replacement entries; durable JSON lists every item/binding and source availability.

Proof/data receipts: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/toyfx3-2026-10-10/`; [linked lifecycle](../../../crates/server/src/auras/linked_summon.rs), [importer](../../../scripts/add_toy_summon_templates.py). Concrete loopback tests cover193033 use/cancel; isolated46709/37710 no-actions masks, interruption, overlapping/removal;193476 real cast/model/2yd placement, rotation, refresh, owner isolation and cancellation/death cleanup. The226 fixture preserves90175's real trigger/period but gives child131569 Fireball133's real damage effect **only in the fixture**, because its unimplemented terminal dummy has no observable outcome and triggered casts emit no SpellGo. It proves tick scheduling/targeting/removal, not Gin-Ji toy completion.142528 proves explicit bobber refusal. No client changes, merge, live realm or deployment claim.

### Toy-effects11 summon classification and scenery (2026-10-10)

[All43 runtime summon blockers](../../toy-effects11-coverage.md) are classified against read-only world.db and the actual local TDB1210 archive:32 missing templates,5 puppet-control3,3 vehicle-control4,3 specialized-title55. Every source display/model metadata join resolves; missing model-file residency is not inferred from metadata. Other effects/targets and interaction/visibility/scale contracts remain explicit. Existing guardians do not close Mylune's dummy/target72 chain. No new SummonProperties type is admitted without its required runtime.

Prepared census128→130/1166 adds Pilfered Sweeper147867 and Brewfest Chowdown Trophy169865, zero removals. Existing generic importer/summon runtime preserves authentic121762/76543 and183727/92396, wild factions35/0, source forward radii1/1.5yd, stationarity and300000ms expiry. Trophy triggers cosmetic caster aura302730: visual88525→kit115127→animation367705/loop82, not a stat buff or guardian. Exact source matrix, preparation command, per-toy retained contracts and proof boundaries live in the linked coverage page. Canonical DB unchanged by this task; parent owns preparation/promotion and native rendering proof.

## Toy-effects14 authentic input and rendering boundaries (2026-10-11)

The [coverage ledger](../../toy-effects14-coverage.md) distinguishes prepared320/1166 from complete effect certification. Source item selection and three TC costume mappings were followed by ordinary caster/area/enemy kits and static actors; unknown scripts, procedure/condition payloads, controls and interactions remain explicit gaps.

Native target87 placement now uses a real clicked point, with source range validation, Escape/right-click cancellation and no cast-at-feet replacement. Source-targeted toys carry the existing player sight witness in UseToy: the real Northshire bake refuses missing/forged rays and accepts an actual Gaze122120 witness. Snapshot metadata prevents caster/ground-only toys from depending on unrelated target eye data. Focused Toy Box typing must not execute AssistTarget/F; the native process regression reproduces and fixes the selected enemy becoming the player.

Stationary ownership no longer takes the combat-pet frame. Source display multipliers .35/.75/.5/1.5 survive summon creation and aura object-scale application/removal; native ModelData/CDI scale remains separate. Master NativeObjectScale for ordinary NPCs and the tick catalog-borrow fix are retained. All four scaled identities have both-skin actual journal/native mesh+scale proof. Sealamp and Spire additionally pass authored CDI state-kit start/scale/ground-origin and natural180-second actor/effect expiry in both skins; Banner/Prey native expiry and display-replacement process proof remain separate boundaries. [Coverage](../../toy-effects14-coverage.md#batch9-source-display-scale-for-stationary-scenery) links exact artifacts.

Both-skin private5911 PNG/JSON proofs cover costumes, wings, friendly-area powder, clicked Scarecrow and witnessed enemy Gaze. Ocean uses its authored precast/cast/impact sounds: isolated 3D source-stream mixes are nonzero in both skins; physical speaker output is not certified. Authentic BC5 texture2022306 exposed GPU dispatch bypassing the existing decoder; real-ID RED→GREEN fixes upload without replacement bytes. New artifacts live outside canonical checkout at `/home/osso/.worktrees/logs/toy-effects14-continuation/`; requested1280×720 PNGs are in `/syncthing/AgentShared/2026-10-11/toy-effects14/`. Private runtime only; no shared realm promotion.

## Proof and coverage

Pinned local export:1166 decrypted toys. Concrete behavioural fixtures cover Orb of Deception1973 learning/consumption; Time-Lost Figurine32782 spell41301, durable account favourite, second-character relog/reopen, same-account broadcast and cooldown rejection; Eagger Basket188698 spell61810 uses ItemEffect category1203/10s instead of spell category44, rejects repeat and works after expiry; Flag of Ownership38578 rejects unsupported effect3 without starting cooldown. Census output records every root effect/aura and unavailable type including triggered spell chains. These are CPU/loopback proofs, not rendered client or live Retail parity.

## Sources

- [Toy contract](../../specs/toys.md) — requirements and Retail/TrinityCore references.
- [Catalog](../../../crates/server/src/toy_catalog.rs) — joins and unsupported-type preflight.
- [Requests](../../../crates/server/src/toys.rs) — account replication and activation.
- [Persistence](../../../crates/server/src/persistence_toys.rs) — atomic learning.
- [Behavioural fixtures](../../../crates/server/src/toys_tests.rs) — redb/loopback/cooldown proofs.

## See Also

- [[persistence]] — account tables and frozen character ABI.
- [[inventory]] — physical UseItem and inventory delta publication.
- [[spell-casting]] — cast history, effects and cooldown transport.
- [[auras]] — transform and script-only aura limitations.

## Blazing Diamond Pendant (2026-10-13)

Exact171352 admission uses source zero-payload shape, ordinary ground model937030 and existing area group4031 lifecycle; not generic dummy-aura support. Source/server/native evidence and wall-time limits: [Pendant coverage](../../toy-effects14-coverage.md#blazing-diamond-pendant--bounded-lifecycle-proof-2026-10-13). Dedicated expiry fixture keeps position unchanged and waits for natural aura/kit removal while recovery continues.

## Fountain of Knowledge (2026-10-13)

Exact91743 source cosmetic admission renders ordinary ground253570 with scale.5 for the source10s aura; no creature or gameplay effect. Natural removal preserves the15-minute item recovery. [Source, private asset provenance and bounded native proof](../../toy-effects14-coverage.md#fountain-of-knowledge--bounded-lifecycle-proof-2026-10-13).

## Accursed Tome of the Sargerei (2026-10-13)

Exact187146 source cosmetic admission uses ordinary1097945 at attachment17 and offsets(-.3,0,.3), scale1. Source aura lasts300s; natural removal retains one-hour item recovery without changing body/health/position. [Sources, native scopes and retained learning-update gap](../../toy-effects14-coverage.md#accursed-tome--bounded-lifecycle-proof-2026-10-13).

## Source group-private runtime (2026-10-13)

Pinned TC `SpellEffects.cpp`1890–1904 captures the private group GUID, and `Object.cpp`827–845 tests the viewer's current membership. Server stores the captured ID in `PrivateObjectGroup`, separately from caster-only `PrivateObjectOwner`; private descendants inherit either identity. The existing interest index caches group owners and observer membership, invalidating only observers whose group changes. Ordinary map, radius, stealth and death filters still apply. `retail_content.TOY_SCENERY_SUMMON_ENTRIES` imports the twelve direct source contracts' authentic TDB templates/models; no synthetic click/script payload is added. Seventeen census graphs contain this flag; source-only admission is not complete-effect certification. [Contract](../../specs/toys.md#group-private-summons).

### Signed summon title source boundary (2026-10-13)

The current canonical import publishes SummonProperties3001.Title=-1. The old u32 loader aborted the entire store at that row, preventing unrelated source toys from casting. Pinned TC `DB2Structure.h`4318–4325 declares Title as int32; `SummonProperties.title` and title constants now preserve that signed representation. Unknown titles remain explicitly unsupported; the loader neither wraps the sentinel nor substitutes old data. Concrete3001/4493 loader regression accompanies the actual ten toy loader failures.

## Wild variable-count summons (2026-10-14)

Wild/default E28 rolls source CalcValue variance before int32 truncation and minimum1, only for TC's explicit MiscValueB list. The first actor uses the selected destination; secondary actors use TargetA's primary disk radius and native caster orientation, not TargetB's destination radius. Valid map heights adjust secondary elevation; invalid map heights preserve the source elevation, matching `WorldObject::GetRandomPoint`. Count resolution does not add SP/AP damage coefficients. Other effect handlers retain their existing unrandomized base-point helper.

Flute98552 is the concrete zero-primary-radius case; native6122 proves nonzero primary scatter. Guardian/ally count and Scarab2301 are not implemented by this wild-only slice. [Source, behavioral evidence and pending native scope](../../toy-effects14-coverage.md#xantishs-flute--wild-variable-count-slice-2026-10-14); [contract](../../specs/toys.md).

### Source forward destination

TC a352b1fa `SpellInfo.cpp` maps78 to DEST_DEST_FRONT; `Spell.cpp::SelectImplicitDestDestTargets` moves the prior destination by this slot's CalcRadius and direction. The bounded18/78 route shares destination-offset calculation with existing18/79 backward placement. Caster-based destinations retain caster facing even when an unrelated unit is selected; destination-relative offsets retain that prior destination's facing rather than replacing it with the selected unit's orientation. Spirit of Bashiok173895 authors18/78, primary radius0 and secondary radius8=5yd. AttributesEx4=0x00800000 does not set USE_FACING_FROM_SPELL0x80000000, so its authored facing pi is ignored. No destination fallback, other directional selector, or terrain/collision implementation is added. Authentic86916/display60051 and60s actor lifecycle remain separate from destination selection; see [contract](../../specs/toys.md).

### Initial random destination radius (2026-10-14)

Pinned TC `SpellInfo.cpp`822–825 randomizes `CalcRadius.Max` for targets72/74/86 as `(Max-Min)*sqrt(rand_norm())`; `Spell.cpp` destination callers only consume that result. The implemented72/86 selectors previously randomized direction alone, placing Flute and class summons on rings. They now sample radius too, preserving target91's fixed-distance/random-direction contract. When TargetB lacks its own radius entry, `CalcRadius` inherits both TargetA's entry and target identity; inheriting only the numbers would spuriously randomize an18/86 combination. This is not secondary `GetRandomPoint` scatter: that separate function adds Min, while `CalcRadius` does not. Concrete native Flute142917 and class1276452/123904 destination regressions cover disk interior/area, nonzero-Min span, and target91 isolation. Unsupported74 remains unsupported; broader radius modifiers are not added here. Direct91 without a preceding destination (Fisherman's Folly388259) must run native `CheckDst` before selecting its fixed2yd ring. Destination-relative facing must not return early before that initialization; the generic summon-at-caster path is not a substitute for applying the authored offset.

### Spawn-group guardian count boundary (2026-10-14)

Scarab Storm64881/92146 uses property2301, Control1/Title0/flags0x400202. TC `SpellEffects.cpp`1944–1951 checks JoinSummonerSpawnGroup0x200 before the title switch; `Object.cpp`1191–1231 gives the default title a Guardian actor, and `CreatureAI.cpp`163–205 permits following. The server now preserves that supported default-title route instead of treating it as an ordinary one-actor ally. Source count is rolled once from the existing explicit property list; each actor independently selects its authentic weighted model. `SummonGuardian`5015–5052 places the first at destTarget and additional actors in a5yd disk. Existing follow-owner designation, map/owner identity, private inheritance and non-pet designation remain separate fields. Group28 properties3535/4799 carry the same flag: their five direct source cases also require following; the native Doll/Gong witnesses use other properties and remain separately scoped. No creature spell is invented when local bindings are absent.

`Unit::SetMinion`6264–6290 adds PLAYER_CONTROLLED only when the minion's owner is a player. The server previously treated all ally-category actors as controlled and skipped wild-category flagged guardians; a later companion immunity insert also replaced the ownership bit. A source-role predicate and one pure flag composition now preserve ownership plus existing immunity bits. Concrete actor casts cover the five flagged Group28 cases, NPC-owned guardians, plain allies and both player/NPC companions; IsPet remains a separate component.

The additive importer preserves native51113 and both36611/36595 choices with weights1/2. This bounded source route/count/lifetime slice does not certify native visuals, complete Scarab behavior, default ally multiplicity, or all pet/specialized-title count paths. `TempSummon::InitStats` clamps source levels before guardian initialization; the pre-existing generic guardian level path is not certified by this count slice, and unconditional owner-level assertions were retired rather than presented as a native contract. Earlier wild-only evidence remains scoped to that slice; see [contract](../../specs/toys.md).

### Source guardian level selection (2026-10-15)

Guardian51 reproduces owner80 Storm Scarabs at80 instead of Northshire's30 through real bag learning/UseToy; owner20 is the positive control. Source `Creature::ApplyLevelScaling` selects difficulty-template ContentTuning first, then map difficulty, then actual destination-area ancestry; critters skip regional tuning. Source difficulty deltas remain effective even when the template's own ContentTuning is zero. `TempSummon::InitStats` clamps the summoner's level before `Guardian::InitStats`; flag0x100 UseCreatureLevel instead retains the native maximum plus delta. NorthshireArea9 inherits ContentTuning73, range1..30, so owner20→20 and owner80→30; the test-only flag control yields30 for owner20.

[Read-only projection](../../../crates/server/src/world_data/summon_levels.rs) preserves the existing SQL tuning ranges, map/area IDs and raw template/difficulty deltas without changing ordinary NPC spawning. [Summon creation](../../../crates/server/src/spell_cast/summon.rs) passes the selected level into both health calculation and UnitLevel before guardian initialization. Missing actual terrain or referenced tuning stays explicit, not a guessed legacy template range. A known zero-tuning ancestry is different: pinned UF::UpdateFieldBase initializes the scaling bounds to zero, and missing ContentTuning does not replace them; native SelectLevel first clamps maximum0 to1, while ordinary TempSummon subsequently clamps owner level to0..0. No fabricated1..max range is introduced. WMO-only actual-area identification remains unavailable.

At0e2c24e8, thirteen final focused controls pass: real UseToy levels/flag, source priority/parent ancestry/difficulty deltas, signed and reversed delta bounds, unavailable-area refusal, retained count/follow/expiry, native random class summons, Rune Weapon and startup fixtures. Legacy synthetic-origin summon fixtures now use authored Northshire terrain rather than a production fallback. Merged check/format/build gates and actual private UDP bag-learning/UseToy replication pass: fresh owners20/80 receive29/26 own Scarabs, all levels20/30 respectively, with positive health. [Exact wire scope and remaining boundaries](../../toy-effects14-coverage.md#guardian-source-level-wire-proof-2026-10-15) preserve binary hashes and distinguish level-only evidence from complete guardian ExpectedStat/health/AI parity. [Contract](../../specs/toys.md).

### Flute follow source adjudication (2026-10-15)

Active70391 `SpellEffect`191120 routes142917 to71500/property64. That `SummonProperties` row is Control0/Title0/Slot0/Flags0. Pinned TC a352b1fa `SpellEffects.cpp`1943–2036 therefore uses wild/default `TempSummon`, not Minion/Guardian; `CreatureAI.cpp`163–205 `ShouldFollowOnSpawn` returnsfalse. The lead's source-parity decision preserves non-follow behavior rather than inventing a script/distance from descriptive text. This proves the TC route, not independent Retail client behavior. [Contract](../../specs/toys.md); [proof boundaries](../../toy-effects14-coverage.md#xantishs-flute--wild-variable-count-slice-2026-10-14). Private evidence: `flute48-source-properties64.json`, `flute48-source-effect.json`. Both-skin quiet expiry remains separately proved; fresh conclusive source-model pixels remain open.
