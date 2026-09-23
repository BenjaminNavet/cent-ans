# Lot V4 — Batailles semi-réalistes (branche `visual-v4`)

## État
- [x] Textures Poly Haven CC0 + procédurales dans `game/assets/textures/battle/` (README, `build_textures.py`)
- [ ] Ciel, lumière, météo (`battle.tscn`, `battle_sky.gdshader`, particules GPU)
- [ ] Sol texturé (splatmap, `battle_ground.gdshader`), anneau de collines lointaines
- [ ] Herbe animée, arbres, buissons, rochers
- [ ] Rivière (`battle_water.gdshader`)
- [ ] Figurines refaites + animation par shader (`battle_soldier.gdshader`), cadavres, bannières
- [ ] Sièges (pierre, toits, maisons au sol)
- [ ] Performance, captures après

## Mesures « avant » (Apple M4 Pro, `--disable-vsync`, 600 images)
- `--benchmark --units=20` (40 unités, 4800 soldats) : 141,3 i/s
- `--benchmark` (14 unités, 1160 soldats) : 176,8 i/s
- `--benchmark --siege` (12 unités, 972 soldats) : 201,3 i/s

## Prochaine étape
Environnement et shaders du sol.
