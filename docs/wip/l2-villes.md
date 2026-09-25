# L2 — Villes emblématiques (suite) : Londres, Avignon, Calais, Rouen, Bordeaux, Bruges

Gabarit L1 (`docs/wip/l1-paris.md`, ADR 0015 + addendum L2) appliqué à six villes. Captures :
`docs/audit/captures/l2/`.

## État : terminé (à fusionner par l'orchestrateur)

- [x] Schéma étendu (`data/schemas/landmark.schema.json`) : `monuments[].params` (gabarits paramétrés,
  `variants` fusionnés pour `variant_from_year`), modèles `gothic_cathedral`, `castle`, `belfry`,
  `royal_palace`, `gate_tower`, `papal_palace`, `cog` ; ponts `arches` (piles à avant-becs et reins
  d'arches), `chapel_at`, `gatehouses_at`, `drawbridge_at` ; plans d'eau `waters` (découpés à la zone).
- [x] Gabarits réutilisables dans `tools/blender_scripts/landmark_monuments.py` : cathédrale gothique
  (chevet rond ou plat, tours de façade dont « pas encore bâtie », tour de croisée et flèche, flèches de
  transept), château (enceintes concentriques, donjon rond/carré/White Tower, douves, salles, portes),
  beffroi (étages carrés/octogonaux, flèche/lanterne, halles), palais à grande salle et chapelle palatine,
  porte à tours, cogue. Seul le palais des Papes est un maillage dédié.
- [x] Poids : maisons sans couvercle, couleurs en octets (`compact_glb`). Londres 5,8 Mo, Avignon 4,8,
  Calais 2,5, Rouen 5,8, Bordeaux 5,5, Bruges 5,6 (test `test_landmark_model_weight`, ≤ 6 Mo).
- [x] Hauteurs cuites conservatrices (`landmark_model.gd`, max sur le texel) : l'eau et les quais ne
  plongent plus sous le lit creusé des rivières de la carte (Rouen).
- [x] **Londres** : Tour de Londres (White Tower blanchie, deux enceintes, douves), Old St Paul's (flèche
  149 m, chevet plat à rose), London Bridge (19 arches, chapelle Saint-Thomas, pont-levis, portes),
  Westminster (abbaye ; Hall + chapelle Saint-Étienne, variante Richard II 1394 ; horloge 1367 ; tour du
  Joyau 1365), Savoy jusqu'en 1381, Southwark, Lambeth, mur de Londres et portes, Fleet, cogues du Pool.
- [x] **Avignon** : palais des Papes (Palais Vieux ; Palais Neuf de Clément VI dès 1342), pont
  Saint-Bénézet (22 arches, chapelle, châtelet), remparts datés 1358 (enceinte du XIIIe jusqu'en 1370),
  tour Philippe-le-Bel, fort Saint-André 1362, Chartreuse 1356, deux bras du Rhône, Durance.
- [x] **Calais** : ville close à fossés, château, tour du Guet, Notre-Dame, havre, tour de Rysbank dès
  1350, Étape des laines 1363, cogues. Ancre sur le havre : la mer de la carte reste en place.
  Le Pale de Calais est un territoire (propriété de province), pas un élément de maquette.
- [x] **Rouen** : cathédrale (tour Saint-Romain, flèche de croisée ; tour de Beurre en 1506), Saint-Ouen,
  château de Bouvreuil et son donjon, beffroi du Gros-Horloge 1389, pont de pierre, île Lacroix, Robec.
- [x] **Bordeaux** : Saint-André (nef unique, flèches du transept nord), tour Pey-Berland 1440, palais de
  l'Ombrière, Grosse Cloche, Saint-Michel, Sainte-Croix, Saint-Seurin, enceinte et quais, port de la Lune
  et nefs du vin, Peugue.
- [x] **Bruges** : beffroi et cour des halles (lanterne octogonale 1486), Waterhalle, Stadhuis 1376,
  Saint-Donatien, tour de Notre-Dame (115 m), Saint-Sauveur, reien, enceinte de 1297 et vesten, portes.
- [x] Captures Godot : `londres_d24/d9/tour_d7`, `avignon_d9`, `calais_d9/d7`, `rouen_d9/d7_1400`,
  `bordeaux_d9/d7`, `bruges_d9/d7_1490`. Option `--hide-armies` ajoutée à `campaign_map.gd`.

## Point d'accroche V4 (fleuves) — à reporter à la fusion
`data/map/river_styles.json` → `custom_zones` (le fichier n'est pas encore dans main). Tant que ce n'est
pas fait, le ruban générique du fleuve de la carte traverse la maquette (visible à Londres, Rouen,
Bordeaux, Avignon) :
```
{ "id": "london", "name": "Londres", "lonlat": [-0.11, 51.5125], "radius_px": 8.0, "boundary_bridges": false },
{ "id": "avignon", "name": "Avignon", "lonlat": [4.8075, 43.9508], "radius_px": 6.3, "boundary_bridges": false },
{ "id": "calais", "name": "Calais", "lonlat": [1.851, 50.96257], "radius_px": 5.3, "boundary_bridges": false },
{ "id": "rouen", "name": "Rouen", "lonlat": [1.095, 49.4402], "radius_px": 6.3, "boundary_bridges": false },
{ "id": "bordeaux", "name": "Bordeaux", "lonlat": [-0.5776, 44.8378], "radius_px": 6.4, "boundary_bridges": false },
{ "id": "bruges", "name": "Bruges", "lonlat": [3.2247, 51.2089], "radius_px": 6.3, "boundary_bridges": false }
```
CV1 : l'exclusion du rendu générique passe par `LandmarkLibrary.for_settlement` (même mécanisme que Paris).

## Régénérer
```
blender --background --python tools/blender_scripts/landmark_city.py -- data/landmarks/<id>.json game/assets/models/landmarks/<id>.glb
godot --headless --path game --import
```
Aperçus Blender : `landmark_preview.py -- <glb> <dossier> "nom=x,y,z,tx,ty,tz,objectif"`.
Captures : `godot --path game res://scenes/campaign_map.tscn -- --screenshot=<png> --stage=map --focus=x,y,d --hide-armies [--landmark-year=N]`.

## Points ouverts
- Pas de toile de fond de siège pour ces six villes (bloc `siege` optionnel, à ajouter comme Paris).
- Paris n'a pas été régénéré : il profiterait de la compaction (10 → 8,3 Mo) mais reste au-dessus de 6 Mo.
- Routes de la carte dessinées par-dessus les maquettes (comme à Paris).
- Monuments exagérés et loupe radiale : fortes compressions radiales en bord de noyau (Tour de Londres,
  Westminster) ; positions recalées OSM au mètre près, mais non à l'échelle entre elles.
- Datations approximatives signalées dans les JSON (Rysbank « vers 1350 », remparts d'Avignon 1358).
