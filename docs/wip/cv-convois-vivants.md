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
- [ ] CV-A  - [x] CV-B (code + tests, à regarder)  - [ ] fusion, tests, captures (≤ 5), regard du joueur

## Prochaine étape
Lancer CV-A et CV-B en parallèle.

## CV-B (14/10, branche cv/stable)
Fait : taille fixe FolkPool + CountrysideLayer (`fixed_size`), semis ancré monde (aléa d'unité,
grille emboîtée), éclaircie stable en shader, fondus de zone et de portée, tests `fk_stable_test`.
Détails : ADR 0354 § Détails d'implémentation CV-B. Réglages : `town_maquettes.json` § props
(`folk_scale` 600, `folk_keep_*`, `folk_fade_band`, `folk_zone_margin`, `folk_edge_inner`, `folk_range_fade`).
À vérifier au regard (session principale) : lisibilité du tramé, densité en vue moyenne
(`folk_keep_from` 30), taille des charrettes ; compilation shader (testée en fenêtre, voir rapport).
Contrat pour CV-A : les accessoires de FolkModels doivent garder le shader `folk_prop.gdshader`
(ou inclure `folk_fade.gdshaderinc`) pour le fondu ; `fixed_size: true` sur les nouveaux ids du calque campagne.
