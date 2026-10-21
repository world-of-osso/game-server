# Crenna70448 kit census

Read-only local `world.db` spec105/follower209072 snapshot, verified2026-10-21. This inventories learned kit rows, not proof every native mechanic is implemented. Decision policy and named gaps: [healer triage](../systems/follower-healer-triage.md).

Canonical JSON-row SHA256: `3ab2c349305f3590e33d39b028e291f41f37aecee68911723ad858cf67593cdf`.

| Spell | Name | Levels | Slot | Authored conditions |
|---|---|---|---|---|
| 77758 | Thrash | 11–90 | aoe | `{"min_enemies": 3, "source_center": true, "target": "enemy"}` |
| 5176 | Wrath | 1–90 | attack | `{"target": "enemy"}` |
| 8921 | Moonfire | 2–90 | attack | `{"missing_aura": 164812, "target": "enemy"}` |
| 5221 | Shred | 5–90 | attack | `{"target": "enemy"}` |
| 22568 | Ferocious Bite | 7–90 | attack | `{"target": "enemy"}` |
| 93402 | Sunfire | 26–90 | attack | `{"missing_aura": 164815, "target": "enemy"}` |
| 1126 | Mark of the Wild | 9–90 | buff | `{"missing_aura": 1126, "target": "self"}` |
| 22812 | Barkskin | 10–90 | defensive | `{"attacked": true, "health_below": 0.5, "missing_aura": 22812, "target": "self"}` |
| 102342 | Ironbark | 27–90 | defensive | `{"attacked": true, "health_below": 0.5, "missing_aura": 102342, "target": "party"}` |
| 88423 | Nature's Cure | 10–90 | dispel | `{"dispel_types": [1, 2, 4], "target": "party"}` |
| 8936 | Regrowth | 3–90 | heal | `{"health_below": 0.8, "target": "party"}` |
| 18562 | Swiftmend | 17–90 | heal | `{"health_below": 0.4, "target": "party"}` |
| 48438 | Wild Growth | 34–90 | heal | `{"health_below": 0.7, "min_injured": 3, "target": "party"}` |
| 740 | Tranquility | 37–90 | heal | `{"health_below": 0.7, "min_injured": 3, "target": "party"}` |
| 145205 | Efflorescence | 39–48 | heal | `{"health_below": 0.7, "min_injured": 3, "target": "party"}` |
| 774 | Rejuvenation | 10–90 | hot | `{"health_below": 0.9, "missing_aura": 774, "target": "party"}` |
| 33763 | Lifebloom | 21–90 | hot | `{"exclusive_aura": true, "health_below": 0.95, "missing_aura": 33763, "target": "tank"}` |
| 20719 | Feline Grace | 1–90 | passive | `{"target": "self"}` |
| 137009 | Druid | 1–90 | passive | `{"target": "self"}` |
| 137012 | Restoration Druid | 1–90 | passive | `{"target": "self"}` |
| 162701 | Stat Negation Aura - Intellect Healer | 1–90 | passive | `{"target": "self"}` |
| 392378 | Improved Nature's Cure | 1–90 | passive | `{"target": "self"}` |
| 462069 | Druid | 1–90 | passive | `{"target": "self"}` |
| 462073 | Restoration Druid | 1–90 | passive | `{"target": "self"}` |
| 1256995 | Restoration Druid | 1–90 | passive | `{"target": "self"}` |
| 125972 | Feline Grace | 6–90 | passive | `{"target": "self"}` |
| 77495 | Mastery: Harmony | 10–90 | passive | `{"target": "self"}` |
| 197490 | Feral Affinity | 10–90 | passive | `{"target": "self"}` |
| 449182 | Ursoc's Spirit | 10–90 | passive | `{"target": "self"}` |
| 405834 | Improved Prowl | 13–90 | passive | `{"target": "self"}` |
| 301768 | Verdant Heart | 14–90 | passive | `{"target": "self"}` |
| 113043 | Omen of Clarity | 15–90 | passive | `{"target": "self"}` |
| 327993 | Improved Barkskin | 16–90 | passive | `{"target": "self"}` |
| 276012 | Aquatic Form | 17–90 | passive | `{"target": "self"}` |
| 33873 | Nurturing Instinct | 18–90 | passive | `{"target": "self"}` |
| 392410 | Verdant Infusion | 19–90 | passive | `{"target": "self"}` |
| 159456 | Travel Form | 20–90 | passive | `{"target": "self"}` |
| 276029 | Flight Form | 20–90 | passive | `{"target": "self"}` |
| 158478 | Soul of the Forest | 21–90 | passive | `{"target": "self"}` |
| 231032 | Improved Regrowth | 23–90 | passive | `{"target": "self"}` |
| 403521 | Tranquil Mind | 25–90 | passive | `{"target": "self"}` |
| 86093 | Leather Specialization | 27–90 | passive | `{"target": "self"}` |
| 197524 | Astral Influence | 28–90 | passive | `{"target": "self"}` |
| 474678 | Aessina's Renewal | 30–90 | passive | `{"target": "self"}` |
| 155675 | Germination | 33–90 | passive | `{"target": "self"}` |
| 1226140 | Grove Guardians | 35–90 | passive | `{"target": "self"}` |
| 231040 | Lingering Healing | 36–90 | passive | `{"target": "self"}` |
| 392302 | Power of the Archdruid | 37–90 | passive | `{"target": "self"}` |
| 16931 | Thick Hide | 38–90 | passive | `{"target": "self"}` |
| 328025 | Improved Wild Growth | 39–90 | passive | `{"target": "self"}` |
| 470549 | Improved Swiftmend | 41–90 | passive | `{"target": "self"}` |
| 131768 | Feline Swiftness | 42–90 | passive | `{"target": "self"}` |
| 382559 | Unstoppable Growth | 43–90 | passive | `{"target": "self"}` |
| 385786 | Matted Fur | 44–90 | passive | `{"target": "self"}` |
| 400533 | Wild Synthesis | 47–90 | passive | `{"target": "self"}` |
| 377847 | Well-Honed Instincts | 48–90 | passive | `{"target": "self"}` |
| 1217941 | Lifetreading | 49–90 | passive | `{"target": "self"}` |
| 468146 | Reactive Resin | 50–90 | passive | `{"target": "self"}` |
| 470562 | Renewing Surge | 51–90 | passive | `{"target": "self"}` |
| 145108 | Ysera's Gift | 53–90 | passive | `{"target": "self"}` |
| 377796 | Natural Recovery | 54–90 | passive | `{"target": "self"}` |
| 289237 | Master Shapeshifter | 55–90 | passive | `{"target": "self"}` |
| 343238 | Entangling Roots | 56–90 | passive | `{"target": "self"}` |
| 378988 | Lycara's Teachings | 56–90 | passive | `{"target": "self"}` |
| 393371 | Cenarius' Guidance | 59–90 | passive | `{"target": "self"}` |
| 474541 | Circle of the Heavens | 60–90 | passive | `{"target": "self"}` |
| 392256 | Harmonious Blooming | 61–90 | passive | `{"target": "self"}` |
| 1232897 | Lycara's Inspiration | 62–90 | passive | `{"target": "self"}` |
| 1263879 | Nature's Bounty | 63–90 | passive | `{"target": "self"}` |
| 392356 | Reforestation | 65–90 | passive | `{"target": "self"}` |
| 449191 | Oakskin | 66–90 | passive | `{"target": "self"}` |
| 1264649 | Intensity | 67–90 | passive | `{"target": "self"}` |
| 1262034 | Gift of the Wild | 68–90 | passive | `{"target": "self"}` |
| 207383 | Abundance | 69–80 | passive | `{"target": "self"}` |
| 392167 | Everbloom | 81–90 | passive | `{"target": "self"}` |
| 1244331 | Everbloom | 81–90 | passive | `{"target": "self"}` |
| 449185 | Lore of the Grove | 82–90 | passive | `{"target": "self"}` |
| 400129 | Forestwalk | 85–90 | passive | `{"target": "self"}` |
| 207383 | Abundance | 87–90 | passive | `{"target": "self"}` |
| 1244470 | Everbloom | 90–90 | passive | `{"target": "self"}` |
| 5225 | Track Humanoids | 1–90 | utility | `{"target": "enemy"}` |
| 440015 | Remove Corruption | 1–9 | utility | `{"target": "enemy"}` |
| 1229376 | Single-Button Assistant | 1–90 | utility | `{"target": "enemy"}` |
| 339 | Entangling Roots | 4–90 | utility | `{"target": "enemy"}` |
| 768 | Cat Form | 5–90 | utility | `{"target": "enemy"}` |
| 1850 | Dash | 6–90 | utility | `{"target": "enemy"}` |
| 5487 | Bear Form | 8–90 | utility | `{"target": "enemy"}` |
| 33917 | Mangle | 8–90 | utility | `{"target": "enemy"}` |
| 6795 | Growl | 9–90 | utility | `{"target": "enemy"}` |
| 783 | Travel Form | 10–90 | utility | `{"target": "enemy"}` |
| 5215 | Prowl | 13–90 | utility | `{"target": "enemy"}` |
| 50769 | Revive | 13–90 | utility | `{"target": "enemy"}` |
| 132158 | Nature's Swiftness | 13–90 | utility | `{"target": "enemy"}` |
| 20484 | Rebirth | 19–90 | utility | `{"target": "enemy"}` |
| 22842 | Frenzied Regeneration | 21–90 | utility | `{"target": "enemy"}` |
| 18960 | Teleport: Moonglade | 22–90 | utility | `{"target": "enemy"}` |
| 197628 | Starfire | 22–90 | utility | `{"target": "enemy"}` |
| 24858 | Moonkin Form | 24–90 | utility | `{"target": "enemy"}` |
| 132469 | Typhoon | 32–90 | utility | `{"target": "enemy"}` |
| 102359 | Mass Entanglement | 34–90 | utility | `{"target": "enemy"}` |
| 102401 | Wild Charge | 40–90 | utility | `{"target": "enemy"}` |
| 212040 | Revitalize | 47–90 | utility | `{"target": "enemy"}` |
| 33786 | Cyclone | 50–90 | utility | `{"target": "enemy"}` |
| 106898 | Stampeding Roar | 52–90 | utility | `{"target": "enemy"}` |
| 33891 | Incarnation: Tree of Life | 57–90 | utility | `{"target": "enemy"}` |
| 1261867 | Heart of the Wild | 64–90 | utility | `{"target": "enemy"}` |
| 29166 | Innervate | 70–90 | utility | `{"target": "enemy"}` |

## Sources

- `data/world.db`: `follower_spec_kit`, `spell_name`, and `follower_profiles` entry209072 (native70448 server import).

## See Also

- [[follower-healer-triage]]
- [[follower-data]]
