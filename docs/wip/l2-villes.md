# L2 — Villes emblématiques (suite) : Londres, Avignon, Calais, Rouen, Bordeaux, Bruges

Gabarit L1 (`docs/wip/l1-paris.md`, ADR 0015) appliqué à d'autres villes. Captures : `docs/audit/captures/l2/`.

## État

- [x] Schéma étendu (`data/schemas/landmark.schema.json`) : `monuments[].params` (gabarits paramétrés,
  `variants` fusionnés pour `variant_from_year`), nouveaux modèles `gothic_cathedral`, `castle`, `belfry`,
  `royal_palace`, `gate_tower`, `papal_palace`, `cog` ; ponts `arches` (piles à avant-becs et reins
  d'arches), `chapel_at`, `gatehouses_at`, `drawbridge_at` ; plans d'eau `waters` (découpés à la zone).
- [x] Gabarits réutilisables dans `tools/blender_scripts/landmark_monuments.py` (cathédrale gothique
  paramétrable, château à enceintes concentriques/donjon/douves, beffroi à étages et halles, palais à grande
  salle + chapelle palatine, porte à tours, cogue) ; `landmark_geometry.offset_polygon`, `box(top=False)`.
- [x] Poids : couvercles de maisons supprimés, couleurs en octets (`compact_glb`) : Paris 10 → 8,3 Mo si
  régénéré (non régénéré ici). Londres 5,8 Mo, Avignon 4,8 Mo, Calais 2,5 Mo.
- [x] **Londres** (`data/landmarks/london.json`) : Tour de Londres (White Tower blanchie, deux enceintes,
  douves), Old St Paul's (flèche 149 m, chevet plat à rose), London Bridge (19 arches, chapelle Saint-Thomas,
  pont-levis, portes), Westminster (abbaye, Hall + chapelle Saint-Étienne, variante Richard II 1394, tour de
  l'horloge 1367, tour du Joyau 1365), Savoy jusqu'en 1381, Southwark, Lambeth, mur de Londres et portes,
  Fleet, cogues du Pool.
- [x] **Avignon** : palais des Papes (Palais Vieux, variante Palais Neuf de Clément VI dès 1342), pont
  Saint-Bénézet (22 arches, chapelle, châtelet), remparts datés 1358 (enceinte du XIIIe jusqu'en 1370),
  tour Philippe-le-Bel, fort Saint-André 1362, Chartreuse 1356, deux bras du Rhône, Durance.
- [x] **Calais** : ville close à doubles fossés, château à l'ouest, tour du Guet, Notre-Dame, havre, tour de
  Rysbank dès 1350, Étape des laines 1363, cogues. Ancre sur le havre (la côte, droite passant par l'ancre,
  reste en place malgré la loupe : la mer de la carte suffit, pas de polygone de mer).
- [x] Captures Godot : `londres_d24`, `londres_d9`, `londres_tour_d7`, `avignon_d9`, `calais_d9`, `calais_d7`.
  Option `--hide-armies` ajoutée à `campaign_map.gd` (les marqueurs d'armée cachaient le palais des Papes).
- [ ] Rouen, Bordeaux, Bruges.

## Point d'accroche V4 (fleuves) — à reporter à la fusion
`data/map/river_styles.json` → `custom_zones` (le fichier n'est pas encore dans main) :
```
{ "id": "london", "name": "Londres", "lonlat": [-0.11, 51.5125], "radius_px": 8.0, "boundary_bridges": false },
{ "id": "avignon", "name": "Avignon", "lonlat": [4.8075, 43.9508], "radius_px": 6.3, "boundary_bridges": false },
{ "id": "calais", "name": "Calais", "lonlat": [1.851, 50.96257], "radius_px": 5.3, "boundary_bridges": false }
```
CV1 : l'exclusion du rendu générique passe déjà par `LandmarkLibrary.for_settlement` (même mécanisme que Paris).

## Régénérer
```
blender --background --python tools/blender_scripts/landmark_city.py -- data/landmarks/<id>.json game/assets/models/landmarks/<id>.glb
godot --headless --path game --import
```
Aperçus Blender : `landmark_preview.py -- <glb> <dossier> "nom=x,y,z,tx,ty,tz,objectif"`.
Captures : `godot --path game res://scenes/campaign_map.tscn -- --screenshot=<png> --stage=map --focus=x,y,d --hide-armies [--landmark-year=N]`.

## Prochaine étape
Rouen (cathédrale, pont Mathilde, château de Philippe Auguste, Gros-Horloge 1389), puis Bordeaux, Bruges.
