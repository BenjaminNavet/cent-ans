# DN-FORET : forêts de campagne

ADR 0216 (provisoire). Branche `worktree-agent-a5b06394d1aadcb00`, pas fusionnée.

## Fait
- Atlas d'imposteurs recuit depuis les glb DN : 40 essences (`ga3_vegetation_l2.py sheets` puis `atlas` ;
  `board` pour la planche de contrôle ; gains de couleur dans `data/art/tree_model_gains.json`).
- Modèles décimés (`lod1`) dans une zone autour du point visé : `DnTreeModels`, `foliage_model.gdshader`,
  `Vegetation._update_model_zone`, `VegetationScatter.split_rows` (Rust). `--no-dn-trees` pour comparer.
- Peuplements par massif : `core/crates/vegetation/src/stands.rs`, `data/art/forest_stands.json`,
  `game/scripts/map/forest_stands.gd` (146 massifs nommés + 23 types d'écorégion).
- Couverture : `data/map/landcover_params.json` (`cleared_scale` 0,55) -> `geo landcover` (forêt 40 -> 51 %).
- Arbres grossis (`generalised_tree_height` 1,5, `generalised_spacing` 1,3).
- Outils : `tools/dn_forest_bench.sh <distance> [--no-dn-trees]`, `game/tests/dn_forest_shot.gd`.

## Pour une session locale
Aucune génération n'a été faite (0 $ fal). Essences à refaire ou à ajouter :
- `tree_orchard_apple` : glb cassé (arbre éclaté) ; le pommier garde son ancienne planche d'image.
- `tree_spruce_siberian` : glb cassé ; l'épicéa de Sibérie réutilise `tree_spruce`.
- Chêne kermès, lentisque : pas de glb DN ; anciennes planches d'image.
- Modèles proches : les `lod1` TRELLIS sont des amas de facettes ; un maillage de houppier propre
  (cartes de feuillage ou décimation sans éclatement) améliorerait le proche (< 40 unités).
- Pin maritime des Landes : le massif « landes_gascogne » est de la lande (peu d'arbres) ; un vrai pinède
  demande de décider si l'on plante le massif (anachronique avant le XIXe siècle).

## Reste
- Éclaircissement du modèle (`MODEL_BOOST` 1,55) à juger en jeu à côté des imposteurs.
- Semis GDScript de repli : sans peuplements (le Rust natif est la voie normale).
- Banc à machine calme ; captures de revue sur Landes, Alpes, Suède, Oural, Carpates.
