# vq-cperf : marge de temps d'image sur la carte (ADR 0405)

## Fait
- `markers/lod.rs::fade_maquettes` : plage de fondu écrite seulement pour les maquettes dont le type est dans la bande
  `[limite - marge, limite]` (décalage quantifié à 1 unité) ; hors bande, plage abrupte partagée `(0, MAX)` ; masquage
  au-delà de la limite toujours par `update_range` (racine cachée). Ancien chemin : `CENT_ANS_PERF_OFF=fadeband`.
- `countryside/budget.rs` : budget de racines par image (`render.spawn_roots_per_frame` = 150, `data/map/map_countryside.json`
  + schéma) partagé par `install_cell`, `stream_rocks` (reprise sans minuteur : `RockView::backlog`) et `swap_lods`.
  Ancien chemin : `CENT_ANS_PERF_OFF=spawnbudget`.
- Outil : `CENT_ANS_SPAWN_LOG=1` (journal `SPAWN_BURST frame N meshes M top [...]`, `campaign/perf_switch.rs`).
- Tests : `countryside::budget::tests` (4), `markers::lod::tests` (bande, quantification).

## Analyse de l'énigme VisibilityRange (Bevy 0.20)
- `RenderVisibilityRanges::range_to_index` n'est jamais purgée : chaque valeur distincte y reste (65 535 emplacements, puis
  tout nouveau partage l'emplacement 0). L'ancien chemin (valeurs continues, 1176 meshes, chaque image de panoramique) la
  sature en quelques secondes : le culling GPU par plage devient alors arbitraire. Hypothèse : le « 12 ms » de l'ancien chemin
  profitait de ce culling faussé, et les plages « constantes » rétablissaient le dessin complet (28 ms). NON prouvé.
- Plage non abrupte = variante de pipeline `VISIBILITY_RANGE_DITHER` ; expérience `fadesoftsolid` (plage constante non
  abrupte) ≈ plage abrupte : la variante de pipeline n'est pas en cause.
- Le fondu ne dépend que du rig ; la plage par mesh ne sert qu'à décaler œil/rig.

## Mesures (release, hors écran, machine à charge 90-150 : seulement relatif)
CPU (profil, panoramique d=30) : `extract_visibility_ranges` 1,49 + `write_render_visibility_ranges` 1,85 ms/image -> absents
du classement (>= -3,3 ms). Image (3 lots de paires alternées, p50 / p95 panoramique) : nouveau 40,4/88,9 (froid), 39,3/74,2,
38,0/65,4 contre ancien 34,9/78,2, 35,4/76,2, 39,4/79,2 : pas de régression GPU mesurable (bruit +-5 ms), p95 plutôt meilleur.
Rafales de spawn (descente seule, 3 paires) : plus gros pic après démarrage 4 000-4 200 meshes -> 267-356 ; image max
223-272 ms -> 130-149 ms ; p50/p95 descente 36,2/72,3 (budget) contre 36,2/69,6... (bruit) ; images > 50 ms 203-279 contre 126-341.

## Reste
- Images 2-4 de la scène (démarrage) : 4 573 meshes (pluie 1800, neige 1100, bannières, nuages), bâtiments de villes 1 400 :
  hors de la descente, à étaler si on veut < 50 ms au chargement.
- Vérifier sur machine calme : coût GPU (nouveau vs `fadeband`) et l'énigme ; deux captures de fondu non faites.
- `decor.rs` (budget 120) et `scars` non touchés.
