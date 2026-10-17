# 0405 — Fondu des maquettes par bande et budget de spawn partagé

## Contexte
`markers/lod.rs::fade_maquettes` réécrivait chaque image la `VisibilityRange` des ~1176 meshes de maquettes :
`extract_visibility_ranges` + `write_render_visibility_ranges` (~3,3 ms/image mesurés au profil), plus une ré-extraction
des meshes (`Changed<VisibilityRange>`). Lecture de Bevy 0.20 :
- `RenderVisibilityRanges::range_to_index` n'est jamais purgée (65 535 emplacements, puis tout nouveau partage
  l'emplacement 0) : des plages continues réécrites chaque image la saturent en quelques secondes de panoramique.
- Une plage non abrupte (start != end) active la variante de pipeline `VISIBILITY_RANGE_DITHER`.
- Le test de distance de la plage est fait par vue (CPU `check_visibility_ranges`, GPU `mesh_preprocess`), depuis l'œil.
Le fondu ne dépend que de la distance du rig (décalage œil-rig propre à chaque mesh) : hors de la bande
`[limite - marge, limite]`, un mesh est soit pleinement visible, soit masqué (racine `Visibility::Hidden`, `update_range`).

Les rafales de spawn de la descente (jusqu'à 4 600 meshes en une image : props de campagne 1 500, affleurements 4 000,
changement de niveau de détail 1 500) coûtaient 94-190 ms (`calculate_bounds`, `check_entities_needing_specialization`).

## Décision
1. Les meshes de maquette gardent la plage abrupte partagée `(0, MAX)` hors bande ; la plage de fondu n'est écrite que pour
   les maquettes dont le type est dans la bande, avec un décalage œil-rig quantifié à 1 unité (ensemble de plages borné).
   Interrupteur de l'ancien chemin : `CENT_ANS_PERF_OFF=fadeband` ; `fadesoftsolid` garde une plage constante mais non abrupte
   (expérience : variante de pipeline dither conservée).
2. `countryside/budget.rs::SpawnBudget` : budget de racines par image (`render.spawn_roots_per_frame`, 150, dans
   `data/map/map_countryside.json`) partagé par les semis de cellules, les affleurements (`rocks.rs`, avec reprise sans
   attendre le minuteur) et les échanges de niveau de détail (`swap_lods`). Ancien chemin : `CENT_ANS_PERF_OFF=spawnbudget`.
3. Outil dev `CENT_ANS_SPAWN_LOG=1` : journalise les images qui ajoutent 100+ meshes et leurs ancêtres nommés.

## Conséquences
- -3,3 ms/image de CPU rendu sur la carte (profil). Coût GPU : aucune régression mesurable (A/B alterné), l'énigme de la note
  bm-perf4 (« plages constantes = 28 ms ») n'est pas reproduite avec ce schéma ; cause non établie (voir `docs/wip/vq-cperf.md`).
- Les props apparaissent sur 2-10 images au lieu d'une (invisible : ils poussent déjà en fondu).
