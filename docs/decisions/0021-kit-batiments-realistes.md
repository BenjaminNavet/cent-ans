# ADR 0021 — Kit de bâtiments réalistes (carte de campagne et batailles)

Date : 2026-09-25. Statut : accepté. Complète ADR 0004 (direction semi-réaliste) et reprend les
points d'audit A1-12 (villes de siège denses et texturées) et A1-19 (colonies texturées, hors monuments).

## Contexte

Le joueur trouve les bâtiments « pas assez réalistes » sur les deux cartes :
- en bataille, chaque maison est une `BoxMesh` et un `PrismMesh` (toit sans épaisseur ni débord,
  colombage en planchettes posées sur le mur, pas de fenêtres, cheminée en poteau) ;
- en campagne, les maquettes `settlements.py` / `models.py` sont en couleurs unies sans texture
  (< 5 000 triangles pour une ville entière), maisons en pavés.

## Décision

- **Un kit Blender unique** `tools/blender_scripts/building_kit.py` génère les bâtiments par des
  recettes paramétrées (chaumière, longère, maison à colombage à encorbellement, maison de ville à
  pignon sur rue, maison de pierre, grange, église paroissiale, maison forte, moulin, halle, puits),
  à **deux niveaux de détail** : `high` (batailles : colombage en relief, fenêtres en retrait avec
  volets, portes, cheminées, lucarnes, toits épais à débords, faîtage) et `low` (maquettes de
  campagne : volumes, toits à débords et cheminées, ouvertures peintes).
- **UV en mètres réels** (projection cubique par face, calculée dans le kit) et **couleurs de
  sommet** (variation par bâtiment, assombrissement au pied des murs et sous les débords) ; aucune
  texture embarquée dans les GLB.
- **Matériaux nommés** (`Plaster`, `PlasterOchre`, `Rubble`, `Ashlar`, `Timber`, `Planks`,
  `RoofTile`, `RoofFlat`, `RoofSlate`, `Thatch`, `Window`, `Banner`…) remplacés à l'exécution par
  `game/scripts/visual/building_materials.gd` : matériaux PBR partagés (textures Poly Haven CC0 de
  `game/assets/textures/buildings/`, albédo × couleur de sommet), variantes neige et brûlé.
- Batailles : maisons choisies par type et proportions, posées par `MultiMesh` (une instance par
  maison, un appel de dessin par surface du modèle).
- Campagne : les mêmes noms de fichiers de colonies (`settlements/*.glb`, `town.glb`…) sont
  régénérés avec le niveau `low`, pour ne pas toucher aux scripts qui les chargent (CV1, L1).
- Coût : 0 $ (génération procédurale, textures CC0).

## Conséquences

Plus de triangles (maison de bataille 1 500 à 5 000, ville de campagne jusqu'à ~40 000) : portées de
visibilité inchangées, `MultiMesh` en bataille. Les modèles se régénèrent par une commande Blender
headless ; les captures avant/après sont dans `docs/audit/captures/br1/`.
