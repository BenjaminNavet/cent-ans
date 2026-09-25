# ADR 0015 — Villes emblématiques : plan en données, maquette générée, loupe radiale

Date : 2026-09-25. Statut : accepté. Lot L1 (backlog `docs/audit/backlog-tw.md`, « Villes emblématiques »).

## Contexte

Le joueur veut, comme Rome ou Constantinople dans Total War, des villes reconnaissables : Paris
d'abord (la Seine, l'île de la Cité, Notre-Dame), puis Londres, Avignon, Calais, Bordeaux, Rouen,
Bruges. À l'échelle de la carte (1 px = 719 m), Paris intra-muros tient dans 6 px et Notre-Dame
dans 0,2 px : une maquette à l'échelle vraie est illisible, une maquette générique ne dit rien.

## Options

- **A** : maquette modelée à la main par ville (Blender interactif) — coûteuse, non reproductible,
  difficile à corriger quand une source change.
- **B** : génération par IA (Hunyuan3D, Rodin) — payante, invérifiable historiquement, style
  incohérent avec les maquettes procédurales du reste de la carte.
- **C** : plan historique en données (`data/landmarks/<id>.json`, schéma `landmark.schema.json`),
  générateur Blender procédural en ligne de commande pour le tissu urbain, maillages dédiés
  paramétrés pour les monuments, rendu Godot générique.

## Décision

**C**.

- **Plan** en mètres réels, nord vrai, origine sur un point connu (Notre-Dame) : fleuve (axe +
  largeur), îles, quartiers bâtis (densité), espaces ouverts, enceintes (tours, portes, fossés),
  ponts (habités ou non), grandes rues, monuments (gabarit, angle, échelle). Les éléments datés
  (`from_year`, `until_year`, `variant_from_year`) deviennent des nœuds séparés du glTF.
- **Loupe radiale** : r_monde = A·r + B·r² au cœur (Paris : centre ×3,5, 2 km → 6 px), raccord
  linéaire jusqu'au bord d'une **zone réservée** (6,8 px) où l'échelle redevient celle de la carte.
  Le fleuve et les routes de la carte se raccordent au bord ; dans la zone, le rendu générique des
  fleuves, ponts, arbres et hameaux s'efface (point d'accroche `custom_zones` de V4 dans
  `data/map/river_styles.json`, `SettlementLayer.covered_by_landmark`).
- **Monuments** exagérés en plus (Notre-Dame ×2,6 par rapport au centre) : lisibles au zoom
  rapproché, à la manière de Total War.
- **Maquette** à plat, drapée sur le relief affiché par le shader (`landmark.gdshader`) : chaque
  sommet porte en UV son point d'ancrage (le centre de son bâtiment, qui reste rigide) ; les
  hauteurs sont cuites dans une texture 96² recalculée quand une tuile de terrain change de niveau.
- **LOD** par portées de visibilité : maisons détaillées < 120, îlots simplifiés au-delà, sol,
  Seine et monuments jusqu'à 1 100-2 000.
- **Année** : la couche datée est affichée selon l'année lue dans le libellé de date de la
  simulation (rendu seulement, aucune règle).
- **Caméra** : zoom minimal ramené de 22 à 7 au-dessus d'une ville emblématique.

## Conséquences

- Nouvelle ville = un JSON (+ au besoin un gabarit de monument dans `landmark_monuments.py`) et
  une commande Blender ; pas de dépense.
- Le GLB de Paris pèse ≈ 10 Mo (≈ 110 000 triangles, 5 000 maisons) ; à régénérer après toute
  modification du plan.
- Sources : plans du domaine public (plan de Bâle, Legrand 1380, Viollet-le-Duc) et OpenStreetMap
  (ODbL) pour le recalage de positions ; citées dans le JSON.
- La bataille de siège à Paris réutilise le même plan (voir `docs/wip/l1-paris.md`).
