# RV-D — occlusion de vallée précalculée

Branche feat/rv-d (worktree gp-rv-d). Chantier parent : docs/wip/rv-relief-vivant.md.

## État
- [x] Outil `cent-ans geo relief-occlusion` (`tools/cent_ans_tools/geo/relief_occlusion.py`) + tests.
- [x] Bake `data/map/relief_occlusion.png` (3584×3072 L8, 3,4 Mo, rayons 6/12/25 km, 16 azimuts, exag. ×3).
- [x] Include shader `game/shaders/relief_occlusion.gdshaderinc` + 2 lignes dans terrain.gdshader.
- [x] Chargement GDScript (map_data.gd + terrain_builder.gd) (terrain_builder.gd) avec repli.
- [x] Import, smoke (exit 0), captures avant/après (hc_shots, Massif central + Alpes, rig 700/250) : fonds de vallée −20 à −36 niveaux (1er centile), crêtes +10 ; repli sans fichier vérifié.

## Réglages (relief_occlusion.gdshaderinc)
rvd_strength 0.40, rvd_crest_lift 0.14, rvd_ao_strength 0.6, fondu de près (quadtree présent)
footprint 0.04 → 0.3 vers 0.5. Bake : rayons 6/12/25 km, 16 azimuts, exagération ×3, γ 0.75.

## Points ouverts
- Les arbres instanciés (forêts) ne reçoivent pas l'occlusion : collines boisées peu touchées.
- Réglage fin à faire après intégration avec RV-A (normales) et RV-B (soleil).

## Prochaine étape
Intégration dans feat/rv (orchestrateur).
