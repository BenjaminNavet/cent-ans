# OMR R7 — relief fin à l'Est et au Sud (paliers 1, E1-E2)

Lot R7 du chantier OMR (`docs/wip/omr.md`). Worktree `../gp-omr-r7`, branche `feat/omr-r7`.
Objectif : étendre la pyramide de relief fin (aujourd'hui Ouest seulement, cadre 16 × 16 décalé
`root_origin_tiles` [0, 5]) à tout le monde OM (28 × 24 tuiles racines), pour qu'un zoom sur
Constantinople, Moscou, Le Caire ou Tunis ait un relief Copernicus comme en France.

## Contraintes
- Disque : 96 Go libres au départ ; ne jamais descendre sous 25 Go (`df -h /` avant chaque bloc).
- Pas de dépense (Copernicus GLO-90, bucket public AWS anonyme). Pas de release GitHub.
- Pyramide du checkout principal intouchée : la nouvelle est produite dans
  `data/map/pyramid` du worktree (liens durs pour les tuiles reprises : 0 octet de plus).

## Plan
1. Squelette + note (ce fichier).
2. Mesure des volumes bruts et cuits attendus avant tout téléchargement.
3. Cadre monde : `root_origin_tiles` [0, 0], cadre rectangulaire 28 × 24 dans l'outillage ;
   tuiles existantes renommées (ligne + 5·2^k), CAFV fleuves/routes décalés (+20 lignes E2,
   +1280 unités y).
4. Téléchargement + cuisson E1-E2 par blocs (télécharger, cuire, supprimer le brut), reprise.
5. Paquet : version incrémentée (ADR 0077), rien publié.
6. Vérifs : `FineGeoStore`, zg5b, test sur le Bosphore, `smoke.gd`.

## État
- [x] 1 squelette

## Prochaine étape
Mesurer les volumes (étape 2).
