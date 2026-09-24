# BR1 — Bâtiments réalistes (campagne et bataille)

Branche `br1-buildings`, worktree `../gp-br1`. ADR 0021. Session séparée (le joueur est absent
plusieurs heures, autonomie complète).

## État

- [x] 0. Squelette : ADR, wip, textures Poly Haven (`game/assets/textures/buildings/`, 3,3 Mo)
- [ ] 1. Kit Blender `tools/blender_scripts/building_kit.py` (recettes high/low, UV mètres, couleurs de sommet)
- [ ] 2. Matériaux Godot `game/scripts/visual/building_materials.gd`
- [ ] 3. Bataille : `battle_village.gd` et `battle_siege.gd` posent les GLB du kit
- [ ] 4. Campagne : `settlements.py` / `models.py` régénérés avec le niveau low
- [ ] 5. Captures avant/après, mesures FPS, fusion

## Prochaine étape

Écrire le kit et rendre des aperçus Blender.
