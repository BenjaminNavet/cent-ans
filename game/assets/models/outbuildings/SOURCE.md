# Bâtiments hors les murs de la carte de campagne (lot TB3)

- **Générés par** : `tools/blender_scripts/tb3_outbuildings.py`
  (`blender --background --factory-startup --python tools/blender_scripts/tb3_outbuildings.py -- export game/assets/models/outbuildings`).
  Ne pas retoucher à la main.
- **Sources** : aucune ; assemblages des recettes `low` du kit de bâtiments
  (`building_kit.py`) et de pièces procédurales (rangs de vigne, œillets de saline, jetée, grue à
  cage d'écureuil, manège et chevalement de mine, ailes de cloître, étals, tentes, barques,
  échafaudage). Aucune génération payante (ADR 0152, ADR 0162).
- **Licence** : CC0 1.0 (production du projet).
- **Conventions** : mètres, sol à y = 0, fondations en dessous, façade (+Z) = côté du site (eau
  pour moulins, salines et ports, route pour les marchés). Une seule surface `Building` par
  modèle (couche de l'atlas dans l'alpha de la couleur de sommet, comme `town_kit/`).
- `manifest.json` : famille, niveau, rayon d'emprise, longueur, profondeur, hauteur (m),
  triangles. `worksite_1` : chantier (lot TB3, point 6).
