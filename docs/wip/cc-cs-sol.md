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
