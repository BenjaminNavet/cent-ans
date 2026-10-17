# fix-mapgeo — défauts géographiques du playtest du 17/10

Branche `fix/mapgeo`. Rapport source : `docs/wip/playtest-2026-10-17/carte-campagne.md`.

## État
- 1. Baie du Mont-Saint-Michel : reproduit (dalle noire). Cause : la pyramide de relief E1-E3 (cache hors git,
  `data/map/pyramid` -> `~/dev/cent-ans-raw/compose/pyramid`) n'avait jamais reçu la retouche de côte de mapgeo11
  (E0 et heightmaps oui) : la baie restait à +13 m sous un masque « mer ». Corrigé en rejouant
  `compose coast-patch` sur la pyramide (66 tuiles), rien dans git.
- 2. Léman : reproduit. Cause : `flatten_lakes` aplatit le lac au 10e centile de la rive (607 m), alors que le relief réel
  garde le vrai niveau (366 m) autour. Corrigé par `compose lake-patch` (12+ lacs, voir rapport de l'outil) : E0, E1-E3,
  heightmaps, `lakes.json` (`level_m`, `level_patched_from`).

- 3. Villes sur l'eau : Constantinople reproduit (bloc carré dont l'angle sud dépasse sur le Bosphore). Cause : aucune
  contrainte terre/mer sur la pose des maquettes. Corrigé : `placement::shore_fit` (maquettes et villes-repères) tire la
  maquette vers la terre (<= 1,25 rayon) quand plus de 4 % de son empreinte est sur l'eau ; dump `campaign_markers.ground.shore_moved`.
  Londres : le « aplat gris » est la Tamise cuite dans la texture du glb généré `landmark_london` (Meshy) ; le pont ne franchit
  rien parce que le modèle est une seule maille. Non corrigeable côté client sans regénérer le modèle (lot mapfx / modèles).
- 4. Rivières en aval : reproduit (Douro, ruban noir haché). Causes : (a) berges sombres (`look3.x`, `wet_band`) sur un ruban
  de quelques pixels = tout est berge ; (b) nœuds de terrain grossiers au-dessus du lit au loin. Corrigé dans
  `water/shaders/river.wesl` : berges proportionnelles à la largeur écran (`bank_k`), sommet glissé vers l'oeil le long
  de son rayon de vue (`FAR_EYE_PUSH`).
- 5. Côtes en escalier : cause = `coast_dist.png` issu d'une EDT sur masque binaire (contour en marches d'un pixel carte).
  Corrigé dans `geo/splat.py` (`compute_coast_dist` : distance au contour lissé près de la côte, exacte au loin) ;
  `coast_dist.png` régénéré (`splat.build_coast_dist()`), test `test_coast_distance_has_no_pixel_staircase`.
- 6. Étiquettes : la plaque d'armée « 620 » cachait « Paris » (nom épinglé = capitale, placé de force sous/sur la plaque).
  Corrigé : les noms de villes publient leur position préférée (`RegionLabelObstacles.names`) ; plaques, noms de régions
  et de rivières les évitent.

## Commandes à rejouer sur le cache de main après fusion

    cd tools && uv run cent-ans compose coast-patch <ancien land_mask (git show f30fa5d86^:data/map/land_mask.png)> --pyramid data/map/pyramid
    uv run cent-ans compose lake-patch --pyramid data/map/pyramid   (déjà fait ; rejouable, rapport vide)
NB : la pyramide partagée est un lien vers `cent-ans-raw` : elle a été retouchée en place par erreur (par ces deux commandes,
idempotentes) avant la fusion des données E0.

NB 2 : la pyramide partagée (`cent-ans-raw/compose/pyramid`) est DÉJÀ retouchée (coast-patch + lake-patch) : main, tant
que cette branche n'est pas fusionnée, a un relief E1-E3 cohérent avec la côte/les lacs mais un `lakes.json` et un E0 anciens.

## Prochaine étape
Vérifier après-captures (const40b, spain250b, malta80b), clippy/tests client, smoke.
