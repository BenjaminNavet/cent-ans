# TX T2c — sols de bataille régionaux (branche tx-battle)

Spec : docs/superpowers/specs/2026-10-09-textures-regionales-design.md § 2c. Base : 79e60b9ee.

## État
- 2026-10-09 : démarrage, squelette.

- 2026-10-09 (2) : code en place, paquets binaires pas encore commités. Paquets battle_b01..b14
  (13 couches, 1024, normales 256, JPEG q80/85) = 77 Mo au total (4-6 Mo chacun), micro_battle 4,8 Mo.
  Chargeur `BattleGroundTextures`, `BattleTerrain.resolve_ground()`, shader (`tx_ground`, `micro_*`),
  `cent-ans textures battle-data` (matières par biome + `battle_province_biomes.json`).
  Reste : tests, captures, ADR 0240, commit des binaires après verdict visuel.

- 2026-10-09 (3) : TERMINÉ. Binaires commités (81 Mo dont .import), tests (om3, ga2, smoke, pytest) verts,
  ADR 0240, règle d2d289623 cherry-pickée + `models-textures-fix`. Captures :
  `~/dev/cent-ans-raw/textures/planches/battle_*.png`. Verdict : TX meilleur que Poly Haven.
  Reste : grain fin non jugé de près, boréal doré au printemps, fusion avec micro_ground de la campagne.
  Les variantes 2048 (hi/) et leurs tuiles ont été supprimées (disque) : `textures pack ground_battle --size 2048`.

## Plan
1. Paquets par biome (13 couches, 1024 dépôt / 2048 local hi/), `packs:` dans ground_battle.yaml.
2. battle_ground_layers.json -> rôle -> matière par biome (+ schéma), repli biome -> parent -> défaut.
3. Micro-détail fondu (paquet micro_battle).
4. Captures et verdict vs Poly Haven.
5. Tests, ADR 0240.
