# DV — deux vues de la carte de campagne

Date : 2026-09-29. Statut : spec validée en brainstorming, plan à écrire.

## Problème

La carte change de rendu en quatre temps au dézoom : maquettes 3D (< 150 à 420), pictogrammes
peints + écu (palier « moyen », 150–620), provinces colorées + pictogrammes (palier « loin »,
620–2050), parchemin (fondu 2050–2500, zoom max 2600). Le palier pictogramme, qui schématise
les villes, n'apporte rien que la 3D ou le parchemin ne donnent déjà et brouille la lecture.

## Décision

Deux vues seulement, choisies automatiquement par la distance caméra :

- **Vue normale (0 → ~1200)** : relief 3D, maquettes des villes à toute distance de la vue,
  hameaux jusqu'à 260 (inchangé), étiquette nom + petit écu au-dessus de chaque ville (sans
  pictogramme, densité des noms décroissante selon le rang, règle actuelle), frontières DZ,
  marqueurs d'armée 3D. Les sous-paliers vallée et site (ZG4) restent tels quels.
- **Vue stratégique (> ~1200)** : le parchemin CM2 seul : noms des royaumes et des provinces,
  villes en vignettes à l'encre, armées en jetons à blason, météo. La couche « provinces
  colorées » disparaît.
- **Transition** : fondu croisé sur ~200 unités centré sur le seuil. Sélection et clics
  actifs dans les deux vues (mêmes positions écran).

Pas de bascule manuelle (touche ou bouton) : hors périmètre.

## Composants

| Unité | Changement |
|---|---|
| `game/scripts/map/zoom_tiers.gd`, `game/resources/zoom_tiers.tres` | enum `Tier { NEAR, VALLEY, SITE, STRATEGIC }` ; `strategic_threshold = 1200`, `strategic_fade = 200`, `strategic_weight(d)` seule source du fondu. Supprimés : `far_threshold`, `far_fade`, `near_threshold`/`near_fade` au sens « près → moyen », `town_label_distance`, `medium_weight`, `far_weight`, `near_weight`. `model_range = 1250` |
| `game/scripts/map/strategic_view.gd` | `weight_at` délègue à `ZoomTiers.strategic_weight` (suppression de `start_distance`/`end_distance`) ; `overlay.draw_towns = true` toujours |
| `game/scripts/map/settlement_layer.gd` | suppression du chemin pictogramme (quads d'atlas, poids near/medium/far). Maquettes jusqu'à `model_range`, étiquette nom + écu conservée (sans pictogramme), estompée par `1 − strategic_weight` |
| `HeraldryAtlas` (nouveau, `game/scripts/map/heraldry_atlas.gd`) | atlas des écus de faction, extrait de `SettlementMarkers` (`shield_atlas`, `shield_index`, construction depuis `PortraitLoader.heraldry_texture`) |
| `settlement_markers.gd`, `data/map/settlement_markers.json`, `data/schemas/settlement_markers.schema.json`, `game/assets/map/markers/settlement_markers.png` (+ `.import`) | supprimés |
| `game/scripts/map/city_markers.gd` | supprimé (noms de provinces du palier « loin ») ; les noms de provinces ne vivent plus que sur le parchemin. Retirer son instanciation dans `campaign_map.gd` |
| `army_markers.gd`, `campaign_life.gd`, `life_ambient.gd`, `life_effects.gd`, `campaign_map.gd` | tout usage de `medium_weight`/`far_weight`/`near_weight` au sens « près-moyen-loin » remplacé par `1 − strategic_weight` (vue normale) ou `strategic_weight` (vue stratégique) |
| `legend_sample.gd`, `data/schemas/map_legend.schema.json` et données de légende | entrées pictogramme remplacées par « maquette + écu » (vue normale) et « vignette à l'encre » (parchemin) |
| ADR (prochain numéro libre) « Deux vues de campagne » | remplace la partie DA3 de l'ADR 0066 (marqueurs peints) et les paliers moyen/loin du lot C6 |

Routes : l'affichage routes principales / toutes routes, jusqu'ici indexé sur moyen/près, passe
sur la vue normale (routes principales partout, toutes les routes sous l'ancien seuil près,
conservé comme simple distance `minor_roads_distance = 150`). Les routes sont masquées sur le
parchemin.

## Performances

Le passage de `model_range` de 420 à 1250 est le risque principal (chantier FPS carte ouvert,
`docs/wip/fps-carte.md`). Mesure : FPS à la distance 1100, France entière à l'écran, qualité
haute, sur le banc existant, avant/après. Si le budget est dépassé, dans l'ordre :
1. ombres des maquettes coupées au-delà de 500 ;
2. LOD simplifié des maquettes au-delà de ~500 (niveau le plus grossier des `visibility_range`
   existants).

Imposteurs pré-rendus : hors périmètre, seulement si la mesure l'exige après ces deux étapes.

## Tests

- Supprimé : `game/tests/da3_markers_test.gd`.
- Adaptés : `settlements_render_test.gd`, `zg4_camera_test.gd` (et tout test qui échoue sur
  les noms retirés).
- Nouveau `game/tests/dv_two_views_test.gd` : `strategic_weight` = 0 à 1000 et 1 à 1400 ;
  aucun quad de pictogramme instancié ; maquettes visibles à 1100 ; parchemin dessine les
  vignettes (`draw_towns`) ; `tier_at` renvoie NEAR / VALLEY / SITE / STRATEGIC aux bonnes
  distances.
- `smoke.gd` passe ; une capture de contrôle à 1100 et une à 1400 (budget captures : 3).

## Critères de réussite

- Aucun pictogramme de ville à aucun zoom.
- Au dézoom, la 3D reste jusqu'à ~1200 puis le parchemin prend le relais en un seul fondu.
- FPS à 1100 dans le budget du banc, éventuellement après les deux mesures ci-dessus.
