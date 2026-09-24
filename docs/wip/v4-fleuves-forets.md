# V4 — Fleuves, ponts et forêts de la carte de campagne (lots A1-11, A1-10)

Branche `worktree-agent-ae5685646cc8b59ac` (depuis `main` 356a1ad). Rendu seulement.
Captures : `docs/audit/captures/v4/` (avant `avant_*`, après `apres_*`).
Banc : `godot --rendering-driver vulkan --disable-vsync --path game --script res://tests/v4_map_bench.gd`
(Metal plafonne à 145 i/s et ne rend pas le temps GPU ; Vulkan/MoltenVK donne le temps GPU).

## Point d'accroche pour L1 (villes emblématiques : Paris)

Une **zone personnalisée** retire le rendu générique des fleuves pour qu'un modèle dédié prenne
le relais. Tout est piloté par `data/map/river_styles.json` → `custom_zones` (schéma
`data/schemas/river_styles.schema.json`) :

```json
{ "id": "paris", "name": "…", "lonlat": [2.3488, 48.8534], "radius_px": 4.8, "boundary_bridges": true }
```

- `lonlat` : centre (Notre-Dame) ; `radius_px` : rayon en pixels carte (1 px = 719 m) ; 4,8 = cercle des murs de la maquette générique actuelle (rayon 6 × 0,8).
- Après modification : `uv run --project tools cent-ans geo rivers-render` régénère
  `data/map/rivers_render.json` (tronçons d'eau **coupés** dans le cercle, zone recopiée avec son
  centre en px), `data/map/river_bed.png` (pas de lit creusé ni de berges dans le cercle) et
  `data/map/crossings_px.json` (`"in_custom_zone": true` pour les ponts dont la position source est
  dans le cercle : le Grand-Pont et le Petit-Pont de Paris ne sont pas dessinés par V4).
- En jeu (`RiversRenderer`, `game/scripts/map/rivers_renderer.gd`) : aucun ruban d'eau, lit, berge,
  pont de `crossings.json` ni pont-porte de muraille de colonie dans la zone.
  `boundary_bridges: true` pose un pont-porte générique (pierre crénelée, `BridgeMeshes` « gate »)
  là où un fleuve entre dans la zone ou en sort ; L1 le passe à `false` s'il dessine ses propres
  entrées d'eau. `RiversRenderer.custom_zones()` rend les zones (id, centre px, rayon) pour L1.
- La maquette générique de Paris (`SettlementLayer`) n'est pas touchée par V4 : c'est à L1 de la
  remplacer.

## Plan
### A1-11 fleuves et ponts
1. Outil `tools/cent_ans_tools/geo/river_render.py` (`cent-ans geo rivers-render`) — fait.
2. `terrain.gdshader` + `river_bed.gdshaderinc` : lit creusé (sommets abaissés dans les tuiles de
   relief fin), berges (vase, roseaux, herbe grasse), pente des berges dans l'ombrage.
3. `rivers_renderer.gd` + `river_water.gdshader` : ruban d'eau avec test de profondeur (plus de
   fleuve dessiné par-dessus les murs), écoulement, reflets, eau peu profonde en bord, gués.
4. `river_crossings.gd` + `bridge_meshes.gd` : ponts 3D procéduraux (pierre à arches, bois sur
   pilotis, bateaux), bacs, gués ; ponts-portes aux murs des colonies traversées par un fleuve
   (l'eau passe sous la ville).
### A1-10 forêts
5. Essences : chênaie, hêtraie, conifères de montagne, bocage/haies ; maillages procéduraux
   distincts ; canopée continue (masse par tuile + arbres de lisière) ; teinte saisonnière.

## Performance (avant, Vulkan, meilleur de 3 × 150 images, machine partagée)
| Vue | GPU ms | primitives | appels |
|---|---|---|---|
| large (France, d=1500) | 6,06 | 0,29 M | 526 |
| très proche (Paris, d=22) | 16,46 | 7,6 M | 699 |
| forêt (Orléanais, d=90) | 24,87 | 8,7 M | 1405 |
| Loire (Orléans, d=40) | 17,96 | 9,0 M | 842 |

## État
- [x] squelette, données (`river_styles.json`, `structure` dans `crossings.json`), outil + tests
- [ ] shaders, rendu de l'eau, ponts (en cours)
- [ ] forêts
