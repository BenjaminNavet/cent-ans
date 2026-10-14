# CV — convois vivants (caravanes, charrettes, bêtes de la carte)

ADR : 0354. Demande du joueur (14/10) : modèles TRELLIS anciens, pop-up (disparaissent / réapparaissent),
taille qui suit la distance (pas réaliste) → taille monde fixe et grande, plus aucun pop, modèles Meshy.
Go Meshy : 18 modèles en meshy-6-lite (≈ 270 crédits sur 930, remise 13/11).

## Diagnostic
- Deux systèmes : `CountrysideLayer` (DN, charrettes/caravanes/bouviers garés, glb TRELLIS 1,
  taille ∝ distance^`size_exponent` × `countryside_size`) et `FolkPool` (figurines + accessoires
  FK2 procéduraux Blender, taille × (d/25)^0,9 depuis VG2).
- Pop FolkPool : `place()` refait tout le semis quand le point visé bouge de 20 % du rayon, le zoom
  de 15 %, `spread` change de puissance de deux, un tour passe ; le plafond servi dans l'ordre des
  fournisseurs choisit d'autres instances ; pas de fondu au bord ni à la portée (clear()).

## Lots
- **CV-A Meshy** (worktree `../gp-cv-meshy`, branche `cv/meshy`) : 18 modèles 6-lite, ingestion DN,
  FolkModels pointe les accessoires vers les glb DN Meshy (fin des boîtes FK2).
  Liste : cart_merchant env_cart_ox_peasant wagon_four_wheel terrain_haywain_wagon cart_wool_bales
  cart_wine_barrel_wagon cart_carreta_iberian cart_arba_steppe env_cart_pack_train_mules
  env_merchant_caravan_camels herd_drover_cattle env_shepherd_hut_wheeled animal_cattle_red
  animal_ox_draught animal_sheep_wool + images neuves (Z-Image local) folk_plough folk_dead_cart
  folk_market_stall (+ animal_horse_draught si pas encore généré en Meshy).
- **CV-B taille fixe + anti-pop** (worktree `../gp-cv-stable`, branche `cv/stable`) : taille monde
  constante (FolkPool et CountrysideLayer), placement ancré monde, éclaircie stable par aléa
  d'instance + fondu tramé en shader, fondus de bord et de portée.

## État
- [x] diagnostic, go Meshy
- [x] CV-A (285 crédits)  - [x] CV-B  - [~] CV-A2 mouvement des bêtes  - [ ] attelage  - [ ] fusion main, captures (≤ 5), regard du joueur

## CV-A fait (14/10)
- 19 modèles Meshy 6-lite (vagues 14-15 de `meshy_waves.py`, 285 crédits, solde 930 -> 645), tous réussis, ingérés
  dans `game/assets/models/dn/` du checkout principal (gitignoré ; le worktree y pointe par lien) et
  `data/art/dn_manifest.json` (commité). Les 16 anciens objets gardent classe et taille du manifeste.
- `FolkModels.PROPS` : champs `dn`, `yaw_deg`, `slots` ; ordre DN -> glb FK2 -> boîtes. Bornes et emplacements
  calculés sur le maillage. Avant des glb : bêtes +Z, chariots à timon -X (`yaw_deg` 90), charrue et charrette à bras
  +X (-90), étal dont le comptoir regarde +Z (90, comptoir vers +X monde).
- Tests verts dans le worktree : smoke, fk_folk, fk2_assets, dn_pays, as1, as8c (les deux derniers sautent les masques
  de mouvement FK2 pour les glb DN).
- Restes : mouvement des bêtes et des roues (`animal_motion.json` est calé sur la géométrie FK2) éteint pour les glb DN ;
  charrettes sans bête (le glb DN de charrette n'inclut plus le boeuf FK2 : atteler un `ox`/`horse` est à faire en CV-B ou
  suite) ; charrue seule (le conducteur `drover` est posé 2 m devant) ; sens du timon de `env_cart_ox_peasant`,
  `wagon_four_wheel`, `cart_wool_bales` déduit du maillage, à confirmer à l'œil.

## CV-B (14/10, branche cv/stable)
Fait : taille fixe FolkPool + CountrysideLayer (`fixed_size`), semis ancré monde (aléa d'unité,
grille emboîtée), éclaircie stable en shader, fondus de zone et de portée, tests `fk_stable_test`.
Détails : ADR 0354 § Détails d'implémentation CV-B. Réglages : `town_maquettes.json` § props
(`folk_scale` 600, `folk_keep_*`, `folk_fade_band`, `folk_zone_margin`, `folk_edge_inner`, `folk_range_fade`).
À vérifier au regard (session principale) : lisibilité du tramé, densité en vue moyenne
(`folk_keep_from` 30), taille des charrettes ; compilation shader (testée en fenêtre, voir rapport).
Contrat pour CV-A : les accessoires de FolkModels doivent garder le shader `folk_prop.gdshader`
(ou inclure `folk_fade.gdshaderinc`) pour le fondu ; `fixed_size: true` sur les nouveaux ids du calque campagne.

## CV-A2 fait (14/10) : mouvement des bêtes et des roues DN
- `data/fx/animal_motion.json` : 8 entrées (`animal_cattle_red`, `animal_ox_draught`, `animal_sheep_wool`,
  `animal_horse_draught` = `base` FK2 pour les valeurs vidéo + géométrie mesurée ; `cart_merchant`, `env_cart_ox_peasant`,
  `folk_dead_cart`, `folk_plough` = roues seules, `half_m` 0). Mesures sur le maillage après `yaw_deg` (repère de
  `FolkModels.prop_mesh`, avance +Z, centre de la boîte au sol : l'ancre `anchor_m` n'est donc pas 0 pour les bêtes, tête comprise).
- Shader `animal_motion.gdshaderinc` : uniforme `am_axle` (Z de 1 ou 2 essieux) ; FK2 inchangé (un essieu à Z = 0). Schéma : `axle_z_m`.
- Test `cv_a2_test.gd` (pattes bougent / dos non, roues tournent autour de l'essieu, caisse fixe), rejeu CPU des masques.
- Approximations : x_min des roues de `env_cart_ox_peasant` (1,18) et `folk_dead_cart` (0,61) calé sur la caisse (roue partiellement
  tournante) ; queues des bêtes hors masque (croupe = bord du maillage) ; à juger à l'œil.

## Intégration (14/10, cv/stable)
- cv/meshy et CV-A2 fusionnés ; attelage `team` + `FolkPool.add_team` (a748f5f6a) ; tests verts : smoke, fk_folk,
  fk_stable, fk2_assets, dn_pays, as1, as8c, cv_a2.
- Captures (3/5, `game/tests/cv_views_shot.gd`) : proche = figurines grandes, charrette attelée lisible ; moyenne =
  densité clairsemée mais présente ; lointaine = plus de convois (portée). Aucun réglage changé.

## Retour du joueur (14/10)
- Chevaux qui disparaissaient puis réapparaissaient derrière la charrette : l'attelage décalait la phase, donc les bêtes
  bouclaient avant la charrette. Corrigé : même phase, segment avancé (`FolkPool.add(..., ahead_m)`), test dans
  `fk_stable_test` (fcb61518c).
- Visibles de plus haut : `folk_keep_from` 30 → 50, `folk_keep_min` 0,03 → 0,1, `pool_cap` 700 → 2000. Vue moyenne
  nettement plus peuplée. Puis `folk_scale` 600 → 900 (+50 %, accord du joueur) : un homme ≈ 2,2 unités, une charrette ≈ 4,4 ; points visibles à 150 du rig.

## Prochaine étape
Regard du joueur en jeu. Restes : sens du timon `wagon_four_wheel`, `cart_wool_bales` (parqués) et
`env_cart_ox_peasant` ; roues partiellement tournantes (ox_peasant, dead_cart) ; plafond dur ; paquet de modèles
non republié (accord du joueur).

- 14/10 soir (joueur : trop de caravanes et de personnages) : folk_scale 900 → 675 (−25 %), folk_density 2 → 1, convois de bord de route (map_countryside.json, règles `road` à modèles fixed_size) per_100km ÷ 2.
