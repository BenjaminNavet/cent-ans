# FPS carte — saccades au zoom / déplacement (demande du 29/09)

**État : 2 correctifs dans main, estimés et non mesurés** (le joueur a demandé une estimation : la machine est
chargée jusqu'au 30/09 à 6 h). Mesure réelle à faire sur une machine calme, après FK (`docs/wip/fk.md`).

## Fait (29/09 soir)
1. **Budget de pixels HiDPI** (ADR 0123, 1b94fc31) : « Automatique » garde le nombre de pixels
   rendus de la référence 1080p. Sur l'écran du joueur (2624×1644), Haute passe de 75 % à 52 %
   (vérifié dans une vraie fenêtre, libellé « MetalFX spatial 52 % »). Estimation (modèle en pixels de l'ADR 0080) :
   −33 % à d = 150, −34 % à d = 12, −16 % à d = 40.
2. **Ombres des arbres** coupées au-delà du zoom 120 en Haute (300 avant) et 100 en Moyenne (200 avant) :
   d = 150 : 9,84 → 6,52 M primitives (compté) ; d = 30 inchangé. Capture comparée : forêts à
   peine plus claires au zoom stratégique.

## Pistes non faites (primitives comptées à d = 30 / 150)
- 2 cascades d'ombre au lieu de 4 en Haute : −2,1 / −3,0 M primitives, mais ombres proches moins nettes.
- Ombres des colonies au zoom stratégique : −0,9 M primitives et −838 appels de dessin à d = 150.
- Ombres des effets de vie (`CampaignLife`) : −1,1 M. FK y travaille : attendre la fin de FK.
- Pics pendant la descente : dus au rendu (les scripts ne dominent que dans 5-45 des 310-374 pics).
  Il faut Metal System Trace pour les attribuer.

## Constat du joueur
Le jeu saccade un peu, surtout sur la carte de campagne pendant les zooms et les déplacements.

## Mesures préliminaires (sous charge, indicatives) (main d1ea1de0, M4 Pro, fenêtre 2624×1644 Retina, qualité Haute auto)
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

## Mesure du 03/10 (main 61fbbfd7f, machine chargée, charge 35-64 : ms gonflées, attributions fiables)
`--bench-map --bench-probe` (d = 30) : image p50 53,6 ms, p99 100 ms, 741 pics > 50 ms ; descente
process p50 20,7 ms (≈ 5 ms le 29/09) → **les scripts dominent de nouveau les pics**.
- `map.settlements` en tête de 550 des 741 pics (moyenne 8,4 ms/image, p99 54 ms) :
  - `settle/outbuildings` (TB3) : pics 40-85 ms. Cause lue : `OutbuildingLayer._write_batches()`
    réécrit toutes les instances (`_place` par dictionnaire) dès que le zoom change de 4 %, hors
    budget ; idem `_reground()`.
  - `settle/declutter` : ≈ 5 ms à chaque image en mouvement (`_declutter_step`).
- `map.life` / `life/reground` (FK) : pics 15-19 ms. `map.update_lod` : 3,4 ms en moyenne.
Compteurs (`--map-ab`, temps inexploitables : 6,9 ms partout = cadence) : d = 150 → 4,14 M prim.,
1 313 appels ; **rivières 1,42 M prim. (34 %)** et 281 appels ; colonies 682 appels (52 %) ;
relief 0,86 M. d = 40 → 4,71 M, 880 appels ; rivières 0,9 M, vie 0,67 M, colonies 455 appels.
