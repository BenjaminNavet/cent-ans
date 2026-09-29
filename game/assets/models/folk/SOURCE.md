# Accessoires de la carte vivante (lot FK2)

- **Générés par** : `tools/blender_scripts/folk_props.py` (Blender 5.2,
  `blender -b --factory-startup --python tools/blender_scripts/folk_props.py -- game/assets/models/folk`),
  avec les primitives et la palette de `models.py`. Ne pas retoucher à la main.
- **Sources** : aucune ; géométrie entièrement procédurale, matériaux PBR unis.
- **Licence** : CC0 1.0 (production du projet).
- **Conventions** : mètres (même échelle que les figurines `battle_fine`), +X devant, origine au
  sol (voir la docstring du script) ; `manifest.json` donne triangles, boîtes et créneaux (repère
  Godot).
- Figurines civiles `villager_*` et clips `scythe` / `carry` / `plough` : voir
  `battle_fine/SOURCE.md` (même chaîne que les figurines de bataille).
