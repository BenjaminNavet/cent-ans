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
