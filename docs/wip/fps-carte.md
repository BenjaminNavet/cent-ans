# FPS carte — saccades au zoom / déplacement (demande du 29/09)

**État : EN PAUSE**, à la demande du joueur, jusqu'à la fin de la session FK (`living-map-folk-scenes`,
`docs/wip/fk.md`), qui touche au rendu de la carte. Des programmes tournaient à côté pendant les mesures
(charge 2 → 7) : à refaire sur une machine calme. Aucune optimisation faite.

## Constat du joueur
Le jeu saccade un peu, surtout sur la carte de campagne pendant les zooms et les déplacements.

## Mesures préliminaires (main d1ea1de0, M4 Pro, fenêtre 2624×1644 Retina, qualité Haute auto)
- `--bench-map --bench-probe` (Metal), 3 passes : panoramique p50 53-59 ms, p99 97-289 ms,
  680-778 images > 50 ms ; descente p50 8-42 ms. Pour comparaison, RS-K2 donnait une image p50 de 17,6 ms.
- Scripts : ~5 ms par image dans les pires images (sections `settle/*`, `life/*`, `lod/*` < 8 ms
  en p99) → les pics viennent du **rendu** (GPU / présentation), pas du GDScript.
- 8,6 M de primitives et ~1 150 appels de dessin à d=30.
- A/B Vulkan (`--map-ab=30`, masquage par couche) : `hide:Terrain` environ −40 ms, `low` −47 ms,
  `no_shadows` −8 ms ; les autres couches −1 à −4 ms. **Attention** : sous MoltenVK, `gpu_ms` n'est pas fiable
  (92 ms de GPU pour 6,9 ms par image lors de certaines passes). Ces valeurs ne servent qu'à orienter le travail.
- Metal : ni temps GPU ni horodatages par passe (`--ab-passes`, ajouté à `release_journey.gd`,
  ne remonte que `vp_end`) : il faudra Xcode Instruments / Metal System Trace pour découper.

## Pistes (à reprendre après FK, machine calme)
1. Refaire le banc Metal 3 fois sur une machine calme (base propre).
2. Chercher une régression GPU depuis PB1 (relief ~12 ms de shader à 1440×900) : shader du relief
   (hide:Terrain le plus coûteux), éventuels ajouts OM, GA ou FK ; le cas échéant, bisection à d=30.
3. Metal System Trace sur 5 s de panoramique pour obtenir le coût par passe.
