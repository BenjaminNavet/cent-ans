# 0124 — Deux vues de la carte de campagne

Date : 2026-09-29. Statut : acceptée. Remplace la partie DA3 de l'ADR 0066 (marqueurs peints :
pictogramme + écu) et les paliers « moyen » et « loin » du lot C6 (ADR 0036).
Spec : `docs/superpowers/specs/2026-09-29-dv-deux-vues-campagne-design.md`.

## Contexte
Au dézoom, la carte changeait quatre fois de rendu : maquettes 3D, pictogrammes peints + écu,
provinces colorées, parchemin. Le palier des pictogrammes schématisait les villes sans rien
apporter que la 3D ou le parchemin ne donnent déjà. Il brouillait la lecture.

## Décision
Deux vues, choisies par la distance caméra, avec un seul fondu croisé
(`ZoomTiers.strategic_weight`, seuil 1200, largeur 200) :
- **vue normale** : relief 3D, maquettes jusqu'à `model_range = 1250`, nom + petit écu au-dessus
  des villes (densité selon le rang), frontières, marqueurs d'armée 3D ;
- **vue stratégique** : le parchemin CM2 seul, avec ses vignettes de villes à l'encre.

Aucun pictogramme de ville à aucun zoom. Pas de bascule manuelle.

Écarts à la spec, relevés à la lecture du code :
1. `near_threshold = 150` / `near_weight` restent comme sous-palier « détail proche » de la vue
   normale (hameaux, toutes les routes, relief fin, gens et effets de vie). Les indexer sur la
   vue normale les allumerait jusqu'à 1200. Ce sous-palier tient lieu du `minor_roads_distance`
   de la spec.
2. `data/map/settlement_markers.json` reste, allégé : il porte les rangs, la densité des noms par
   rang et le dé-encombrement. On retire l'atlas des pictogrammes et son PNG. L'atlas des écus
   passe dans `HeraldryAtlas`.
3. L'écu reste un quad du multimesh existant, composé seul par `settlement_icon.gdshader`, pour
   garder les tailles par rang et le dé-encombrement.
4. La couleur « politique » du shader du terrain (provinces colorées au loin) est neutralisée en
   vue normale : le parchemin assure seul la lecture politique.

## Conséquences
- Plus de noms de provinces en 3D (`CityMarkers` supprimé) : ils ne vivent que sur le parchemin.
- Les maquettes restent affichées trois fois plus loin (420 → 1250). C'est le risque de
  performance. Mesure à d = 1100 ; si le budget est dépassé : ombres des maquettes coupées
  au-delà de 500, puis LOD grossier au-delà de 500.

## Mesure (30/09, banc `--bench-map --bench-distance=1100`, Haute, machine chargée)
| | base (3b4882b7e) | DV sans coupure | DV + ombres des maquettes coupées > 500 |
|---|---|---|---|
| primitives p50 | 9,59 M | 10,44 M | 9,82 M |
| appels de dessin p50 | 842 | 1 032 | 1 023 |
| image p50 | 38 ms | — | 40 ms |

La charge de la machine (35 à 140) rend les temps d'image indicatifs. Les primitives reviennent au
niveau de la base (+2 %) ; les appels de dessin restent à +21 % (maquettes). Le LOD grossier
au-delà de 500 n'est pas appliqué : il réduirait les primitives, pas les appels. À mesurer sur une
machine calme avec le chantier FPS carte (`docs/wip/fps-carte.md`).
