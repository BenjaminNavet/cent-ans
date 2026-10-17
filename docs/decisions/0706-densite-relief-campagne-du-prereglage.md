# 0706 — La campagne applique la densité de relief du préréglage

Statut : accepté (17/10, lot perf des correctifs post-playtest).

## Contexte
Les préréglages de `data/fx/render_quality.json` portent `relief_vertex_px` (pixels par sommet du quadtree de
relief : ultra 6, high 7,5, medium 8, low 12) et `relief_items` (budget de nœuds : 900 / 700 / 500 / 350). Le
client Godot les appliquait (ADR 0169, qui les a calés pour tenir l'image/s). Le client Bevy ne les lisait pas :
`relief_lod::Config::default()` (4 px, 700 nœuds) servait à tous les niveaux, soit des triangles environ deux fois
plus nombreux que prévu en ultra (1,5 à 2 M de triangles à d=150 / d=60).

## Décision
`Preset` / `RenderQuality` lisent les deux clés ; `terrain::lod::sync_relief_quality` les écrit dans le sélecteur
(`max_vertex_px`, `max_items`) et dans le géomorphage du shader du terrain (`look.morph.y`, qui doit connaître le
même pas). `CENT_ANS_RELIEF_PX` surcharge le pas pour les essais.

## Mesures (release, 1280x800 hors écran, ultra, A/B alternés)
d=150 p50 30,5 -> 23,2 ms (6 px) ; d=60 45,8 -> 41,2 ms. Capture d=60 et d=150, 4 px contre 6 px : écart moyen
1,2 et 0,8 sur 255, 1,6 % et 0,6 % des pixels à plus de 16 ; aucune différence visible sur le crop le plus
différent.

## Conséquences
Le niveau choisi par le joueur règle enfin la densité du relief. La valeur d'ultra suit l'ADR 0169 ; la changer se
fait dans la donnée, pas dans le code.
