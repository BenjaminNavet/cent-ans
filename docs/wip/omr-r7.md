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

## Mesures avant téléchargement (29/09, `measure.py` hors dépôt)
Terres du monde OM (E0 ≥ 0,25 m) en cadre monde 28 × 24 :
- E1 : 1 968 tuiles (313 existantes à l'Ouest, 1 655 nouvelles) ; E2 : 7 294 (1 069 + 6 226).
  Le masque des provinces ne retire presque rien (≈ 6 %) : on cuit toutes les terres.
- Cuit attendu : ≈ 290-300 ko par tuile (moyenne Ouest) → ≈ 0,5 Go E1 + 1,8 Go E2 nouveaux,
  soit ≈ 2,3 Go de plus (cache total ≈ 5 Go). Déserts plats : sans doute moins.
- Brut GLO-90 : 2 213 tuiles 1° utiles, 311 déjà en cache (Ouest), **1 902 à télécharger**,
  ≈ 3 Mo chacune → ≈ 5,5-6 Go au total, mais en flux (blocs de 4 × 4 tuiles E1, brut supprimé
  après cuisson) : pic de quelques centaines de Mo.
- Travail (`tools/geo/raw/pyramid_work/` du worktree) : e0/base (705 Mo chacun) + coast (176 Mo).
- Pas de GLO-30 à l'Est (E3-E4 restent sur le cœur de l'Ouest, comme la consigne le demande).

## Décision (ADR 0121 à écrire)
Pyramide dans le cadre monde (`root_origin_tiles` [0, 0]) : tuiles E1-E7 existantes renommées
(ligne + 5·2^k), CAFV décalés ; palier 1 (E1-E2) recuit en entier sur le monde (version de
cuisson tier1 3) ; tier2/tier3 inchangés (renommés seulement). `geo relief-reframe` et
`relief-all` recadrent sur place un cache d'avant R7 (`pyramid/frame.json` absent).

## État (fini, 29/09)
- [x] 1 squelette, 2 mesures, 3 cadre monde (ADR 0121, `world_frame.py`, `geo relief-reframe`,
  outillage à cadre rectangulaire).
- [x] 4 E1-E2 sur toutes les terres : 146 blocs, 1 902 tuiles GLO-90 (6,66 Go) téléchargées puis
  supprimées, 9 262 tuiles (2,68 Go) en 12 min 35 s ; cache 4,98 Go ; 83 Gio libres au plus bas.
- [x] 5 paquet v2 (`relief_hosting.json`), rien publié.
- [x] aval : hydro-fine (4 446 tuiles), anchors-fine (3 545 tuiles de routes), towns (2 137).
- [x] 6 tests : pytest 1 254 ok ; Godot omr_r7_east_relief (Bosphore −40 m / Beykoz 349 m), zg5b,
  om1, zg2, zg4, zg6, zg7b, zg7c, rs_k, pb3g OK ; smoke.gd OK. Capture `docs/img/omr-r7/bosphore.png`.

## Intégration (à faire par l'orchestrateur)
- Échanger le cache : remplacer `data/map/pyramid` du checkout principal par celui du worktree
  (E3-E7 y sont des liens durs vers l'ancien : `mv` du worktree après suppression de l'ancien,
  ou `geo relief-reframe` puis `geo pyramid --levels 1,2` dans le principal).
- `tools/geo/raw/pyramid_work/{e0,base,coast}.npy` du principal sont dans l'ancien cadre 8192² :
  les supprimer (ou y déplacer ceux du worktree, cadre monde, 1,6 Go) avant toute cuisson.
- Le cache `raw/hydro/cache/links_naturalearth.npz` du principal date d'avant OM2 : le supprimer
  (celui du worktree est à jour). Caches `snap/` et `roads/` locaux au worktree.
- Conflits possibles : `data/map/fine_anchors.json`, `towns_1340.json` si un autre lot les touche.

## Points ouverts
- Pas de GLO-30 (E3-E4) à l'Est : zoom minimal ≈ 5 unités à Constantinople contre 1,5 en France.
- E0 de l'Est reste ETOPO (versionné) ; seul E1-E2 apporte Copernicus.
- Erreurs « Lambda capture … freed » dans smoke.gd (préexistantes a priori, non liées).
