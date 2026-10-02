# FA7 — herbe des batailles en vrais brins (2026-10-02)

Lot du chantier FA (`docs/wip/fa.md`). Branche `feat/fa-grass`, worktree `../gp-fa-grass`.
Brutes et captures hors dépôt : `~/dev/cent-ans-raw/fa/ambientcg/`, `~/dev/cent-ans-raw/fa/grass-shots/`
(`shots.sh <tag> <core|all|vues>` prend les vues, `pair.py` assemble avant/après).

## Consigne de reprise
> Lis ce fichier et `git log --oneline -10`, puis continue à « Prochaine étape ».

## Fait
- Atlas `game/assets/textures/battle/grass_tufts.png` (4 × 2 cases de 512 × 256, BC7, mipmaps) :
  8 touffes (2 rases, 2 hautes, 2 à épis, 2 herbes folles avec pissenlit et pâquerettes) composées
  par `build_fa_grass.py` depuis Foliage001-008 et LeafSet020 (ambientCG, CC0).
- Catalogue `data/art/battle_grass.json` + schéma + `tools/tests/test_art_battle_grass_schema.py`.
- Shader : chemin `fa_on` (case tirée par carte, herbe rase dans les vides, teinte du sol).
- `BattleVegetation` : maillage à 3 cartes d'un mètre, `--no-fa-grass` pour l'A/B.

## Prochaine étape
- Réglages sur captures : clarté de l'herbe trop sombre par rapport au sol, bords francs autour
  des plaques de sol nu, épis trop roux.
- Vues à contrôler : rapprochée (4 terrains, 3 saisons), mêlée, déploiement, bv3, blé.
- Banc avant/après, docs (CREDITS, README des textures), tests Godot.

## Points ouverts
- Fleurs dessinées (pas d'atlas de fleurs CC0 dans les sources retenues).
