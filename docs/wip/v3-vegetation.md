# WIP — lot V3 « Végétation, villes et marqueurs de la carte de campagne »

Branche `visual-v3` (depuis `visual`, V1 inclus). Ne pas fusionner : l'orchestrateur fusionne.

## État
- [x] Végétation : `game/scripts/map/vegetation*.gd`, `game/shaders/foliage.gdshader`, nœud `Vegetation`
  dans `campaign_map.tscn` (autonome : se branche sur `map_data`/`load_ok` du parent et `CameraRig`).
  Tuiles 16×16 semées dans `WorkerThreadPool`, LOD (maillage détaillé / ≈ 20 triangles), éclaircissement
  par graine, vent. Masque : `data/map/splat.png` (B = forêt, G = cultures) sinon repli procédural.
- [ ] Réglages visuels forêts / haies / bosquets (captures)
- [ ] Villes (models.py : PBR, murs, toits ardoise/tuile)
- [ ] Marqueurs d'armée (figurines, bannière armoriée, plaque d'effectif, halo)
- [ ] Navires, camps de siège
- [ ] Captures finales `docs/img/visuel/v3_*.png`

## Prochaine étape
Réglages visuels de la végétation, puis modèles Blender.
