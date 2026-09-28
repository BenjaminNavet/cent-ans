# RS-G lot 1a — Bordeaux v2 vers 1340 (VH8)

Branche `feat/rs-g-cities`. Fichiers : `data/landmarks_v2/bordeaux.json`,
`tools/tests/test_landmarks_v2_bordeaux.py`. Format : `docs/landmarks-v2.md`.

## État
- 28/09 : squelette valide (origine sur la cathédrale Saint-André, OSM ; fleuve fin « Garonne »,
  ≈ 500 m de large dans `hydro_fine`).
- 28/09 : enceintes (troisième enceinte en deux polylignes ouvertes : front de terre avec fossé,
  mur de Garonne sans fossé ; castrum et deuxième enceinte intérieurs), 13 monuments, 5 quartiers,
  places, grève, Peugue et Devèze à la main. Script d'auteur hors dépôt (scratchpad).
- 28/09 : 406 rues OSM (67 principales ; cours, quais, allées et places des XVIIIe-XXe s. exclus),
  Garonne fine ; 4 murs, 26 portes, 13 monuments, 5 quartiers, 4 espaces libres, 1 grève.
  `pytest tools/tests/test_landmarks_v2.py tools/tests/test_landmarks_v2_bordeaux.py` : 60 passés.

## Sources principales
- L. Drouyn, *Bordeaux vers 1450* (1874, domaine public, archive.org `bordeauxvers145000drou`) :
  tracés des trois enceintes par les rues de 1874, tours, portes (texte seulement).
- F. Boutoulle, « Enceintes, tours, palais et castrum à Bordeaux… », *RAB* XCIV (2003) :
  deuxième enceinte 1205-1255, Ombrière, Arbalesteyre.
- Wikipédia FR (remparts et portes, monuments), Bordeaux Métropole (carte des patrimoines).
- OSM (ODbL) : positions projetées (pyproj) des églises, portes, rues repères.

## Manques de sources et restitutions hypothétiques
- Front nord de la troisième enceinte entre les allées d'Orléans et le milieu des Quinconces
  (bouleversé par le château Trompette, connu par les textes seuls) ; pointe sud-est (fort Louis -
  Garonne) : tracés hypothétiques.
- Portes placées d'après une mention unique (hypothétiques) : Sainte-Eulalie (déplacée en 1603),
  Mirail, grand portail Sainte-Croix, Sainte-Croix de la Rivière, la Grave, Pey-Miqueu ; porte du
  Far au bout de la rue du Hâ (probable). Omises faute de position : Saint-Symphorien, Audeyole,
  Casse, Redge, Beyssac, Brisson, Sous-le-Mur, porte Neuve (poterne, 1255).
- Deuxième enceinte restituée en un seul mur (elle était doublée à ~10 m) ; date de comblement des
  fossés au XIVe s. inconnue (fossés non restitués). Castrum : fossés non restitués.
- Largeur des fossés de la troisième enceinte (18 m) restituée.
- Ombrière : site attesté, enceinte et disposition restituées (plans du XVIIIe s. seulement,
  non consultés).
- Gabarits restitués : Saint-Michel ancienne (emplacement et taille inconnus), Saint-Pierre avant
  1358, Saint-Rémi, Saint-Projet, couvent des Dominicains (position dans les allées de Tourny),
  hauteurs de toutes les églises, tour Pey-Berland (hauteur en 1440-1453).
- Couvents des Cordeliers, Carmes, Augustins, hôpital Saint-Julien (1231), Puy-Paulin, tour
  d'Arsac, maison commune près de la porte Saint-Éloi : omis (positions non sourcées).
- Devèze : seul le cours sous la rue de la Devise jusqu'à la Garonne est tracé (entrée ouest,
  près du pont de La Mothe, non localisée) ; elle longe la rue OSM de même nom.
- Bourg Saint-Seurin : emprise restituée ; faubourgs du sud (Saint-Julien) et des Chartrons
  (Chartreux 1383) omis.
- Grève du port de la Lune : la Garonne fine est ≈ 150-280 m à l'est du mur médiéval (limite de
  `hydro_fine`, comme à Rouen) ; la grève (`strand`) borde le mur.

## Format
- Pas de tours de transept dans `gothic_cathedral` (Saint-André : deux tours au transept nord,
  flèches du XVe s.) ; pas de nef unique sans bas-côtés (`width_m` 24 pour une nef de 18 m).
- Pas de `certainty` sur les portes ni sur les tronçons d'enceinte : dit dans les `note`.
- Troisième enceinte en deux polylignes ouvertes pour n'avoir de fossé que du côté de la terre
  (le moteur met le fossé à droite du sens de parcours d'une polyligne ouverte).

## Prochaine étape
Lot terminé. Suites : relecture historienne (lot 1e), recette visuelle Godot (session principale),
Atlas historique des villes de France (Bordeaux, 2009) à consulter pour les manques ci-dessus.
