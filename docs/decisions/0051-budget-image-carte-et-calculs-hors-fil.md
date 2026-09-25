# 0051 — Budget de temps par image sur la carte, calculs lourds hors du fil principal

Date : 2026-09-25 · Lot PB1 (benchmark et performances)

## Contexte

Mesures PB1 (M4 Pro, 1440×900, qualité Haute) : en zoomant, des dizaines de tuiles de relief
changent de niveau dans la même image. Chaque système réagissait tout de suite et à pleine
charge : tuiles proches (4 par image), rubans de route (3), hameaux (4), recalage de toutes les
étiquettes de colonies et de tous les moulins, cheminées et feux de la carte à chaque tuile. Une
image pouvait durer de 0,6 à 1,2 s à l'arrivée au zoom « comté ». En fin de tour, certains
recalculs (masque de terroir, mini-carte du panneau de diplomatie, figurines de toutes les
armées) bloquaient aussi le fil principal alors que leur résultat ne changeait pas ou pouvait
arriver une image plus tard.

## Décision

1. **Budget commun par image** (`FrameBudget`, 6 ms) pour les constructions progressives de la
   carte : chaque système construit au moins un élément par image (la progression est garantie),
   puis continue tant que le budget de l'image n'est pas épuisé. Les plafonds `max_*_per_frame`
   restent des bornes supérieures. `flush()` (captures, tests) lève le budget
   (`FrameBudget.unlimited`).
2. **Regrouper les recalages** déclenchés par `chunk_surface_changed` : marquer, puis traiter une
   fois par image (étiquettes de colonies), et seulement ce qui a changé (moulins, cheminées et
   feux des seules tuiles modifiées, écrits en bloc dans le tampon MultiMesh).
3. **Ne pas refaire un calcul dont les entrées n'ont pas changé** : textures de mini-carte mises
   en cache par `MapData`, marqueurs d'armée gardés si l'armée est identique (signature hors
   position), effets de vie reconstruits seulement si leurs entrées effectives changent (seuils de
   dévastation, paliers de ruine, maquettes).
4. **Calcul pur hors du fil principal** quand le résultat peut arriver une image plus tard ou
   quand il se découpe : masque de terroir des fins de tour (`TerroirMask.build_async`), décodage
   des masques de la carte et sommets des tuiles lointaines au chargement (`WorkerThreadPool`).
   Les nœuds, maillages et textures restent créés sur le fil principal.

Règle pour la suite : aucun contenu n'est retiré ni simplifié. Le résultat affiché doit être
identique, ou le même avec une ou deux images de décalage.

## Conséquences

- Le contenu apparaît parfois une ou deux images plus tard au zoom (routes, hameaux, tuiles
  proches), au lieu de figer l'image.
- Tout nouveau système qui construit du contenu au zoom doit passer par `FrameBudget.has_time()`
  (au moins un élément par image) plutôt que par un nombre fixe par image.
- Les bancs `game/tests/pb1_bench.gd` (zoom, pics attribués avec `--trace`) et
  `game/tests/pb1_turns.gd` (fins de tour) mesurent ces régressions.
