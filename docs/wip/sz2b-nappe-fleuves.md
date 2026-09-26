# SZ2b — nappe d'eau des fleuves au palier site (suite de SZ2)

Chantier SZ (`docs/wip/sz-suites-zoom.md`). Branche `sz2b-water-sheet` (worktree d'agent
`agent-a098d00691a342944`, depuis `main` 89bc960a). Liens symboliques non versionnés
`data/map/pyramid`, `tools/geo/raw` → dépôt principal ; dylib reconstruite
(`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge`, copie
dans `game/bin/libcent_ans.debug.dylib`). Ne touche ni aux villes (VH5-VH7), ni à l'exagération (SZ1).

## Défaut
Au palier site, la Seine à Rouen s'affiche en lit sableux gris sans nappe d'eau, grève verte entre
mur de rive et lit (`docs/img/vh4/rouen_site.jpg`, `rouen_pont.jpg`, `rouen_seine_sud.jpg`).
Recette de départ (`docs/img/sz2b/avant_*`) : même chose ailleurs, la Loire est **absente** au palier
site à Orléans et à Tours (et au palier vallée à Tours).

## Causes (établies)
- **A. Eau coupée sous les emprises** (règle ZG5b d'avant ZG6/VH4) : rubans fins coupés et lit non
  creusé sous les emprises des maquettes de colonies (rayon × 0,8 : Orléans 5,2 km, Tours 2,9 km,
  Marmoutier 1,5 km) et dans les zones personnalisées (Rouen, Paris, Londres, Bordeaux…). Au palier
  près, les maquettes sont masquées au profit des villes 1:1 (ZG6, VH4) : plus d'eau du tout autour
  des villes. Le « lit sableux » de Rouen était le chenal du relief E7 (sans eau), coloré par le
  terrain.
- **B. Demi-pixel entre rasters et vecteurs** : Godot lisait tous les rasters avec le pixel i centré
  en x = i (`uv = (p + 0,5) / taille`, `ReliefPyramid.GRID_OFFSET = −0,5`, `MapData.height_m_at`,
  miroir Rust de la végétation), les outils et toutes les données vectorielles avec le pixel i sur
  [i, i + 1] (`docs/geo.md` § convention). Relief affiché 360 m au nord-ouest des fleuves, colonies
  et villes 1:1. Mesure (`scratchpad/shift.py`) : les fleuves fins collent au relief E4/E7 avec un
  décalage (−0,5 ; −0,5) exactement à Orléans, Rouen, Paris, Londres, Tours. Effets : lit creusé sur
  la berge (Loire d'Orléans : fleuve fin sur la rive à 94 m, eau à 86,8 m, tranchée de 7 m), vrai
  lit sec à côté, falaise de la côte Sainte-Catherine dans Rouen 1:1.
- Écartées : niveaux `hydro_fine.water_level` (cohérents avec le relief dans la convention des
  outils : Rouen 2,9 m dans un chenal à −0,3 m sous des berges à 4-8 m ; Londres, Bordeaux, Paris,
  Tours idem), recalages en cache (clé liée à `BAKE_VERSION` 5, SZ2), ruban non construit (il l'était,
  seulement coupé ou enterré). **Aucune recuisson** nécessaire.

## Correctif
- A : `FineRibbonJob` ne coupe plus l'eau sur les emprises ni dans les zones des villes 1:1 : sommets
  marqués (`UV2.y` = ordre + 100 × marque), points de coupe doublés au bord ; `river_fine.gdshader`
  efface les sommets marqués selon `cover_open` (villes ZG6 actives) et `zone_open` (1 − opacité de
  la maquette L1/L2) ; `FineGeoLayer` distingue zones fermées (maquettes sans ville 1:1) et ouvertes,
  ne creuse plus d'exception que dans les zones fermées, masque les ponts-portes quand les maquettes
  sont masquées. Eau peu profonde moins transparente (`shallow_alpha` 0,9).
- B : ADR 0086. `GRID_OFFSET = 0`, `ReliefQuadtree.surface_height_at` sans −0,5 codé,
  `MapData.height_m_at` en (x − 0,5, y − 0,5), `uv = p / map_size` (terrain, parchemin, mer,
  `river_bed`, parcellaire fin, repli heightmap du quadtree), végétation native Rust (même
  décalage, même lecture).

## État : terminé (à fusionner par l'orchestrateur)
- [x] diagnostic (causes A et B, mesures) ; captures `avant_*`
- [x] correctifs A et B, ADR 0086, `docs/godot-map.md` (section SZ2b)
- [x] captures `docs/img/sz2b/apres_*` (Rouen site / pont / Seine sud / vallée, Orléans, Tours,
  Londres, Bordeaux aux paliers site et vallée, Val de Loire site ; `--map-weather=clear`)
- [x] tests Godot : zg5b_fine_geo (adapté : marques d'emprise, zones fermées / ouvertes, origine
  E4), zg7a, zg2_quadtree, zg7c_partial_cache, zg8_relief (pic cherché aux centres de pixels),
  zg6_towns, vh4_landmarks, smoke : OK ; pytest 723 OK ; cargo fmt / clippy OK, cargo test OK
  (`data-model real_data` échoue seulement avec le dossier cible partagé tant que son binaire de test
  vient d'un autre worktree : OK après `touch` du fichier de test)

## Mesures (relief − eau, mètres affichés, `sz2b_water_shots.gd --probe-heights`)
| Lieu | axe | berge 1,15 demi-largeur (médiane) |
|---|---:|---:|
| Rouen (Seine) | −2,9 | −0,2 |
| Val de Loire (Amboise) | −4,0 | −0,3 |
| Orléans (Loire) | −4,0 | −2,1 (rive sud : chenal E7 plus large que le ruban) |
| Tours (Loire) | −4,0 | −2,5 (rive sud basse, bras multiples) |

## Limites / suites
- Londres, Paris, Bordeaux, Avignon, Calais, Bruges : zones fermées tant qu'elles n'ont pas de ville
  1:1 v2 (leur maquette porte l'eau, caméra au plancher ZG4b) ; elles s'ouvrent seules quand
  VH5-VH8 ajoutent `data/landmarks_v2/<id>.json` (détection au chargement de la carte).
- Rouen : la Seine est à marée (drapeau `tidal`) : eau saumâtre grise et vasières au bord, lisible
  comme de l'eau de près, grise de loin ; la bande de terre à l'est du pont (île Lacroix) vient de la
  largeur de la ligne fine plus étroite que le chenal E7 (données `river_widths.json`), non traitée.
- Orléans, Tours : ruban plus étroit que le chenal du relief sur une rive (largeurs par ancrages) :
  bande basse sèche au bord de l'eau, sans mur.
- Villes ZG6 : `towns_1340.json` n'a pas de couloir de fleuve pour Orléans ni Tours (`river: null`) :
  le plan peut poser des maisons de faubourg au bord de l'eau (données `geo towns`, hors lot).
- Restent dans l'ancienne convention (invisible, ≤ 0,5 pixel de 719 m) : maillages E0 des morceaux
  (vue parchemin, repli sans pyramide), `FineTerrainJob` (repli sans cache), grille du fond ZG8
  (`ReliefFloor`, `campaign_relief_floor_info`, fichiers de SZ1 : laissés).
- Fichiers partagés à surveiller à la fusion : `terrain.gdshader` (1 ligne), `map_data.gd`
  (`height_m_at`), `relief_quadtree.gd`, `core/crates/vegetation/src/lib.rs`, `fine_geo_layer.gd`.

## Prochaine étape
Fusion par l'orchestrateur (pas de bascule de cache : rien n'a été recuit).
