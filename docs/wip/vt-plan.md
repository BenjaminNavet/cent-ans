# VT — plan technique (rapport de l'agent de planification, 30/09)

## Constats
- 2 147 colonies (`data/settlements/prov_*`), 2 134 avec emprise dans `data/map/towns_1340.json` (438 cités, 699 villes, 418 villages, 335 châteaux, 244 abbayes) + 8 villes v2. Le « 570 » des commentaires est périmé.
- `TownPlan` coûte 0,5-2 s/ville : le lointain ne passe pas par lui.
- `towns_1340.json` a déjà : `radii` (32 relèvements), `faubourgs`, `walls`, `monuments` (`at`, `size_m`), `river`/`bridge`, `z_m`. Il manque des hauteurs fines sur l'emprise ; `towns.py:590` échantillonne déjà la pyramide.
- Rien n'aplanit le terrain sous les maquettes.

## Architecture (ADR 0138)
| Palier | Contenu | Tuiles | Triangles | Appels |
|---|---|---|---|---|
| F1 rig < ~300 | nappe de toits polaire (32 relèvements × 2 anneaux), jupe −30 m, faubourgs, enceinte (murs, chemin de ronde, portes), monuments simplifiés (nef + flèche, donjon) | 128 u, `visibility_range` | ≈370/ville | ≈10 (+ ombres < d 60) |
| F2 au-delà | polygone 16 côtés + jupe + 1 flèche si cathédrale/château | 512 u | ≈55/ville | ≤ 35, sans ombre |
| ZG6 rig < 16 | blocs/maisons (HLOD inchangé) | | | |

- Hauteurs : grille `ground_m` (centre + 32 × 2 points, mètres) cuite par `cent-ans geo towns`, schéma `towns_1340.schema.json`. Base en mètres dans `UV2.x`, posée par `campaign_display_height` (ZG8).
- Villes v2 : lointain depuis `districts`, `walls`, grands `monuments` de `data/landmarks_v2/*.json` (≈4 k tri/ville).
- Génération `WorkerThreadPool` au chargement (≈0,3 s réel), ≈30 Mo mémoire vidéo.
- Transition : index de ville par sommet ; texture R8 64×64 = villes 1:1 construites ; le shader enfonce les sommets d'une ville marquée sous `block_range × 0,95` (miroir de `FADE_SELF`, `town_builder.gd:375`). Masque mis à jour dans `TownLayer._stream`. Teinte partagée `roofscape.gdshaderinc`.

## Retraits
- `settlement_layer.gd` : `_build_model` :321, `_build_landmark` :379, `_limit_model`/`_update_model_shadows` :352-376, `_fit_models`/paires/absorption :483-557, `_apply_model_visibility` :558, `_fit_model` :566, `_ground_model`/`_low_point` :627-647, échelle SZ4b :648-765, `_landmarks_root.visible` :1072, `_compute_real_radii` :2186, `_update_model_visibility` :2231.
- `map_prop_scale.gd` :43-47, :103 (`settlement_*`) + `.tres` ; `settlement_fit.gd`.
- `landmark_city_layer.gd` : `PREPARE_VALLEY`, `_maquette`, `_city_visible`, `fade`.
- `town_layer.gd:176` : `min_valley_weight` → `max_rig_distance` (16) dans `town_render.tres`.
- `zoom_tiers.gd` : `model_range` sert aux tuiles F, `model_shadow_distance` retiré.
- Garder `landmark_model.gd` + `data/landmarks/` (bataille, `battle/landmark_backdrop.gd`).

## Recâblage
- Étiquettes : branche « au sol » de `_update_label_height` :1182, décalage px écran.
- Clic `_model_pick_score` (≈:1880) : score d'emprise réelle (`radii`), rayon min 8 px ; écu cliquable. Anneau de sélection sur l'emprise.
- `model_radius`, `model_top`, `model_holder`, `model_scale_at` (:2019-2066) → emprise/hauteur réelles ou null. Consommateurs : `life_effects.gd` :162-221, :367 ; `campaign_life.gd` :274-283 (`replace_models`, abandonné) ; `rivers_renderer.gd` :216 ; `fine_geo_layer.gd` :187 ; `folk_pool.gd` :560.
- Exclusion de végétation (:2001) : rayon `finage_radius_m`.
- Zones caméra/armées `campaign_map.gd:306-309` : lire les données L1/v2 sans instancier de maquette.

## Critères de perf (lot I)
`--bench-map --bench-probe` à d = 1100, 150, 30, 3 passes machine calme, comparé à la base : appels p50 à d 1100 ≤ 900 (1 023 aujourd'hui) ; primitives ≤ base ; i/s p50 ≥ base ; p99 `--bench-towns` non dégradé ; tâche principale ≤ 3 ms ; chargement non allongé.

## Arbitrages retenus
Arbres : exclusion par le finage seulement. Hameaux 1:1 (`hamlet_scale` = 1). Cheminées réelles, incendies exagérés. CV1 abandonné. F2 : toits mats. Jupe 30 m ; sans pyramide, repli sur `z_m`.
