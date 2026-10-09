# TX T2c — sols de bataille régionaux (branche tx-battle)

Spec : docs/superpowers/specs/2026-10-09-textures-regionales-design.md § 2c. Base : 79e60b9ee.

## État
- 2026-10-09 : démarrage, squelette.

## Plan
1. Paquets par biome (13 couches, 1024 dépôt / 2048 local hi/), `packs:` dans ground_battle.yaml.
2. battle_ground_layers.json -> rôle -> matière par biome (+ schéma), repli biome -> parent -> défaut.
3. Micro-détail fondu (paquet micro_battle).
4. Captures et verdict vs Poly Haven.
5. Tests, ADR 0240.
