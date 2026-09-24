# L1 — Villes emblématiques : Paris d'abord

Branche `worktree-agent-a36a911aec4fbbb5c` (depuis `main` 93d466c). Backlog : `docs/audit/backlog-tw.md`
§ « Villes emblématiques ». Captures : `docs/audit/captures/l1/`.

## Gabarit
- Plan : `data/landmarks/<id>.json` (schéma `data/schemas/landmark.schema.json`), coordonnées locales
  en mètres réels (x est, y nord vrai, origine `anchor.lonlat` = Notre-Dame pour Paris).
- Loupe radiale (`scale`) : centre à 1 unité = 205 m (×3,5 par rapport à la carte), cœur de 2 km
  → 6 px, raccord linéaire jusqu'à la zone réservée (6,8 px) où l'échelle redevient celle de la carte.
- Générateur : `tools/blender_scripts/landmark_city.py` (Blender en ligne de commande) → GLB dans
  `game/assets/models/landmarks/`.
- Test : `tools/tests/test_landmarks.py` (schéma, ancrage px et nord EPSG:3035, loupe monotone).

## Point d'accroche V4 (fleuves)
V4 lit `data/map/river_styles.json` → `custom_zones` : `{ "id": "paris", "lonlat": [2.3499, 48.853],
"radius_px": 6.8, "boundary_bridges": false }` (valeurs L1 ; V4 proposait 4,5 px). À reporter à la fusion.

## État
- [x] squelette : schéma, données de Paris, test, script vide
- [x] générateur : Seine, îles, enceintes, rues, tissu urbain (Voronoï, maisons le long des îlots)
  `landmark_city.py` + `landmark_geometry.py` ; aperçus `landmark_preview.py` (EEVEE)
- [x] Notre-Dame soignée, puis autres monuments (`landmark_monuments.py`)
- [x] plan recalé sur OSM (berges, îles, vestiges des enceintes, monuments)
- [x] rendu campagne : `LandmarkModel` (drapé shader `landmark.gdshader`, année via
  `get_date_label`, LOD maisons/îlots), `LandmarkLibrary`, crochet dans `SettlementLayer`,
  zoom rapproché (7) au-dessus de Paris (`CampaignCamera.close_zones`)
- [ ] captures avant/après finales, ADR 0015
- [ ] ville de siège (si le temps le permet)

## Régénérer
`blender --background --python tools/blender_scripts/landmark_city.py -- data/landmarks/paris.json game/assets/models/landmarks/paris.glb`
puis `godot --headless --path game --import`. Le GLB (≈ 10 Mo) n'est commité qu'aux étapes finales.

## Prochaine étape
Captures finales (proche, moyen, loin, années 1340/1380), ADR, puis siège.
