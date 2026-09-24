# BR1 — Bâtiments réalistes (campagne et bataille)

Branche `br1-buildings`, worktree `../gp-br1`. ADR 0021. Session séparée (le joueur est absent
plusieurs heures, autonomie complète).

## État

- [x] 0. Squelette : ADR, wip, textures Poly Haven (`game/assets/textures/buildings/`, 3,3 Mo)
- [x] 1. Kit Blender : `kit_geometry.py` (géométrie pure Python : murs percés, pans épais, poutres, bruit de patine), `building_kit.py` (11 recettes high/low, ruines calcinées, affaissement), `kit_export.py` (GLB + aperçus Eevee). Jeu de bataille exporté : `game/assets/models/buildings/` (42 GLB + `manifest.json`)
- [ ] 2. Matériaux Godot `game/scripts/visual/building_materials.gd`
- [ ] 3. Bataille : `battle_village.gd` et `battle_siege.gd` posent les GLB du kit
- [ ] 4. Campagne : `settlements.py` / `models.py` régénérés avec le niveau low
- [ ] 5. Captures avant/après, mesures FPS, fusion

## Commandes

- Aperçus : `blender -b --python tools/blender_scripts/kit_export.py -- preview <préfixe> cottage:11 timber:31 townhouse:47! [--low]`
- Export bataille : `blender -b --python tools/blender_scripts/kit_export.py -- export game/assets/models/buildings`

## Prochaine étape

Matériaux Godot (`building_materials.gd`) puis branchement bataille.
