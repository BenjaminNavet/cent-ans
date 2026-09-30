# CR1 — artefacts des gros plans de bataille

État : CR1 terminé (6 artefacts corrigés), grille avant/après `docs/img/cr/cr1_ab.jpg` (captures brutes `~/dev/cent-ans-raw/cr1/`).
Prochaine étape : jugement du joueur ; points ouverts plus bas. Références : `~/dev/cent-ans-raw/cav/c1_mounted.jpg`, `c2_close4.jpg`.
Prochaine étape : bissection par options `--no-*`, identification du code fautif pour les 6 artefacts
(rectangles saumon, fantômes blancs, cubes noirs, halos blancs, disques de lance, eau éblouissante).

| # | Artefact | Cause | Correctif | Commit |
|---|---|---|---|---|
| 1 | Rectangles saumon flottants à hauteur d'épaule | Rubans des ordres donnés (`BattlePathPreview`, 1,1 m, levés de 0,6 m, sans test de profondeur) et arcs de tir, jamais masqués en gros plan ni par `--no-hud` | Masqués sous 30 m de caméra (`ORDERS_NEAR_HIDE_M`, l'aperçu en direct reste) ; `--no-hud` cache aussi contours, trajets, arcs | 3dfbabb01 |
| 1b | Taches rouges au sol, caparaçons rougis | Décale de contour ennemi (boîte 30 m, `normal_fade` 0) projetée sur tout, figurines comprises | `cull_mask` = `BattleTerrain.DECAL_LAYER` (sol, rivière, ruisseaux, herbe) | 3dfbabb01 |
| 2 | Soldats « fantômes » blancs, bande blanche sur le pont | Même cause : trait or pâle des décales de contour/fantôme peint verticalement sur figurines et piles | Idem 1b | 3dfbabb01 |
| 3 | Cubes noirs en l'air | Mottes BV1 : dégradé carré 12-24 cm, découpe nette, noires à contre-jour, lancées à 6,5 m/s, aussi sur le tablier des ponts | Silhouette bosselée, 4-10 cm, rétroéclairage, fondu tramé, 2-4,5 m/s, jamais sur un pont | 3dfbabb01, 7abcf6bd4, 0c02393d0 |
| 4 | Halos blancs le long du parapet | Surtout des gerbes d'eau (disque blanc sans éclairage) émises pour les régiments sur le pont (emprise au-dessus de la rivière) ; en plus poussière en disque clair et fumée de camp proche | Pas de gerbes au-dessus de 2,5 m du lit (`_on_bridge`) ; poussière bruitée plus brune (alpha 0,32, fondu au contact et près de la caméra) ; fumée de camp alpha 0,3 et fondu sous 25 m (`near_fade` de `fire_smoke`) | 0c02393d0 |
| 5 | Grands disques blancs sur les lances | Blancheur : décale de contour (cf. 2). Taille : rondelle de 20 cm | Rondelle 14 cm ; recuisson cavalry_0,1,3,4,6 (+ atlas partagés) | 414a7eb3a, 0c02393d0 |
| 6 | Eau blanche éblouissante | `battle_water` : ciel compté deux fois (Fresnel maison dans EMISSION + spéculaire moteur à rugosité 0,04 sur vagues bosselées), ciel d'horizon clair jusqu'à 72 % | Spéculaire 0,45 → 0,25, rugosité 0,1, ciel reflété ×0,7 plafonné à 40 % | 414a7eb3a |

Tests : smoke, fg3_maps_test, sr2_weathering_test, cb_m1, cb_m2, cb_m4 verts.
Échecs préexistants (hors CR1) : `cb_m3_queue_test` (« last queued point where clicked », échoue aussi sur ddb7bd1cf),
`bv1_check` (« traits dans les pavois : 0/256 », `BattleVolleys`, non touché).

Points ouverts :
- Rebord blanc au pied des piles du pont (écume EP3 ou dessus des avant-becs) : non traité.
- Les mottes restent visibles en rondelles brunes à 2-3 m : acceptable, à juger.
- Eau : `EMISSION` ne suit pas la lumière du jour (au crépuscule la rivière reste claire) ; non traité.

# CR2 — acier crédible (plates, casques)

État : CR2 terminé (grille `docs/img/cr/cr2_cr3_ab.jpg`, captures brutes `~/dev/cent-ans-raw/cr2/`, `shot.sh`) ; jugement du joueur attendu.

Causes établies par captures de débogage (`d_mirror`, `d_rad`, `d_ry`) :
- Le seul environnement reflété est le ciel HDRI : son voile d'horizon, clair, couvre tout ce
  que reflète une pièce verticale (miroir forcé : acier bleu pâle partout, aucun sol sombre).
  Reflet coupé (RADIANCE noir) : la plate passe de 106 à 60 (sRGB) → le reflet fait la moitié
  de sa luminance.
- Rugosité SR2 0,45-0,75 : ce voile clair est moyenné en un gris uniforme sans structure
  (aucune ligne d'horizon, aucun nuage) = « plastique blanc ». Martelage trop faible (0,05-0,22).
Correctifs (`steel_occlusion`, 0 = rendu SR2) : occlusion spéculaire par `RADIANCE` (reflet
×0,12 sous l'horizon → ×1 vers 40° de hauteur, élargi par la rugosité, ×0,35 dans les creux
cuits) sur plate, garniture et maille ; rugosité 0,16-0,42 ; acier 0,42 ; film de crasse non
métallique dans les creux et par plaques ; martelage +0,3.
Maille : la lumière directe (que ni l'AO ni RADIANCE ne touchent) la blanchissait comme un
tricot à contre-jour (`d_mail` : diffus rouge vif une fois le reflet coupé) → albédo ×0,55
(ombre d'anneau à anneau), rugosité ≥ 0,5, reflet ×0,5.
Casques : bassinet à calotte ogivale (arcs de rayon 1,6, pointe à +7,5 cm tirée vers la nuque),
dos rentré vers la nuque (plus de paroi verticale « boîte de conserve ») dans les deux
constructeurs (`battle_fine_equipment.bassinet` du kit FG0, `battle_fine_gear.bassinet_shell`) ;
cavalry_0 variante 0 : bassinet à visière en museau de chien + camail au lieu du grand heaume
(visière omise au LOD2). Recuisson des 14 figurines à bassinet ou caparaçon.
Triangles LOD0/1/2 : cavalry_0 16 786 / 2 028 / 549 (avant 16 864 / 1 986 / 534), cavalry_3
17 414 / 2 055 / 508, standard_1 16 910 / 1 998 / 502 ; à pied inchangés (≤ 11 879).
Tests : smoke, fg3_maps_test, sr2_weathering_test, an1a_motion_test verts. 26 captures sur 30.
Points ouverts : casque vu de dos encore « obus » (la pointe tirée vers la nuque se lit de
profil, peu de dos) ; LOD2 de cavalry_0 +15 triangles (camail) ; armoiries à 128 px floues de
très près sur le caparaçon (atlas `battle_soldiers._build_arms_atlas`).

# CR3 — caparaçons en drap lourd

État : `battle_fine_cavalry.caparison` réécrit (le constructeur utilisé ; celui de `battle_fine_horse.py` est mort) :
plis irréguliers en tuyaux d'orgue qui se creusent vers l'ourlet (5 cm LOD0), évasement, ourlet
ondulé à 0,46 m ± 3 cm (jarrets), dents (`dagged`) pour cavalry_3, fentes aux jambes (panneau
avant/arrière porté à 45 % par le haut de la jambe), armoiries sur tout le drap (meubles ~25 cm)
au lieu d'un écu collé sur chaque flanc ; anneau redistribué (1/3 dos, 1/3 par flanc) : même
nombre de triangles. Balancement : déjà fait par AN1a (`sm_weight` part 1, vitesse du régiment).
État : terminé (recuit, échelle 1,5 m par unité d'écu : meubles ~22 cm, bordure = liseré du drap).
Prochaine étape : jugement du joueur.
