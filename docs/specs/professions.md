# Professions

Retail professions on the server: learning at profession trainers and crafting known recipes. How it works: [professions](../wiki/systems/professions.md).

## Requirements

- Trainers are TrinityCore TDB1210 trainers (`tdb_*` tables). Opening a trainer sends every service that fits the character's race and class, with its cost, required level/skill/abilities and state (Available, Unavailable, Known).
- A purchase succeeds only for an Available service of the open trainer, with enough copper and a free primary profession slot (2) when it adds a primary profession. It takes the cost and teaches the spell; refusals are `TrainerBuyFailed` (`NotEnoughMoney`, `Unavailable`) and change nothing.
- Learning a profession (264617 Tailoring) learns its base spell, its Classic line at rank 1 with the Retail tier cap (300), and the line's automatic recipes (2963 Bolt of Linen Cloth).
- A known recipe casts with its Retail cast time. Without every reagent, tool or bag room it fails with Retail error text and changes nothing. On completion it consumes the reagents, puts the created item in the bags and may raise the recipe's skill-up line by the TrinityCore difficulty chances.
- `CraftRecipe { casts }` repeats the cast; stopping or moving cancels the rest, and the first cast that cannot start ends the queue.
- Professions persist per character and reach the owner as `ProfessionSnapshot` on world entry and after every change.

## Out of scope

Gathering nodes, Dragonflight+ quality/concentration/crafting orders/specializations, recrafting, unlearning, First Aid, reputation price discounts, gossip-menu-specific trainers.
