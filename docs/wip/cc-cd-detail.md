# CC CD — détail rapproché réaliste

Branche `cc/cd` (worktree `game_project-cccd`), partie de `cc-carte-composee`. ADR 0355 (addendum du 14/10).

## Cap actuel (changement de cap du 14/10)
Le relief procédural des massifs est abandonné comme source de forme : **MNT réel déformé + contraintes d'auteur**.

### Commande `cent-ans geo compose-relief-real` (`compose/relief_real.py`, réglages `compose/relief_real.json`)
1. Passe 1 : relief réel échantillonné à la résolution de la carte par la déformation inverse (`warp.json`) dans le
   cache de relief réel (pyramide réelle E0-E4 : `~/dev/game_project/data/map`, ou `$CENT_ANS_REAL_RELIEF` ; sources
   gratuites, re-téléchargeables par `geo pyramid`). Étage composé L lit l'étage réel L + 1, bicubique.
2. Contraintes d'auteur en champs ADDITIFS lissés (le détail réel reste par-dessus) : plancher des crêtes `massif`
   (0,85 x sommet réel voisin ou 0,8 x hauteur d'auteur plafonnée à 1 700 m, bande 3 px + fondu 4), 27 cols en encoches
   (profondeur 0,2), plateformes sous les lieux, rampe côtière (côte composée != côte réelle), lacs plats d'auteur.
3. Passe 2 par tuile racine (1024 x 1024 px E1) : écrit E1, E0 (`height/`, tracké) et E2-E3 (tuiles E1 à relief >= 180 m
   et couverture réelle >= 50 %), puis `heightmap.png`, `heightmap_render.png`, ombrage BC5 par bandes, `map.json`.
4. `compose/author_mass.png` (tracké, 41 Ko) : masses de la règle `massif_impassable` appliquée au relief d'auteur,
   réunies (OU) à la règle sur le relief réel par `geo navgrid` (`navgrid.authored_mass`). Raison : le réel est plus
   étroit et plus bas (Carpates : 0,9-1,6 km au lieu de 3 km d'auteur) ; l'enveloppe relevée a été essayée et abandonnée
   (elle relève aussi le « bas pays » de la règle). Part non franchissable des terres : 18,9 % -> 20,3 %.
- Hauteurs VRAIES : l'exagération RE (ADR 0353) reste celle du profil `relief_exaggeration.tres` à l'exécution.
- Mémoire : `/usr/bin/time -l` plus sonde de RSS des processus fils : **pic total 6,2 Go** (parent 4,4 Go à l'écriture
  de l'ombrage, 2 workers de ~1 Go ; 2 workers par défaut). Cause du plantage de la machine : bords du cartogramme (NaN et
  fenêtres très comprimées) -> boîte englobante de plusieurs Go de mosaïque ; corrigé (repli sur la moyenne, étage réel
  plus grossier au-delà de 16 Mpx). Durée ~ 11 min (passe 1 : 8 min une fois, mise en cache `real_base.npy`, `--reuse-base`).
- Pyramide fine : `data/map/pyramid/` (hors git, ~ 1,9 Go : E1 2 156, E2 1 140, E3 4 355 tuiles), à déplacer dans
  `~/dev/cent-ans-raw/compose/pyramid` + lien symbolique. Le jeu la charge si `pyramid/relief_pyramid.json` existe
  (`terrain_builder.gd`, `relief_pyramid.gd` : `dir` ".", `e0_dir`), sinon détail procédural C5 : aucune erreur.

### Aval rejoué (une commande lourde à la fois)
`geo navgrid` (27 cols ouverts), `geo splat` (forêt 27,1 % des terres, pic 14 Go : outil existant), `geo colormap`
(pic 12 Go : outil existant). Non rejoués : `roads --computed`, `horizon`, `compose-layers` (occlusion) — à rejouer en C7 si besoin.

## Forêt et occupation du sol (consigne du 14/10)
`geo/landcover.py` + `data/map/landcover_params.json` : `forest_share` 0,30 (calibrage du potentiel, rendu 27 %),
`plain_open` 0,9 (grandes plaines agricoles dégagées : plat, bas, large), `noise_mix` (3 échelles : ~250, ~100, ~40 px),
lisière plus douce (σ 1,5 px). Aperçu : `~/dev/cent-ans-raw/compose/cd/colormap_apercu_900.jpg`.

## Neige, nuées
- `data/ui/campaign_seasons.json` `snow.line_m` [2000, 2900, 2300, 650] (printemps, été, automne, hiver) et
  `snow.mountain_line_m` 2900 : lus par `SeasonLook.snow_lines()` / `campaign_life.gd`. Cohérent avec des hauteurs vraies.
  Le shader abaisse déjà la neige sur les pentes raides (`1 - smoothstep(0.55, 0.9, slope)`) : roche et éboulis dessous.
- `data/fx/map_atmosphere.json` `cumulus_cards` : apparition à 480-720 (avant 260-520), sortie 900-1200, opacité 0,28.

## Pixelisation / grille (à confirmer à l'oeil par le coordinateur)
Cause principale identifiée : sans pyramide fine, la surface vient du raster 719 m/px (facettes triangulaires visibles
sur la neige) ; la pyramide fine réelle E1-E3 la supprime de près. Autres : forêt binaire monoéchelle (corrigé), splat à
demi-grille 1,4 km (lisière lissée), colormap 719 m/px étiré : non modifiés (pas de shader modifié, aucune vérification
visuelle possible sans Godot fenêtré).

## Érosion simulée (`geo compose-erode`, `compose/erode.py`)
Conservée comme complément éventuel (zones étirées) : érosion fluviale implicite + thermique (numba, ~ 27 min pour la
carte entière à 6 workers : mémoire à revoir avant tout rejeu) ; `compose/derived/erosion_masks.png` (roche, éboulis,
neige) lu par `masks.load_all`. Non utilisée par la chaîne principale.

## État des tests
`tools/tests/test_compose_relief_real.py` (3), `test_compose_erode.py` (6), `game/tests/cd_pyramid_test.gd`.
`test_compose_relief.py` : 6 tests de propriétés du relief procédural réécrits pour le relief réel.
`smoke_map.gd` : bloque sans fin dans mon worktree (même sans pyramide ; pas de bibliothèque native construite ici).

## Points ouverts
- Contrôle visuel en jeu (coordinateur) : profil RE sur hauteurs vraies, masses d'auteur vs relief réel, forêt, neige.
- E2-E3 limités aux zones à couverture Copernicus 30 m (Alpes, Pyrénées...) ; ailleurs E1 (180 m) bicubique.
- Rejouer `roads --computed`, `horizon`, `compose-layers` (occlusion).
