# CC — carte composée façon Total War

Spécification : `docs/design/2026-10-14-carte-composee.md` ; ADR 0355.
Choix du joueur (14/10) : réinterprétation (C), Europe + Méditerranée, ≈ 400 lieux, géographie
reconnaissable mais caricaturée ; batailles au relief réel ; 0 $.

## État
- 14/10 : inventaires faits (chaîne de fabrication, références aux identifiants), spec + ADR.
- C0a fait (`docs/wip/cc-c0a-selection.md`) : périmètre `realm`, 208 provinces, 400 lieux.
- C0b fait (`docs/wip/cc-c0b-cartogramme.md`) : cartogramme par diffusion + planches.
  Itération v2 : sigma 200 km, densité de la mer 1,0 (au lieu de 150 km / 0,6) : la Méditerranée,
  l'Adriatique et l'Égée ne sont plus écrasées, l'Italie redevient une botte. Strength 1,0 écarté
  (Italie et Ibérie méconnaissables). Planches : `~/dev/cent-ans-raw/compose/v2/`.

- **14/10 : C0 validé par le joueur : strength 0,5** (sigma 200 km, mer 1,0), périmètre `realm`.

## Contrat commun (fixé le 14/10, partagé par C1/C3/C5)
- Canevas composé = emprise recadrée `CROP_LONLAT` (−11..45 E, 28..66 N) de l'ancienne grille,
  pleine résolution (≈ 7080 × 6104 px), `meters_per_px` **nominal** 719 (réglable en C6).
- `data/map/map.json` : `"crs": "composed"` + bloc `composed` (chemin de `warp.json`, taille,
  `meters_per_px` nominal). L'ancienne grille EPSG:3035 reste décrite (`real_grid`) : elle sert à
  lire les rasters réels (relief de bataille, climat, couverture du sol).
- `data/map/compose/warp.json` (versionné) : paramètres (strength 0,5, sigma 200, mer 1,0),
  `real_grid` (origine, m/px de l'ancienne carte), `crop_box_px`, `factor` 8, `size_px`, et deux
  grilles binaires float32 little-endian, forme (ny, nx, 2), ligne 0 = nord :
  - `warp_forward.f32` : pour chaque nœud de la grille réelle réduite (px réels / 8, dans le
    recadrage), la position composée (x, y) en px pleine résolution ;
  - `warp_inverse.f32` : pour chaque nœud de la grille composée réduite (px composés / 8), la
    position réelle (x, y) en px pleine résolution de l'ancienne grille.
  Interpolation bilinéaire ; hors emprise → NaN / `None`.
- Projection : lon/lat → px réels (EPSG:3035, `MapGrid` réel) → `forward` → px composés. Une seule
  implémentation par langage : Python `MapGrid` (geo/project.py), Rust `data-model/src/map.rs`,
  GDScript `MapProjection` (nouveau, `game/scripts/map/map_projection.gd`).
- Chaque province et lieu garde son `lonlat` **réel** (règles : météo, peste, sécheresse, batailles)
  ; ses px composés sont précuits par la chaîne (C4). Les règles ne projettent pas à l'exécution.
- Identifiants : `data/map/compose/id_map.json` (C0a) fait foi.

## Vagues
- V1 (parallèle) : C1 compilateur (`cc/c1`), C3 données (`cc/c3`), C5 exécution (`cc/c5`).
- V2 : C2 relief (après C1). V3 : C4 couches dérivées (après C1-C3). V4 : C6 tests/équilibre.
  Puis C7 avec le joueur.
- **CE — éditeur de carte** (ajouté le 14/10 à la demande du joueur, après C2) : page web locale
  sur la planche 2D ; crêtes, cols, épingles, lieux déplaçables à la souris ; réécrit les JSON
  d'auteur et redessine la planche en quelques secondes ; pinceau de masques (forêt, roche,
  champs, marais) qui alimente les couches peintes ; bouton « recompiler » pour la chaîne.
- **CP — carte peinte** (décision du joueur, 14/10) : en DERNIER, une fois la géographie stabilisée
  par l'éditeur ; peindre de préférence des masques plutôt qu'une image finale figée.

- 14/10 : C1 (a058421b9), C5 et C2 (révisé, a23ced7f8) fusionnés dans `cc-carte-composee`.
  Restes C2 notés pour C4/C7/CE : chaînes étroites et de largeur uniforme (TW : plus larges),
  Massif central en ligne au lieu d'un plateau, cols en bandes rectilignes, quelques collines en
  vers (Ibérie, Anatolie). **Décision : Carpates murées** (≈ 2 500 m + cols des Carpates dans
  `crossings.json`) en C4, cohérent avec « infranchissable hors des cols ».

## Prochaine étape
- C3 en cours ; C5 vérifie son chargeur sur le vrai warp ; puis C4 ; CE (éditeur) en parallèle.

## Points ouverts
- Taille nominale de la nouvelle carte (px, `meters_per_px`) : à fixer en C0 selon la densité visée.

## C2b — relief des massifs (retour joueur 14/10, après C4a, avant C4b)
Planche Alpes C2 : cols et vallées creusés en bandes plates étroites jusqu'au niveau de la plaine,
qui découpent la chaîne en blocs. À corriger :
- cols : une encoche à la crête (≈ 50-60 % de la hauteur), approche en pente, plus de couloir plat ;
  le franchissement reste garanti par `crossings.json` (navgrid), pas par la géométrie ;
- vallées : seules quelques grandes vallées en auge U à fond montant (Rhône valaisan, Inn, Adige,
  Isère) ; les autres fleuves ne creusent plus les massifs ;
- chaînes plus larges et de largeur variable, Massif central en plateau (restes C2).
Planche Alpes avant/après à montrer au joueur.

## Retour du joueur sur les premières captures en jeu (14/10)
« La carte donne l'impression d'être basse résolution, un peu jouet, peu de textures. » Captures
`~/dev/cent-ans-raw/compose/jeu/` (état C4b jalon 1, relief d'avant C2b). Causes identifiées :
1. Pyramide de relief fin réel (E1-E4) coupée par l'ADR 0355 et pas encore remplacée : de près,
   seul le raster 719 m/px procédural lisse reste → aspect « pâte ».
2. Splat, colormap, routes, champs, habitat encore provisoires (jalon 2 de C4b).
3. Textures régionales TX non revérifiées sur la carte composée.

Lot ajouté **CD — détail rapproché réaliste** (après C4b, avant C6, 0 $) :
- érosion hydraulique/thermique simulée hors ligne sur le relief composé (ravines, cônes,
  talwegs) ;
- pyramide fine régénérée depuis ce relief érodé (même format E1-E3) ;
- splat roche/éboulis/neige piloté par la pente et l'érosion ;
- textures TX branchées.
Porte : capture comparée main ↔ cc au même cadrage, seulement avec l'accord du joueur (le
lancement fenêtré peut lui prendre le focus).

## État 14/10 (après-midi)
- C2b fusionné (a85373338) ; C4b fusionné (ffd0f79a7) : jeu complet sur la carte composée, smoke OK,
  cargo 1871 ok / 26 ignorés « C6 balance », pytest 2789 ok.
- En cours : CD (worktree cccd, érosion + pyramide fine), C6 (worktree cc6, échelle, base_supply,
  textes, IA, seed bots).
- Ensuite : C7 regard du joueur (captures fenêtrées seulement avec son accord), puis CP.

## Retour du joueur sur les captures jeu2 (14/10, 12 h 40)
« C'est sur la bonne voie, mais les montagnes ne sont vraiment pas réalistes et on voit les
grilles, c'est très pixelisé. » Décision : le relief des massifs devient le **MNT réel déformé**
par le cartogramme (ETOPO pour E0, GLO-90 pour la pyramide E1-E2), avec exagération RE. Les
crêtes, cols et masques d'auteur de C2b deviennent des contraintes (mur continu, encoches,
infranchissable). Confié au lot CD, avec la chasse à la pixelisation et à la grille, la forêt
réaliste et la limite de neige. Addendum à l'ADR 0355 à écrire par CD.

## Reprise (pause du 14/10, fin d'après-midi)

État : branche `cc-carte-composee` (worktree `../game_project-cc`) contient C0-C6, C2b, C3b, C4b, CD (relief réel
déformé, ADR 0355 addendum), CE, passe 2 des textes (6d4d385ae + suivants). Smoke OK dans `../game_project-ccshot`.
Lot en cours : **CS sol de près**, branche `cc/cs`, worktree `../game_project-cccs`, note `docs/wip/cc-cs-sol.md`
(points 1-5 + A-E, mémoire par bandes ADR 0358). Pyramide fine composée hors git :
`~/dev/cent-ans-raw/compose/pyramid` (lien `data/map/pyramid` dans chaque worktree qui lance le jeu).

Mandat du joueur : autonomie complète sur CC, captures fenêtrées autorisées (`godot_bg.sh` ou la commande
ci-dessous), 0 $, **publication autorisée** après CS : fusion dans main **et push**. CP (carte peinte) en dernier.
Mémoire : pic < 6 Go par commande, une commande lourde à la fois, cargo `-j 4`.

Étapes :
1. Lire `docs/wip/cc-cs-sol.md` et `git log cc-carte-composee..cc/cs` ; finir les points restants de CS.
2. Fusionner `cc/cs` dans `cc-carte-composee` (worktree cc, `set -o pipefail`), supprimer le worktree cccs.
3. `../game_project-ccshot` : `git checkout --detach cc-carte-composee`, `core/build.sh`, import headless, smoke.
4. Captures en jeu (10 sites), dans ccshot :
   `app=$(readlink -f "$(command -v godot)"); app=${app%%/Contents/*}; open -g -j -n -W -a "$app" --args --path "$PWD/game" --audio-driver Dummy --resolution 800x500 --position 4000,4000 --script res://tests/re_relief_shot.gd -- --out=<png> --distances=400,1100 "--points=<lon,lat;…>"`
   Ouest : `2.35,48.85;7.4,46.2;0.6,42.75;12.5,41.9` ; Est/Sud : `24.5,47.0;37.6,55.7;29.0,41.0;32.5,38.5;31.2,30.0;-6.0,32.5`.
   Références avant CS : `~/dev/cent-ans-raw/compose/jeu3/{planche,est_sud}.png`.
5. Tests complets (`cargo test -j 4`, pytest), seed bots passe longue (seule) ; constat ouvert : faillites en baisse
   (`seed-bots-findings.md`), à trancher avant la fusion.
6. Fusion dans main (worktree dédié, `--ff-only` si possible), push, puis CP.

État CS à la pause (rapport de l'agent, `cc/cs` 0e7786d40, tout commité) : faits sans contrôle en jeu 1, 2 (ADR 0358,
splat 5,7 Go, colormap 5,77 Go), 4, 5, A, B, D (`valley_courses.json`). Non résolu : 3 Apennin (crête d'auteur `chain_05`
mur de 9 px, relief non recalculé : `compose-relief-real` écrase la pyramide partagée et monte à 8,4 Go → à revoir),
C (Carpates étirées, raccord près de Konya, couverture E2/E3 ?), E (moucheté à l'est). Avant tout : `core/build.sh` dans
cccs puis smoke (17 échecs vus sans GDExtension construite) ; `test_pyramid::test_e2_mean_is_e1` échoue (pyramide
partagée). pytest complet : pic 19 Go, toujours seul.

## CS terminé (14/10, soir) — publication

CS (sol de près) clos : `cc/cs` (relief par chaîne, ADR 0359) et `cc/cs2` (Nil/Euphrate, bord de carte, moucheté
de Paris = nœud `Vegetation`, ADR 0360) fusionnés ; main fusionné dans cc (fab6709a6) ; cargo test, pytest complet
(2808 passés), smoke, passe longue seed bots (bandes faillites/guerre recalibrées) au vert. Détail : `cc-cs-sol.md`.
cc-carte-composee fusionné dans main et poussé.

À regarder en jeu par le joueur : Apennin (`relief_real.json > chains.chain_05`), Carpates, lacs de Konya, delta du
Nil (bords un peu géométriques), bosquets encore un peu sombres à Paris (`impostor_gain`), coin sombre au NE de
Moscou à cam 1100, largeur de la brume de bord (`edge_haze_px`).
Piège : un `geo compose` complet réécrit `rivers.geojson` sans les fleuves de vallée → rejouer `compose valley-rivers`.

**Prochaine étape : CP (carte peinte).**
