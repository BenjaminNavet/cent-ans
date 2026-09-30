# TF — Colombage TimberFrame + toits du château Kenney (lot L4 de realisme-suite)

Branche `feat/tf`, worktree `/Users/jean_hubert/dev/game_project-tf`. Décisions : ADR 0105 §TF.

## État (30/09)
- [x] Atlas : `TimberFrame` couche 14 (fin de tableau), tableaux d'albédo/normales à 15 tranches
      (`build_textures.py`), shaders `building_atlas`/`town_building` : bloc uni
      `first_plain`..`plain_last` (11..13) au lieu du seuil `first_plain`.
- [x] Kit Blender : panneaux des murs à pans de bois nommés `TimberFrame` (`FRAME_PANEL`),
      variantes du Midi (`southern=True` : enduit/pierre, tuiles canal) ; réexport bataille
      (+10 modèles, manifeste `framed`/`southern`, correctif : l'export complet ne perd plus les
      chevaux du manifeste) et villes ZG6.
- [x] Choix régional : `data/art/building_regions.json` + schéma + pytest ;
      `BuildingRegions` (province → région → style), `BuildingKit.region_style` posé par
      `BattleTerrain.build` (`--no-tf` pour l'A/B).
- [x] Château Kenney (cités CV1) converti à l'atlas `Building` : ardoise, pierre, bois.
- [x] Tests : `tf_timber_frame_test.gd` (nouveau), ga5, sr5, zg6, vh4, smoke ; pytest schémas.
- [x] Captures A/B : `docs/img/tf/tf_ab.jpg` (village normand en bataille, cité + château).

- [x] Relecture (coordinateur) : lattis peint + poutres modelées = façade sombre et chargée.
      Corrigé : `TimberFrame` (14) = torchis clair sans poutres (`timber_daub_diff.jpg`) au niveau
      `high` ; lattis peint déplacé en `TimberFrameFar` (15) pour le niveau `low` (kit des villes).
      Atlas plein (16 couches). A/B mis à jour (`docs/img/tf/tf_ab.jpg`, rangée du milieu = gros plan).

## Points ouverts
- Atlas `Building` plein : une nouvelle matière demandera 32 tranches.
- Maquettes de colonies CV1 et monuments non réexportés (panneaux `Plaster`).
- Toits du Midi : seules les variantes `southern` ont des tuiles canal ; longères, granges et
  églises restent communes à toutes les régions.

## Prochaine étape
Fusion par la session principale ; jugement du joueur sur `docs/img/tf/tf_ab.jpg`.
