# RV-D — occlusion de vallée précalculée

Branche feat/rv-d (worktree gp-rv-d). Chantier parent : docs/wip/rv-relief-vivant.md.

## État
- [x] Outil `cent-ans geo relief-occlusion` (`tools/cent_ans_tools/geo/relief_occlusion.py`) + tests.
- [x] Bake `data/map/relief_occlusion.png` (3584×3072 L8, 3,4 Mo, rayons 6/12/25 km, 16 azimuts, exag. ×3).
- [ ] Include shader `game/shaders/relief_occlusion.gdshaderinc` + 2 lignes dans terrain.gdshader.
- [ ] Chargement GDScript (terrain_builder.gd) avec repli.
- [ ] Import, smoke, captures avant/après.

## Prochaine étape
Shader et chargement.
