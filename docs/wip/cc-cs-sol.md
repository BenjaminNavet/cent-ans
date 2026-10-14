# CC CS — sol de près (branche `cc/cs`)

Lot du 14/10 après les captures jeu3 (Alpes/Pyrénées validées). Points du joueur :
1. sol de près « tapis moucheté » (Paris, caméra 400) ;
2. `geo splat` (14 Go) et `geo colormap` (12 Go) par tuiles, < 6 Go ;
3. Apennin presque plat ;
4. taches vert vif en Calabre ;
5. nuées de beau temps encore visibles à moyenne distance.
Ajouts du coordinateur (capture `jeu3/est_sud.png`) : A bande verte vive à l'EST ; B bord de carte sur le
vide noir ; C Carpates « fondues », Anatolie (blobs étirés, rectangle près de Konya) ; D Égypte sans Nil ;
E tavelures sombres partout (steppes, Russie).

## Diagnostics (sans Godot : planches 2D `~/dev/cent-ans-raw/compose/cs/`, `ground_board.py`)
- Paris : la carte de couleur est un confetti de blocs Voronoï de 1,8 km (5 texels) aux cultures très
  contrastées (or / brun labour / vert pré) tirées au hasard bloc par bloc ; le splat est quasi pur
  prairie (la forêt n'y est que 3 taches). Le « tapis moucheté » vient de la mosaïque de la carte de
  couleur, pas du splat. Cause secondaire : forêt binaire noire (massifs sombres, `forest_masses`).
- A : 8,7 % des terres ont `province_ids == 0` (marges est et sud) ; le shader ne voilait que `id > 0`
  (brouillard) -> vert vif non voilé, ceinturé d'une frontière de royaume. Corrigé dans le shader.
- B : le bord de carte donne sur le fond d'abîme du plan d'eau (bleu nuit). Anneau de brume claire
  (`sea.gd`) + fondu des 14 derniers px de terre vers la brume (`terrain.gdshader`) : à juger en jeu.
- 4 (Calabre) : prairie du splat à 100 % sur un sud sec dont la carte de couleur est paille/garrigue ;
  le shader peint l'herbe vert vif. Corrigé à la source : `grass_to_heath` (landcover_params.json).
- 3 (Apennin) : voir § Italie.
- D (Nil) : voir § Égypte.

## Mémoire (`/usr/bin/time -l`, machine 51 Go, mesures par le pic RSS)
- `geo splat` : avant ~ 14 Go. Distances côte / frontières par bandes avec halo (`coast_dist_banded`,
  `border_dist_banded`), EDT plafonnée par bandes (`splat.edt_capped`), gaussiennes et pentes par
  bandes (`gaussian_banded`, `slope_banded`), lon/lat par bandes, `compute_splat` canal par canal,
  composition + encodage du splat par bandes. Sorties identiques (tests d'égalité sur grille synthétique
  et sur recadrage réel ; PNG du splat identique entre deux variantes du code).
- `geo colormap` : cuisson par tuiles (`bake` : `global_fields` une fois, `tile_inputs` par bande),
  indépendante de la hauteur de bande (test `test_tiled_bake_equals_single_band`).

## État / prochaine étape
Voir les commits `wip:` ; le rapport final liste les mesures.

## État / prochaine étape (pause 14/10)

**Fait (branche `cc/cs`, rien de poussé)**
- Item 1 sol de près : colormap en mosaïque groupée (blocs 2,2 km, cultures groupées `cluster`, soles, grain), planches `~/dev/cent-ans-raw/compose/cs/v3_*.png` (Paris plus calme, un peu de brun foncé restant).
- Item 2 cuissons par bandes (ADR 0358) : `geo splat` (splat+landcover) pic 5,7 Go, `geo colormap` pic 5,77 Go (BAND_ROWS 256 ; sortie octet-identique à 1024), `geo navgrid` 4,1 Go. Avant : 14 et 12 Go.
- Item 3 Apennin : cause = crête d'auteur chain_05 mince (9 px) à côté d'un réel modéré (1500-2900 m, ~40 % des Alpes). Paramètres de crête (`crest_band_px`, `crest_fade_px`, `crest_floor_cap_m`) ajoutés mais le relief n'a PAS été recalculé (compose-relief-real écrase la pyramide partagée, pic 8,4 Go) ; l'aperçu `apres_italie_ombre.png` montre un effet faible : il faut soit relever le réel (chaîne de crête réelle), soit un massif plus large. Reste à décider.
- Item 4 vert Calabre : grass_to_heath dans landcover (biomes secs) ; parcelles shader peuvent rester vertes, à vérifier en jeu.
- Item 5 nuages : cumulus 0,15, fondu 640-1300.
- A bande verte est : brouillard appliqué aux id 0 terrestres (shader). B bord noir : brume de bord (sea.gd, shader), non vérifié en jeu. D Nil/delta/Euphrate/Tigre : `data/map/valley_courses.json` + bande « oasis » dans le colormap (planche v2_caire).
- Données régénérées commitées : splat, wetlands, forest_kind, border_dist, colormap bc1 + aperçu, navgrid.

**Restes**
- C : traité (voir § Relief Apennin / C ci-dessous).
- E (moucheture sombre à l'est) : seul le brouillard id 0 corrigé ; à vérifier en jeu (`forest_masses` plancher 0,12).
- Vérifs en jeu (session principale) : Paris cam 400, Apennin, Calabre, bord est, bord sud Égypte, nuages.
- Tests : pytest 2800 ok sauf `test_pyramid::test_e2_mean_is_e1` (lit la pyramide partagée, non touchée par ce lot). Smoke Godot : 17 échecs car GDExtension absente du worktree (`core/build.sh` à lancer) ; erreurs shader/sea.gd non confirmées faute de cela.
- pytest complet : pic 19 Go (existant, ne pas lancer en parallèle).

**Rejouer** : `cd tools && .venv/bin/cent-ans geo splat && .venv/bin/cent-ans geo colormap && .venv/bin/cent-ans geo navgrid` ; planches : `python -m cent_ans_tools.compose.ground_board compare <tag> paris apennin calabre konya moscou caire`.

## Relief : Apennin et item C (14/10, soir) — ADR 0359
**Causes**
- Apennin plat : mur de crête d'auteur (9 px) + réel modéré. Carpates « fondues » : MÊME cause, le plancher de
  crête `chain_09` est un « tube » lisse décalé de l'arc réel (planche `carpates_avant_apres.png`).
- Raccord « rectangulaire » près de Konya : PAS la couverture E2/E3 (la pyramide réelle E3/E4 n'existe que sur
  le cœur France/Angleterre, la composée E2/E3 aussi ; Konya n'a que E1, E1 sans raccord de tuile mesuré :
  `seam.py`) mais les lacs de plateau (Tuz Gölü, Beyşehir) posés au niveau de lac d'AUTEUR (56 m) au fond de
  cuvettes plates de 850 m. Corrigé : niveau = rive réelle.
- Étirement : à l'est le MNT est GLO-90 (90 m) ; la déformation agrandit jusqu'à x1,75 (Konya : 1,1 / 0,57 px réels
  par px composé) : flou inhérent à la donnée. Le GLO-30 de l'Anatolie / des Carpates (~ 120 tuiles de 1°,
  > 2 Go) n'a PAS été téléchargé.
**Corrections** : `relief_real.json` > `chains` (gain de relief réel par chaîne, chain_05 Apennin g1,8, chain_09
Carpates g1,2, `crest: false`), lacs à l'altitude réelle, `min_real_coverage` 0,95.
**Cuissons** rejouées : relief, splat (5,7 Go), colormap (5,8 Go), navgrid (4,1 Go). Navgrid : 1 477 010 -> 1 476 618
cases terrestres impassables (masque d'auteur inchangé) ; Rome, Naples, Florence, Bologne, Milan, Pise, Lecce,
Reggio dans la même composante qu'avant ; l'Italie n'est pas fermée.
**Mémoire de `compose-relief-real`** : 8,4 -> 4,6 Go max par processus (phase 1 dans un processus à part, ~ 4,3 Go ;
phase 2 : principal ~ 1 Go + 2 x 1,5 Go ; écriture des fichiers de carte 4,1 Go). Cadre complet évité
(`derived/inside.npy`), contraintes en place, écriture atomique (`pyramid.new` / `height.new` puis renommage,
la pyramide partagée n'est plus effacée d'abord). `--bbox x0,y0,x1,y1` : recalcul d'une région en ~ 20 s
(`--reuse-base`), sortie `~/dev/cent-ans-raw/compose/cd/pyramid_region` ; tuiles identiques au calcul complet
(150 tuiles Italie comparées octet à octet).
**Planches** : `~/dev/cent-ans-raw/compose/cs/` `italie_cmp_v2.png`, `carpates_avant_apres.png`,
`final_{italie,carpates,konya}_E1.png`, `diag_*`, `real_cov_E*.png`, `comp_cov_E*.png`.
**À juger en jeu** : largeur / hauteur de l'Apennin (réglages `chains.chain_05` ; l'exagération RE s'ajoute),
Carpates sans tube (mais murées par le masque d'auteur), lacs de plateau à l'altitude réelle (eau plate haute),
flou résiduel de l'est (GLO-90).
