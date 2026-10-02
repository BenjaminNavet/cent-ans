# GC — carte généralisée (objets grossis, positions vraies)

ADR 0158. Worktree `../gp-gc`, branche `feat/gc` (pyramide en lien symbolique vers main, dylib
copiée de main : aucun changement Rust prévu).

## Mandat (joueur, 02/10)
« Je sacrifie la vue rapprochée, carte lisible à hauteur de jeu. » Paris, la Seine, etc. agrandis,
géographie relative vraie. Origine : champs (HB3 ×3) et camps plus gros que les villes 1:1.

## Lots
- [x] GC0 : diagnostic, ADR 0158, worktree, inventaire des points d'échelle (ci-dessous).
- [ ] GC1 : prototype du grossissement des villes (un facteur réglable) + captures à hauteur de
      jeu pour choisir facteur, loi compressive et plancher de caméra. **Jugement du joueur.**
- [ ] GC2 : villes — tous les niveaux (plan, F1, F2), sol, masques, clic, anneau, étiquettes,
      zones d'armée.
- [ ] GC3 : collisions — lieux absorbés en faubourgs.
- [ ] GC4 : fleuves et routes élargis, ponts.
- [ ] GC5 : champs (HB3 par rapport aux villes), arbres, hameaux, moulins, figurants.
- [ ] GC6 : plancher de caméra, retrait des paliers vallée/site, ménage (ZG5b, herbe 1:1), banc.
- [ ] GC7 : camps (avec SA, ADR 0156).
- [ ] GC8 : tests, `godot-map.md`, mémoire.

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

## Prochaine étape
Jugement du joueur sur la planche réel ×8 / stylisé ; puis maquettes plus grosses à d 150-300.
