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

## CS2 (14/10 soir, branche `cc/cs2`) — captures jeu4 : tavelures, Nil, bord de carte, nuées — ADR 0360
**État : tout codé et cuit, en attente du contrôle en jeu (session principale).**

1. **E tavelures sombres** — deux causes distinctes.
   - Désert (Égypte) : points noirs = affleurements rocheux `faithful` de biomes secs (`hamada`, `gypsum_badland`,
     `sandstone_dry`, 200-500 m, albédo moyen 0,06-0,08 : noirs sur le sable vus de haut). Corrigé : `faithful: false`
     (le gain de couleur vise le sable) ; `min_pixels` 4 -> 7 (plus de moucheture à 4 px). `data/art/rock_outcrops.yaml`.
   - Plaines (Paris, Moscou) : taches brunes anguleuses = parcelles de labour / « labour mouillé » du parcellaire
     saisonnier (`season_field`, `campaign_life.gdshaderinc`) quasi noires (0,085 / 0,13 linéaire) sur une carte de
     couleur claire. Éclaircies (0,135 / 0,19). Le plancher `forest_masses` ne concerne que les grands massifs.
2. **Nil** — `compile.build_rivers` ne garde que 45 fleuves : le Nil, l'Euphrate n'avaient aucune ligne (seulement
   `valley_courses.json`, 3-4 points, pour la bande verte). Nouvelle commande `cent-ans compose valley-rivers`
   (`compose/valley_rivers.py`) : tracés lon/lat (Natural Earth 10 m pour le tronc, deltas d'auteur) -> cartogramme ->
   simplification + Chaikin -> bras calés sur la côte -> ajoutés à `rivers.geojson` (`source: "valley"`, rejouable).
   Le Nil (28° N -> Caire, 251 km) et ses branches de Rosette et de Damiette, l'Euphrate (440 km). Le Tigre n'est pas
   tracé (extrême est écrasé par le cartogramme, tracé aberrant). Largeurs : `river_styles.json`.
   Bande irriguée : `valley_courses.json > irrigated_areas` (delta, Fayoum, anneaux arrondis) lus par `colormap.py`,
   palette `oasis` `#4d7230` vert foncé, bande pleine ~ 5,6 km fondue jusqu'à `valley_green_km` 16.
   Cuissons rejouées dans l'ordre : `compose valley-rivers`, `geo compose-relief-real --reuse-base` (creuse le lit ; 8 min,
   le cache `derived/author_height.npy` perdu a été reconstruit : pic 7,8 Go UNE fois, correctif `relief_real.author_height`),
   `geo rivers-render`, `geo colormap` (pic 5,5 Go). Non rejoués : `geo splat` (landcover lit `rivers.geojson` : prairies
   humides le long du Nil, effet mineur), `geo navgrid` (hauteurs changées de quelques mètres dans les vallées).
3. **Bord de carte** — la cause du « fond gris » et de la dalle : l'anneau de brume de CS (4 plans gris à 0,6 au-dessus
   de l'eau) est lui-même un rectangle visible, plus sombre que l'horizon (0,62 contre 0,74) ; la terre coupée au bord
   montrait son flanc ; l'abîme sombre apparaissait au-delà du bord (eau transparente sur la terre clampée). Refait :
   plus d'anneau ; la mer s'étend à `margin_factor` 4 (9 cartes de large), `water.gdshader` et `terrain.gdshader` se
   fondent dans la couleur de l'horizon du ciel (0,74, 0,76, 0,76) sur `edge_haze_px` = 12 + 0,14 x distance caméra
   (posé par `CampaignWeatherView._update_edge_haze`), la terre descend au niveau de la mer dans cette bande (sommet) ;
   brume pleine et opaque au-delà du bord ; aucune brume sur le parchemin. Pas de limite de caméra ajoutée.
4. **Nuées** — `cumulus_cards` désactivé (`data/fx/map_atmosphere.json`) : les cartes de cumulus (apparition 640-960)
   étaient les blobs blancs à 1100 ; cirrus effacés avant 680. Ombres de cumulus du sol inchangées (pas blanches).

**À juger en jeu** : Égypte/Caire cam 400 et 1100 (Nil bleu + delta vert, plus de points noirs), Maroc -6,32.5 et est de
Moscou cam 1100 (plus de dalle ni de bande sombre, mer/terre fondues dans l'horizon), Paris/Moscou 400 (taches de labour
plus claires), nuées 400-1100.

### CS2 tour 2 (captures jeu5)
- **Taches anguleuses (Paris, Moscou, Carpates, désert)** : c'était bien le parcellaire `hb_apply` (`hb_ground.gdshaderinc`) :
  la couleur d'une parcelle (matière HB ou couleur de saison absolue) REMPLAÇAIT la carte de couleur (`mix(col, hb_col, w)`),
  avec `open_farm` 0,8 (80 % de parcelles même en désert : biome 8 `farm` 0,15 donnait déjà 12 %), cellules x4,2 (≈ 1,3 km en
  désert) et jitter de luminance ±28 %. Corrigé : `hb_relative()` borne la luminance de tout champ / matière à ±`lum_band`
  (0,15) de la couleur locale (carte de couleur) ; `cell_scale` 2,0, `cell_jitter` 0,07, `cell_hue` 0,12, `sole_share` 0,4 ;
  `farm` désert et toundra 0, steppe 0,2, semi-aride 0,4. Petites forêts : le plancher `forest_masses` passe de 0,12 à 0,17
  et la luminance des canopées est bornée à la carte. Pas de seconde couche de taches trouvée.
- **Nil** : bande verte vif-moyen (`oasis` `#5c9a30`), pleine 5,9 km, fondu jusqu'à 9 km (`valley_green_core` 0,65) ;
  delta = éventail triangulaire entre Rosette et Damiette, plus de Fayoum ni d'anneaux arrondis. Colormap recuit (5,5 Go).
- **Coin NE de Moscou** : mer dans la carte restant sombre jusqu'au bord net ; à vue de capture la rampe de brume (166 px à
  1100) est écrasée par la perspective rasante. `edge_haze_per_distance` 0,14 -> 0,3 (330 px à 1100) et largeur initiale de
  la mer 160 px. À revérifier.

### CS2 tour 3
- `game/tests/re_relief_shot.gd` : `--list` (noms des couches enfants de la carte), `--hide=Vegetation,GroundClutter,RockOutcrops,...`,
  `--ground-only`, `--terrain-off=hb,colormap,cloudshadow` (shader du sol) pour isoler la couche des taches de Paris cam 400.
- Rochers et arbres (impostors) passent par la brume de bord (global shader `campaign_edge`, `campaign_edge.gdshaderinc`) ;
  albédo des rochers plancher 0,09 (jamais noir).
- Rectangle blanc de Damiette : terre du masque sous le niveau de la mer (Manzala, -2 à -3 m) peinte « fond marin clair » par
  `terrain.gdshader` (`hc <= 0`) ; désormais seulement hors raster terre (`land_f < 0.5`).
