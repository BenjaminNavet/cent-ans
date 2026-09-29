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

## Décision (ADR 0119 à écrire)
Pyramide dans le cadre monde (`root_origin_tiles` [0, 0]) : tuiles E1-E7 existantes renommées
(ligne + 5·2^k), CAFV décalés ; palier 1 (E1-E2) recuit en entier sur le monde (version de
cuisson tier1 3) ; tier2/tier3 inchangés (renommés seulement). `geo relief-reframe` et
`relief-all` recadrent sur place un cache d'avant R7 (`pyramid/frame.json` absent).

## État
- [x] 1 squelette
- [x] 2 mesures
- [x] 3 cadre monde : outillage généralisé (pyramid, fine_relief, hydro_fine, fine_anchors,
  detail_dem, landmarks_v2, relief_cache), `world_frame.py`, `geo relief-reframe` ; cache du
  worktree recadré (liens durs vers celui du principal : 0 octet), manifestes décalés.
- [ ] 4 cuisson E1-E2 monde en flux

## Prochaine étape
Lancer `geo pyramid --levels 1,2` dans le worktree (arrière-plan), surveiller `df`.
