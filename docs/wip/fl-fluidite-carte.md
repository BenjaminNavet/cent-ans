# FL — fluidité de la carte : arbres qui clignotent, appels de dessin, FPS (2026-10-08)

Demande du joueur : « les trois me gênent » (arbres qui clignotent, appels de dessin, FPS de la
carte), après avoir écarté une migration Unity/Unreal. Branche `feat/fl`, worktree `../gp-fl`
(dylib copiée de main, `data/map/pyramid` en lien symbolique). Coût cloud : 0 $.
Reprend `docs/wip/arbres-clignotants.md` et `docs/wip/fps-carte.md` (PF, ADR 0169).

## État
- [x] FL0 banc de référence (main d327f2ebd, Haute auto, fenêtre Retina ; charge ≈ 2 au début,
  25 à la fin : la session SC a démarré pendant d = 150).
  | d | image p50 | p99 | i/s | appels p50 | primitives p50 |
  |---|---|---|---|---|---|
  | 30 | 31,2 ms | 100 ms | 27,9 | 652 | 2,87 M |
  | 150 | 45,7 ms | 116 ms | 22,2 | 865 | 3,06 M |
  Sondes (moyenne par image / pics dominés) : **`map.misc` 7,8 ms (d 30) et 12,1 ms (d 150), en tête de
  316 et 391 pics** ; `settle/declutter` 3,9 / 6,2 ms ; `map.update_lod` 2,2 / 2,6 ms ; non attribué
  4,7 / 8,6 ms ; `life/reground` pics à 35 ms. Descente : 249 des 253 pics dominés par les scripts.
  → Le FPS est d'abord limité par le **GDScript du fil principal**, pas par le GPU.
- [x] FL1 arbres qui clignotent : ombres des arbres allumées pour toute la vue selon le zoom
  (`_tree_shadows_on`, hystérésis 8 %), plus partie par partie. Sonde `tree_flicker_probe --keys`
  (bascules d'ombre de parties visibles aux deux images) : rig 60 **22 → 0**, rig 40 **30 → 0**.
  Tests ga3_l2_vegetation, hc_forest, fc1_shadows OK ; tf_far_layer échoue sur un seuil de temps
  (124 ms > 8, charge 231 : SC). Coût des ombres en plus à rig < 64 : à mesurer en A/B (FL3).
  Restes possibles du clignotement : changements de niveau du relief (64 en 150 images) et
  recalages d'arbres une image après (≤ 22 % de la hauteur) — à revoir si le joueur en voit encore.
- [ ] FL2 appels de dessin et primitives.
- [ ] FL3 images p50/p99 de la carte (shader du terrain, pics de scripts).

## FL1 — diagnostic (lecture du code)
Style généralisé (HC1) + imposteurs GA3 actifs par défaut (`Ga3Vegetation.near_impostors()`) :
`_meshes(DETAILED)` et `_meshes(FAR)` rendent les **mêmes** imposteurs ; le changement de palier
par partie de tuile (`lod_d` < `generalised_mesh_distance` = 60) ne change donc que
`cast_shadow` de la partie entière. En panoramique au zoom moyen (rig < `generalised_shadow_distance`
= 70), les parties franchissent cette frontière en continu : l'ombre de tout un paquet d'arbres
apparaît ou disparaît d'une image à l'autre. C'était la piste 1 de `arbres-clignotants.md`
(4 bascules d'ombre en 150 images à rig 60). `ForestDetail` n'est pas créé en style généralisé.

## Coordination
Session SC (`../gp-sc`, `feat/sc`, simplification de tout le code, 20 agents) démarrée le 08/10 :
elle peut toucher les fichiers FL. Bancs FL : uniquement `--bench-ab` en processus (charge).

## Prochaine étape
Décomposer `map.misc` (campaign_map.gd:1548) et `settle/declutter` ; A/B du coût d'ombres FL1.
