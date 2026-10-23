# solari-soldats (23/10) : ombres des soldats sous Solari

État : squelette. Voir « Prochaine étape ».

Conception : proxies de rayons (boîtes de 12 triangles, BLAS partagé par type), un pool d'entités par type (fantassin /
monté), sans `Mesh3d` (invisible au raster), alimenté depuis `SkinnedBatch.records` des régiments (pas de requête sur
13 000 entités). Code : `client/src/battle/scene/solari_soldiers.rs`. Réglages : `data/fx/solari.json`
(`soldiers_in_tlas`, `soldier_proxy_budget`, tailles).

## Prochaine étape
Implémenter, tests unitaires du pool, clippy, puis lister les vérifications GPU pour la session principale.
