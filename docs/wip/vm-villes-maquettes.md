# VM — villes emblématiques en maquettes générées (Meshy)

Demande du joueur (14/10) : remplacer Paris (maquette Blender drapée, `LandmarkModel`) par une maquette
générée façon « maquette » comme les autres lieux, corriger le menu d'accueil (Notre-Dame derrière les
soldats), puis faire de même pour toutes les villes emblématiques 1:1. ADR 0356.

## Chaîne
Image Z-Image locale (gabarit « Clean 3D render… stylised miniature maquette », vue 3/4 haute) →
Meshy `meshy-6-lite` texturé (15 crédits) → `cent-ans dn-ingest --class landmark_city --length 2.0
--faithful` → entrée `landmarks.<id>` de `data/art/dn_campaign_models.json`.
Bruts et images : `~/dev/cent-ans-raw/meshy/paris_maquette/`, `~/dev/cent-ans-raw/meshy/landmarks/`.

## État
- [x] Paris : image `paris_v2_2024` validée par le joueur, glb `landmark_paris`, branché, test GC OK,
  captures avant/après faites (scratchpad).
- [x] Menu d'accueil : `backdrop.generated_model` (glb `landmark_paris_lod0`, largeur 1 000 m, matériau importé) ; repli sur `paris_siege.glb` si absent ; plans `cite_orbit`/`seine_upstream` recadrés.
- [x] Avignon (graine 2024), Bordeaux (7), Bruges (2024), Calais (2024), Rouen (2024) : glb ingérés et branchés.
- [ ] Londres : graines 2024/7/42 puis 11/23/57 donnaient le dôme de Wren (anachronique) ; prompt
  `london2.txt` sans le nom « St Paul's » (cathédrale gothique à flèche façon Salisbury) en cours.
- [x] Le Caire (demande du joueur 14/10) : `data/landmarks/cairo.json` (ancre, échelle, sans plan Blender),
  glb `landmark_cairo` (graine 2024). Schéma `landmark` : bornes lon/lat élargies à la carte OM.
- [x] Gizeh (pyramides + Sphinx) et Pétra : sites isolés de `map_landmarks_extra.json`, types
  `pyramids_giza` / `petra_ruins`, glb `env_giza_pyramids` (900 m) / `env_petra_ruins` (600 m),
  `fraction` 0,18 / 0,14, drapeau `known` (visibles sous le brouillard). Test `vm_wonders_test`.
- [ ] Paquet de modèles (`cent-ans art models-update`) : publication avec l'accord du joueur.

## Points ouverts
- `me6_decor_test` échoue déjà avant VM (zoom moyen Carnac : 14 instances pour > 15 attendues).
- Gizeh/Pétra : pas encore vus à l'œil après le drapeau `known` (budget de captures atteint).
- Batailles de siège et `LandmarkCityLayer` (v2) gardent les plans Blender : hors périmètre.
