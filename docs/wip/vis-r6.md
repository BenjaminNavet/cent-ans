# VIS-R6b — lieux génériques Meshy (famille ouest)

## État (30/10)
- 4 maquettes (village, château, bourg, abbaye) ré-ingérées depuis `~/dev/cent-ans-raw/vis-r6/meshy/` (classe
  `settlement_maquette`, 30 000 / 10 000 / 3 000 triangles, texture 2048) ; manifeste versionné, glb dans le paquet de
  release (non republié).
- Densité village 900 → 350 ; `landmark_clearance` / `landmark_min_factor` (la grosse maquette cède) ; `tints` régionales.
- ADR 0803. Captures dans le scratchpad de la session (3 : Wissant d 90, camp d 300, Paris d 25).

## Reste / points ouverts
- La plaque « Saint-Denis » chevauche encore la cathédrale à d 25 : c'est une projection d'interface, pas une collision de
  maquettes (Saint-Denis est à 25 u de Paris, rayon 13 u). Corriger demande une occultation des plaques par la 3D.
- Republier le paquet de modèles (`cent-ans art models-update`) : accord explicite requis.
- Mesure fps/mémoire : non faite (machine saturée, jetons de `heavy.sh` pris des heures).
