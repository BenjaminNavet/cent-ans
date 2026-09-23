# WIP — lot V3 « Végétation, villes et marqueurs de la carte de campagne »

Branche `visual-v3` (depuis `visual`, V1 + V2 fusionnés). Ne pas fusionner : l'orchestrateur fusionne.

## État
- [x] Végétation : `game/scripts/map/vegetation*.gd`, `game/shaders/foliage.gdshader`, nœud `Vegetation`
  dans `campaign_map.tscn` (autonome : se branche sur `map_data`/`load_ok` du parent et `CameraRig`).
  Tuiles 16×16 semées dans `WorkerThreadPool` (4 parties par tuile pour le LOD), maillage détaillé /
  ≈ 20 triangles, éclaircissement par graine + densité globale au dézoom, vent, ombres des tuiles proches.
  Masque : `MapData.splat_image` (V2 : B = forêt, G+R = campagne ouverte) sinon repli procédural.
  Haies (parcellaire déformé, dense dans le bocage), bosquets et arbres isolés.
- [x] Banc : `godot --disable-vsync --path game --script res://tests/vegetation_bench.gd [-- --no-vegetation]`.
- [x] Villes (models.py : villes fortifiées entières, `city_cathedral`, palette PBR naturelle, fondations).
- [x] Marqueurs d'armée : figurines selon la composition, étendard armorié (`map_banner.gdshader`),
  anneau au sol (`Decal`), plaque d'effectif 2D, pas d'ombre au-delà de l'échelle 2,6.
- [ ] Navires, camps de siège, armées en marche : vérification visuelle (script de mise en scène)
- [ ] Captures finales `docs/img/visuel/v3_*.png`, test Python des noms de modèles

## Prochaine étape
Mise en scène des états d'armée (siège, à bord, en marche) pour captures ; captures finales ; rapport.
