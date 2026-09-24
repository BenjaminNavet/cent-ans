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
- [ ] générateur : Seine, îles, enceintes, rues, tissu urbain
- [ ] Notre-Dame soignée, puis autres monuments
- [ ] rendu campagne (remplacement de la maquette, LOD, année)
- [ ] captures avant/après
- [ ] ville de siège (si le temps le permet)

## Prochaine étape
Générateur Blender : maillage par matériau, loupe, Seine et îles.
