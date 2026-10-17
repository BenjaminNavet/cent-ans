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

## Commandes à rejouer sur le cache de main après fusion
    cd tools && uv run cent-ans compose coast-patch <ancien land_mask (git show f30fa5d86^:data/map/land_mask.png)> --pyramid data/map/pyramid
    uv run cent-ans compose lake-patch --pyramid data/map/pyramid   (déjà fait ; rejouable, rapport vide)
NB : la pyramide partagée est un lien vers `cent-ans-raw` : elle a été retouchée en place par erreur (par ces deux commandes,
idempotentes) avant la fusion des données E0.

## Prochaine étape
Défauts 3-6 (villes sur l'eau, rivières aval, côtes en escalier, étiquettes).
