# GC — carte généralisée (objets grossis, positions vraies)

ADR 0158. Worktree `../gp-gc`, branche `feat/gc` (pyramide en lien symbolique vers main, dylib
copiée de main : aucun changement Rust prévu).

## Mandat (joueur, 02/10)
« Je sacrifie la vue rapprochée, carte lisible à hauteur de jeu. » Paris, la Seine, etc. agrandis,
géographie relative vraie. Origine : champs (HB3 ×3) et camps plus gros que les villes 1:1.

## Lots (plan révisé le 02/10 : villes **stylisées**, autonomie totale du joueur)
- [x] GC0 : diagnostic, ADR 0158, worktree, inventaire des points d'échelle.
- [x] GC1 : prototypes et planches. Réel grossi ×8 / ×14 **rejeté** (tache brune à moyenne
      distance) ; maquettes stylisées **retenues** (planche `gc1_reel_x8_vs_stylise.jpg`).
- [x] GC2 (agent, `../gp-gc`) : système de maquettes — données `data/art/town_maquettes.json`
      (tailles par type, familles d'architecture par culture/région, portées), `TownMaquetteLayer`
      en MultiMesh par tuile, activé par défaut, calques 1:1 éteints, emprises (clic, anneau,
      étiquettes, exclusions) sur la maquette, voisins trop proches réduits, `LandmarkModel` grossi.
- [x] GC3 (agent, `../gp-gc-kit`, branche `feat/gc-kit`) : kits de l'Est et du Sud générés par
      Blender (`tools/blender_scripts/settlements.py`) : `med`, `byz`, `rus`, `isl`, `steppe`,
      5 types × 2 variantes, noms `settlements/<type>_<famille>_<a|b>.glb`.
- [x] GC3b (agent de recherche) : modèles libres sur internet (CC0 / CC-BY seulement, dépôt
      public), rapport + téléchargements hors dépôt (`~/.cache/cent_ans/gc_assets/`).
- [~] GC4 : zone de retrait du fleuve générique grossie avec les villes emblématiques (fait) ;
      fleuves et routes élargis, ponts : **à faire** (raccord visible Seine de la maquette / fleuve fin).
- [x] GC5 : champs réduits (`docs/wip/gc-champs.md`), hameaux, moulins et fumées grossis
      (`props` de `town_maquettes.json`). Arbres : session HC (ADR 0161).
- [~] GC6 : plancher de caméra fait (`map.camera_floor_distance` 20) ; reste : plancher de caméra, retrait des paliers vallée/site, ménage du 1:1 et du prototype
      `MapScale`, banc.
- [ ] GC7 : camps (avec SA, ADR 0156).
- [ ] GC8 : tests, `godot-map.md`, ADR, mémoire, fusion dans main.

### Contrat entre GC2 et GC3
- Modèle : `res://assets/models/settlements/<type>_<famille>_<variante>.glb`, type ∈ city, town,
  castle, abbey, village ; famille ∈ med, byz, rus, isl, steppe (l'Ouest garde `<type>_<a|b>`).
- Un maillage joint, < 5 000 triangles (ville < 12 000), matériau `Banner` teinté par Godot,
  bâtiments prolongés sous z = 0, même emprise au sol que l'équivalent occidental (ville ≈ 3,6,
  bourg ≈ 2,0, château ≈ 1,9, abbaye ≈ 2,2, village ≈ 2,0 unités Blender).
- Modèle absent : repli sur la famille Ouest.

## Coordination
- SA (`../gp-sa`, ADR 0156) refait l'échelle des pions d'armée : ne pas toucher `army_markers.gd`.
- RF (`../gp-rf`) cuit E3 partout + E4 autour des villes : E4 devient peu utile (au joueur de dire
  dans cette session-là).

## Inventaire des points d'échelle (agent Explore, 02/10 ; chemins sous `game/`)
- **Ville proche (plan complet, rig < 45)** : un `Node3D` par ville, échelle `1/meters_per_unit`
  (`town_builder.gd:293`) ; sol posé par le shader (`town_building.gdshader`,
  `campaign_display_height`) et par `TownPlan.Heights` (échantillonne `ancre + m / mpu`). Villes v2 :
  même `TownBuilder` (`landmark_city_layer.gd:293`).
- **Ville lointaine F1/F2** : sommets cuits en coordonnées monde (`town_far_builder.gd:114-126`,
  `ancre + p / mpu`), fusionnés par tuile (128 / 512 unités) ; sol = grille polaire `ground_m`
  cuite aux rayons **réels** (à remplacer par un échantillonnage aux rayons grossis, GC2).
- **Seuils** (`resources/town_render.tres`) : `max_rig_distance` 45, `detail_range` 1,6,
  `block_range` 42, flux 4-48 ; `f1_range` 300, F2 jusqu'à `model_range` 1250.
- **Dépend du rayon de ville** (`settlement_layer.gd:269` `_compute_footprints`) : clic, anneau,
  hauteur d'étiquette, exclusions de végétation, hameaux, rues pavées (`fine_geo_layer.gd:189`),
  effets de vie et figurants (`life_effects.gd:328`, `folk_scenes.gd:172`). Indépendants :
  `CITY_CLEARANCE_PX` 26 (armées), `terroir_mask.gd` (rayons en px par rang).
- **Fleuves** : pas de multiplicateur unique. Rubans grossiers : largeurs de `rivers_render.json`
  (`rivers_renderer.gd:243`) + planchers en px écran ; rubans fins : mètres réels / mpu
  (`fine_ribbon_job.gd:229`) ; ponts `river_crossings.gd:23-28`.
- **Routes** : `road_renderer.gd:18-28` (0,38-0,55 unité + planchers px) ; fines 3-6 m
  (`fine_geo_store.gd:46`).
- **Caméra** : `min_distance` 22 hors zones ; `close_min_distance` 7 et
  `close_camera.tres` `level_min_distance` jusqu'à 0,3 dans les zones proches ;
  `zoom_tiers.tres` (vallée 8, site 1,8).
- **Accessoires** (`resources/map_prop_scale.tres`) : hameaux `hamlet_ratio` 0,03 (portée 260),
  moulins 0,0093 (30), cheminées 0,0098 (25), arbres `tree_ratio` 0,018 (30), herbe (2,6),
  figurants 1/mpu (3).
- **Captures** : `tests/vt_shots.gd` (Paris d 1100…15, Amiens d 150, fenêtre réelle) ;
  scène `campaign_map.tscn -- --screenshot= --focus=x,y,d`.

## GC1 — prototype
- `MapScale.town_scale()` (`map.town_scale` de `data/ui/campaign_map.json`, `--town-scale=<k>`) :
  les villes sont posées avec `mpu / k` (`TownData.meters_per_unit`,
  `LandmarkV2Library.town_meters_per_unit`) ; le finage reste en mètres réels. Défaut 1 (aucun
  changement tant que le joueur n'a pas choisi).
- `MapScale.town_range_scale()` (`map.town_range_scale`, `--town-range-scale=<k>`, défaut = le
  grossissement) multiplie les distances de `TownRenderProfile` (détail, blocs, flux, plan complet,
  teinte de toits).
- Connu : sol du lointain pris aux rayons réels (villes en pente : jupe insuffisante) ; hameaux,
  fleuves, routes, champs inchangés.

### Captures (02/10, `docs/img/gc/`, non suivies par git ; `tests/vt_shots.gd`, 640 × 400)
`gc1_x1_x8.jpg` (×1 / ×8, portées d'origine), `gc1_x8_x14_portees.jpg` (×8 portée ×8, ×8 portée
×3, ×14 portée ×4). Temps total par série de 4 vues : 60-97 s, aucune erreur de script.
- ×1 : Paris invisible à d 300 et d 60, minuscule à d 22 ; les champs le dominent.
- ×8, d 22 : vraie ville lisible (rues, enceinte). ×14, d 22 : maquette massive, maisons énormes ;
  la Seine non élargie y devient un ruisseau → GC4 indispensable.
- d 60 : ×8 présent mais modeste, ×14 bien posé. Avec la portée ×8 le plan complet remplace les
  dalles sombres du lointain.
- d 150-300 : même à ×14 les villes sont des taches brun sombre peu contrastées ; au nord (Lille)
  elles commencent à se toucher → loi compressive + fusion (GC3) et travail de lisibilité du
  maillage lointain (toits plus clairs, enceinte marquée) en GC2.
- Piste de réglage : Paris ×8, ville ordinaire ×12-14, portées ×4 à ×8.

### Piste stylisée (joueur, 02/10 : « styliser Paris et non qu'il soit réaliste »)
- `TownMaquetteLayer` (prototype, `--town-style=maquette`, `--maquette-scale=<m>`) : maquettes du
  kit `assets/models/settlements/` à taille monde constante par type (ville 8, bourg 4,5, château /
  abbaye 2,6, village 2,4 unités) + `LandmarkModel` des 7 villes emblématiques (taille d'origine,
  ≈ 13 unités pour Paris). À lancer avec `--no-towns --no-landmarks-1to1 --no-town-far`.
- Planche `gc1_reel_x8_vs_stylise.jpg` : à d 22 la maquette de Paris se lit d'un coup (toits
  orange, Seine et îles, enceinte) là où le réel ×8 est une nappe brune ; à d 60 elle est claire
  mais petite (13 unités contre ≈ 28 pour le réel ×8) ; à d 150-300 les deux restent presque
  invisibles : la maquette doit aussi grossir (Paris ×2-3, kit ×3-4) et le `LandmarkModel` n'est
  pas encore mis à l'échelle par le prototype (drapé sur le relief à revoir).

## GC2 — système de maquettes (fait le 02/10, à juger sur captures)
- **Données** `data/art/town_maquettes.json` + schéma `town_maquettes.schema.json` (test
  `tools/tests/test_town_maquettes_schema.py` : chaque culture, région et religion de province est
  dans une famille et une seule). `map.town_style` de `data/ui/campaign_map.json` : `maquette`
  par défaut, `--town-style=real|maquette` le remplace.
- **`TownMaquetteData`** : familles (culture > région > religion > Ouest), nom de modèle avec
  repli silencieux sur l'Ouest (`ResourceLoader.exists`), variante et lacet par id, collisions
  (`solve_collisions`, ordre de `SettlementData.settlements` = type, poids, id).
- **`TownMaquetteLayer`** : un MultiMesh par (modèle, tuile de `tile_size` = 256) ; bannières par
  donnée d'instance (`shaders/maquette_banner.gdshader`, `INSTANCE_CUSTOM`, la couleur de sommet
  des bâtiments n'est pas touchée) mises à jour par `SettlementLayer.refresh` ; pose au sol en
  mètres lus une fois (centre, abaissé à la moyenne de l'emprise) puis `display_height` par
  morceau recalé et par tranches de 400 quand l'échelle verticale change ; portée par type =
  fondu sur la distance du rig (`transparency` des tuiles du type) + `visibility_range_end` par
  tuile pour le culling ; ombres jusqu'à `model_shadow_distance` (FC1).
- **`LandmarkModel`** : `create(plan, terrain, échelle, cuisson différée)` ; zone, cœur et étendue
  cuite × `landmark_scale`, drapé ramené à l'échelle (`drape_scale` de `landmark.gdshader`) ;
  première cuisson différée (fil de travail) : la pose des 7 villes coûtait 0,6 à 2 s.
- **`SettlementLayer`** : style `maquette` → `towns`, `landmark_cities`, `town_far` nuls,
  `maquettes` créé ; `_model_radius` / `_model_top` = maquette (clic, anneau, étiquette, hameaux),
  exclusions de végétation ≥ 1,15 × la maquette, finage et `real_radius` réels ; zones des villes
  emblématiques × `landmark_scale` ; clic par l'emprise limité à la portée du type.
- **Mesures** (headless) : 2 147 lieux = 2 140 instances + 7 emblématiques, 1 245 MultiMesh pour
  les 10 modèles de l'Ouest, 51 lieux réduits, 1 091 lieux d'une famille sans kit (repli Ouest),
  pose ≈ 0,26 s.
- **Tests** : `game/tests/gc_maquettes_test.gd`, smoke. Adaptés : `sz4_prop_scale_test.gd`
  (force le style `real`), `settlements_render_test.gd` (contrôle du style) ; scripts de capture
  1:1 (`vt_`, `vh4_`, `vh6_`, `vh7_`, `sz4_`, `sz4b_shots.gd`) : à lancer avec `--town-style=real`.
  `tb2_declutter_test`, `da7d_overlap_test`, `tf_far_layer_test` échouent aussi en `real`
  (machine chargée : temps par image, noms coupés), sans lien avec GC2.
- **Captures** : `game/tests/gc_shots.gd` (`--out`, `--only`, `--full`), non exécuté par GC2.
- **Points ouverts** :
  - fleuve des villes emblématiques : la zone de `river_styles.json` (retrait du fleuve générique)
    n'est pas grossie, la maquette ×1,7 la dépasse (Seine de la maquette + Seine de la carte entre
    6,8 et 11,5 unités) → GC4 ;
  - maquettes du kit pas centrées sur leur origine (faubourg de `city_a` : jusqu'à 7 unités du
    centre pour un disque de 5,5) ; l'emprise est la demi-largeur des données ;
  - 1 245 MultiMesh pour 2 140 instances (≈ 1,7 par tuile) : les tuiles servent le culling, pas le
    regroupement ; +10 modèles par famille livrée. Si le nombre de nœuds pèse, tuiles plus grandes
    pour les types à longue portée ;
  - moulins, cheminées, hameaux, scènes FK restent à 1:1 autour de maquettes grossies → GC5 ;
    rues pavées du réseau fin dans 1,3 × la maquette ; finage du parcellaire proche (`TownLayer`)
    absent en style `maquette` ;
  - anneau de sélection masqué sous `4,4 ×` le rayon (Q8) : invisible à d < 24 pour une cité,
    d < 45 pour Paris → à revoir avec le plancher de caméra (GC6) ;
  - plancher de caméra `landmark_min_distance` de nouveau actif sur les 7 villes (GC6).

## Vague 2 (orchestrateur, 02/10) — réglages sur captures
- Tailles relevées après lecture en 1080p : ville 14, bourg 8, château 4,6, abbaye 4,4, village 4
  unités ; `landmark_scale` 2,2 (Paris ≈ 28 unités) ; `weight_gain` 0,85-1,45 selon le poids du
  lieu (racine carrée) : Constantinople ou Paris dominent, un petit évêché reste modeste.
  96 lieux réduits par collision.
- Kit occidental refait (GC3c, `settlements_west.py`, `<type>_west_<a|b>.glb`) : l'ancien kit BR1
  (24 k triangles, gris) faisait une tache à côté des familles claires. Rus' éclaircie, yourtes
  retravaillées, tous les modèles recentrés (décalage porté par le nœud du glTF : le test de pose
  compare `instance_transform × model_local⁻¹`). Ancien kit gardé en repli (`<type>_<a|b>`).
- Plancher de caméra : `CampaignCamera.floor_distance` (`map.camera_floor_distance`, 20), levé
  par `--camera-min`.
- Hameaux ×0,7 du modèle (≈ 1,3 unité), moulins ×0,5, fumées ×0,3, portées 220 : clés `props`,
  lues par `TownMaquetteData.prop` à la place des rapports 1:1 de `MapPropScale` (fichier de HC).
- Captures : `docs/img/gc/gc3_maquettes.jpg` (15 vues, 1080p recadré), `gc5_champs.jpg`,
  `kit_est_sud.jpg`.

## GC6-perf — une surface par maquette (fait le 02/10, teintes à juger en jeu)
Constat : 3 181 appels de dessin à d 150 (665 avec les villes 1:1) : 1 281 MultiMesh × 8 à 12
surfaces par modèle (une par teinte, matériaux propres à chaque `.glb`) + passe d'ombre.
- **Blender** : `settlements_east.bake_kit` (appelé par `settlements.export_model` et par la
  planche) cuit la teinte de palette de chaque face dans la couleur de coin (`COLOR_0`, linéaire)
  × l'ombrage de pied de mur que posait `kit_campaign.finish_parts` (0,6 au pied enterré → 1 à
  1,6 m), alpha 0 sur les faces `Banner` (1 ailleurs), puis ne laisse qu'un matériau `Kit`
  (rugosité 0,9, nœud « attribut de couleur » → couleur de base : la planche montre les teintes).
  Plus de passage par l'atlas `Building` ni d'UV pour ces modèles. Les 60 `.glb` des six familles
  réexportés : 553 surfaces → 60, 78 078 triangles et emprises inchangés (lecture du JSON glTF).
- **Godot** : `shaders/maquette_kit.gdshader` (albédo = couleur de sommet, sans conversion :
  linéaire des deux côtés ; alpha < 0,5 → `INSTANCE_CUSTOM.rgb`, convention de
  `maquette_banner.gdshader`), un seul `ShaderMaterial` pour tous les modèles et toutes les
  tuiles (`TownMaquetteLayer._kit_material`). Bannière par défaut lue dans la couleur de sommet.
  Ancien kit (`city_a`…, repli) : chemin d'avant (`BuildingMaterials`, surface `Banner`).
- **Ombres** : `shadow_range` par type dans `town_maquettes.json` (château 120, abbaye et
  village 100 ; type absent = sans limite propre), combiné à `model_shadow_distance` (FC1) dans
  `update_view`, par type.
- **Banc** (`--bench-map --bench-pan-only`, 10 s, même machine chargée, avant = commit précédent) :

  | distance | appels de dessin | primitives | i/s |
  |---|---|---|---|
  | d 150 avant | 3 182 | 3,27 M | 38,5 |
  | d 150 après | 1 080 (2 passes) | 3,02 M | 38,3 / 37,9 |
  | d 40 avant | 1 382 | 3,18 M | 20,7 |
  | d 40 après | 627 / 625 | 3,09 M / 3,07 M | 33,7 / 34,6 |

- **Fidélité des teintes** (vérifiée par le calcul, pas en jeu) : pour 99,8 % des sommets, la
  nouvelle couleur de sommet = ancien `baseColorFactor` × ancienne couleur de sommet (écart
  < 0,004) ; le reste = 4 à 8 sommets par modèle sur des faces à la limite mur / toit. Écarts
  voulus ou connus :
  - rugosité uniforme 0,9 : les coupoles et ardoises (0,4 à 0,6, `Gilt` métal 0,3) perdent leur
    reflet de soleil. Piste si ça manque : rugosité dans l'alpha (0,5 à 1), bannière à 0 ;
  - pièces `Wood` (hampes, couronnes de yourte, charrettes : 7 560 sommets sur 156 000) : couleur
    unie (0,096 ; 0,058 ; 0,033) = moyenne de la couche `Planks` de l'atlas × sa teinte, sans la
    texture ni l'usure SR5 ;
  - premier matériau de chaque ancien `.glb` (sol : `Street`, `Sand`, `Dirt`…) : Godot n'y
    appliquait pas la couleur de sommet ; ses flancs (enterrés) reçoivent maintenant l'ombrage.
- **Tests** : `gc_maquettes_test.gd` (une surface, matériau partagé, bannière par défaut, ombres
  par type), `test_town_maquettes_schema.py`, smoke.
- **Reste** : 1 281 MultiMesh pour 2 138 instances : c'est désormais le plancher des appels de
  dessin des maquettes (tuiles plus grandes pour les types à longue portée, ou un seul maillage
  par tuile, si le banc le demande encore) ; `cull_disabled` gardé comme les matériaux importés
  (faces arrière à retirer après contrôle visuel des maillages ouverts).

## Reste à faire
- GC4 : largeur des fleuves et des routes (pas de multiplicateur unique, voir inventaire) ; raccord
  de la Seine de la maquette de Paris avec le fleuve générique ; ponts.
- GC6 : retirer ou laisser dormir le parcellaire de près ZG5b, l'herbe et les figurants 1:1, les
  paliers vallée/site ; retirer le prototype `MapScale` et les calques 1:1 si le joueur confirme
  qu'il ne veut plus du style `real`. Banc : fait (GC6-perf) ; teintes des maquettes à une surface
  à juger en jeu.
- GC7 : camps, avec SA (ADR 0156).
- Bannières des 7 villes emblématiques (pas de teinte de contrôleur) ; étiquette de Vincennes dans
  l'emprise de Paris ; Saraï petite (poids faible) ; yourtes à juger en jeu.

## Prochaine étape
Fusion dans main après la passe de tests carte ; puis jugement du joueur en partie réelle.
