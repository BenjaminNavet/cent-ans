# Lot V4 — Batailles semi-réalistes (branche `visual-v4`)

## État
- [x] Textures Poly Haven CC0 + procédurales dans `game/assets/textures/battle/` (README, `build_textures.py`)
- [x] Ciel, lumière, météo (`battle.tscn`, `battle_sky.gdshader`, `battle_atmosphere.gd`, particules GPU) — premier jet
- [x] Sol texturé (splatmap, `battle_ground.gdshader`), anneaux de collines — premier jet
- [x] Herbe (`battle_vegetation.gd`, `battle_grass.gdshader`), arbres/buissons/rochers — premier jet, à régler (herbe trop sombre)
- [x] Rivière (`battle_water.gdshader`) — premier jet, largeur à régler
- [x] Figurines + animation par shader (`battle_soldier.gdshader`, `battle_soldiers.gd`), cadavres, bannières (`battle_banner.gdshader`) — premier jet, caparaçon trop gros
- [ ] Sièges (pierre, toits, maisons au sol)
- [ ] Performance, captures après

## Mesures « avant » (Apple M4 Pro, `--disable-vsync`, 600 images)
- `--benchmark --units=20` (40 unités, 4800 soldats) : 141,3 i/s
- `--benchmark` (14 unités, 1160 soldats) : 176,8 i/s
- `--benchmark --siege` (12 unités, 972 soldats) : 201,3 i/s

## Prochaine étape
Itérer sur les captures (`--closeup`, `--weather=`), puis sièges (`battle_siege.gd`).
