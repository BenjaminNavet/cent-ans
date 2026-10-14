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
- [ ] Menu d'accueil (`menu_backdrop_3d.gd`, `paris_siege.glb`) → maquette Paris.
- [ ] Avignon, Bordeaux, Bruges, Calais, Londres, Rouen : images (3 graines), choix, Meshy (6 × 15 crédits),
  ingestion, entrées `landmarks`.
- [ ] Paquet de modèles (`cent-ans art models-update`) : publication avec l'accord du joueur.

## Points ouverts
- Batailles de siège et `LandmarkCityLayer` (v2) gardent les plans Blender : hors périmètre.
