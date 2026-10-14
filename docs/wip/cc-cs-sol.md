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
- C (Carpates étirées, raccord rectangulaire près de Konya : probablement couverture E2/E3) non traité.
- E (moucheture sombre à l'est) : seul le brouillard id 0 corrigé ; à vérifier en jeu (`forest_masses` plancher 0,12).
- Vérifs en jeu (session principale) : Paris cam 400, Apennin, Calabre, bord est, bord sud Égypte, nuages.
- Tests : pytest 2800 ok sauf `test_pyramid::test_e2_mean_is_e1` (lit la pyramide partagée, non touchée par ce lot). Smoke Godot : 17 échecs car GDExtension absente du worktree (`core/build.sh` à lancer) ; erreurs shader/sea.gd non confirmées faute de cela.
- pytest complet : pic 19 Go (existant, ne pas lancer en parallèle).

**Rejouer** : `cd tools && .venv/bin/cent-ans geo splat && .venv/bin/cent-ans geo colormap && .venv/bin/cent-ans geo navgrid` ; planches : `python -m cent_ans_tools.compose.ground_board compare <tag> paris apennin calabre konya moscou caire`.
