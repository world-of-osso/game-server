-- INFERRED_WCL: apply only after the seasonal map damage scaling is deployed.
-- Provenance, per-creature sample counts, reports, IQR, exclusions, and retirement:
-- content/dungeons/creature-damage-overrides.json
BEGIN IMMEDIATE;
CREATE TEMP TABLE _wcl_damage_guard (ok INTEGER NOT NULL CHECK (ok=1));
INSERT INTO _wcl_damage_guard VALUES (CASE WHEN
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=187894 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=187897 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=187969 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=188011 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=188067 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=188244 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=188252 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=189232 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=190034 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=190205 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=190206 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=190207 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=197535 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=197697 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=197982 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=198047 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=241808 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=241809 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=241812 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=241813 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=241814 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=241816 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=244100 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=245139 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=245143 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=245146 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=245190 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=245855 AND DifficultyID=8 AND DamageModifier=1.0) AND
    EXISTS (SELECT 1 FROM content_creature_template_difficulty WHERE entry=246404 AND DifficultyID=8 AND DamageModifier=1.0)
    THEN 1 ELSE 0 END);
-- Infused Whelp (187894): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=63; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=1.179642 WHERE entry=187894 AND DifficultyID=8 AND DamageModifier=1.0;
-- Defier Draghar (187897): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=34; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=4.376814 WHERE entry=187897 AND DifficultyID=8 AND DamageModifier=1.0;
-- Deepstone Earthshaper (187969): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=168; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=2.043344 WHERE entry=187969 AND DifficultyID=8 AND DamageModifier=1.0;
-- Earthbound Guardian (188011): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=122; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=2.696413 WHERE entry=188011 AND DifficultyID=8 AND DamageModifier=1.0;
-- Flashfrost Chillweaver (188067): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=17; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=3.162888 WHERE entry=188067 AND DifficultyID=8 AND DamageModifier=1.0;
-- Primal Juggernaut (188244): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=43; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=4.05025 WHERE entry=188244 AND DifficultyID=8 AND DamageModifier=1.0;
-- Melidrussa Chillworn (188252): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=35; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=4.70336 WHERE entry=188252 AND DifficultyID=8 AND DamageModifier=1.0;
-- Kokia Blazehoof (189232): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=51; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=4.736145 WHERE entry=189232 AND DifficultyID=8 AND DamageModifier=1.0;
-- Blazebound Destroyer (190034): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=94; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=3.254308 WHERE entry=190034 AND DifficultyID=8 AND DamageModifier=1.0;
-- Scorchling (190205): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=285; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=0.797821 WHERE entry=190205 AND DifficultyID=8 AND DamageModifier=1.0;
-- Ashseer Flamelasher (190206): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=235; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=1.751794 WHERE entry=190206 AND DifficultyID=8 AND DamageModifier=1.0;
-- Primalist Cinderweaver (190207): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=49; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=2.063589 WHERE entry=190207 AND DifficultyID=8 AND DamageModifier=1.0;
-- High Channeler Ryvati (197535): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=44; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=2.548133 WHERE entry=197535 AND DifficultyID=8 AND DamageModifier=1.0;
-- Flamegullet (197697): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=21; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=3.199698 WHERE entry=197697 AND DifficultyID=8 AND DamageModifier=1.0;
-- Storm Warrior (197982): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=410; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=1.834757 WHERE entry=197982 AND DifficultyID=8 AND DamageModifier=1.0;
-- Tempest Channeler (198047): DJY2fXRrZTALabPv/fight 38 (+3), MyKrABYWmxG9HhtR/fight 1 (+3), wKpcfaj4MzDH9v8X/fight 1 (+2); n=33; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=3.380278 WHERE entry=198047 AND DifficultyID=8 AND DamageModifier=1.0;
-- Territorial Matriarch (241808): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=81; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=3.652402 WHERE entry=241808 AND DifficultyID=8 AND DamageModifier=1.0;
-- Curious Yearling (241809): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=200; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=1.110747 WHERE entry=241809 AND DifficultyID=8 AND DamageModifier=1.0;
-- The Hoardmonger (241812): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=61; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=2.648116 WHERE entry=241812 AND DifficultyID=8 AND DamageModifier=1.0;
-- Thornclaw Gatherer (241813): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=407; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=1.954043 WHERE entry=241813 AND DifficultyID=8 AND DamageModifier=1.0;
-- Earthwhisper Tender (241814): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=55; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=2.102895 WHERE entry=241814 AND DifficultyID=8 AND DamageModifier=1.0;
-- Keen-Eyed Striker (241816): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=274; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=1.986718 WHERE entry=241816 AND DifficultyID=8 AND DamageModifier=1.0;
-- Sentinel of Winter (244100): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=50; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=4.657524 WHERE entry=244100 AND DifficultyID=8 AND DamageModifier=1.0;
-- Stormbound Mystic (245139): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=33; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=1.717736 WHERE entry=245139 AND DifficultyID=8 AND DamageModifier=1.0;
-- Ruthless Totemcaller (245143): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=314; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=1.243702 WHERE entry=245143 AND DifficultyID=8 AND DamageModifier=1.0;
-- Grizzled Warbringer (245146): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=106; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=2.295808 WHERE entry=245146 AND DifficultyID=8 AND DamageModifier=1.0;
-- Loyal Saberfang (245190): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=55; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=2.112676 WHERE entry=245190 AND DifficultyID=8 AND DamageModifier=1.0;
-- Spirit of Hunger (245855): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=85; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=3.213976 WHERE entry=245855 AND DifficultyID=8 AND DamageModifier=1.0;
-- Nalorakk (246404): BDjZ2QF6pNr8xV4W/fight 1 (+2), Hx64j9RrfnytB8cA/fight 12 (+2), YaZ7RKjBnp6dyHTG/fight 53 (+2); n=11; see sidecar for IQR/exclusions.
-- Retirement: replace with sourced server DamageModifier when available.
UPDATE content_creature_template_difficulty SET DamageModifier=5.405291 WHERE entry=246404 AND DifficultyID=8 AND DamageModifier=1.0;
DROP TABLE _wcl_damage_guard;
COMMIT;
