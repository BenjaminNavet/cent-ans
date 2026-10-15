# bm-perf : pics d'image de la carte de campagne (Bevy)

Outil : `BENCH_FEATURES=profile tools/bevy_bench/bench.sh <nom>` (voir `tools/bevy_bench/README.md`).

## Corrigé
- `update_plates` (armies/markers.rs) coûtait 30-40 ms par image et était dans presque chaque pic. Cause : la bande de
  la barre du haut `Rect::new(-1e4,-1e4,1e5,inset)` passée comme obstacle était indexée dans ~270 000 cellules de
  la `SpatialGrid` (HashMap) de `LabelPlacer` (provinces/labels.rs) à chaque placement (chaque image quand la caméra
  bouge). Les rectangles de plus de 256 cellules vont maintenant dans une liste linéaire (`big`) ; `candidates` sans BTreeSet.
- `ResMut<ReliefState>` dans `terrain::lod::update_lod` : `relief.set_vertical_scale()` passe par `DerefMut` à chaque
  image, donc `ReliefState::is_changed()` est toujours vrai. Dans mes dossiers : `markers::settle_ground` (replaçait
  toutes les ancres chaque image) et `life::stream::update_view` (`relief_version` +1 chaque image, tout le décor de
  vie replacé) lisent maintenant `countryside::ReliefVersion` (change uniquement si échelle ou pages changent).
- Paniques « Entity despawned » qui tuaient le banc : `try_insert` dans markers (landmark_fallback), weather
  (banner, atmosphere), water (apply hidden), armies/figures (Tinted).

## Constats hors de mes dossiers (non corrigés)
- terrain/lod.rs `update_lod` : remplacer l'accès mutable systématique par `relief.bypass_change_detection()` +
  `set_vertical_scale` seulement si la valeur change (ou `set_if_neq`). Les autres lecteurs de `ReliefState::is_changed()`
  (terrain/cpu_sampler.rs:76, relief_gpu.rs:91, countryside/layer.rs:178) bénéficieraient du correctif racine.
- armies/markers.rs `update_plates` : écrit `Visibility`, `Node.left/top` et `BorderColor` de chaque plaque à chaque
  image (flags Changed -> mise en page UI) ; utiliser `set_if_neq` ; `markers.iter().find` en O(plaques x marqueurs)
  -> HashMap id -> Transform ; le calcul O(n^2) de `stats.overlaps` à chaque image (debug) ne devrait tourner que sous dump.
- `battle::units::skinned::prepare_buffers` tourne en campagne (≈2,3 ms/image, max 30 ms) et `vegetation::billboard::prepare_buffers`
  (max 31 ms) : conditionner à l'état Battle / à des entités présentes.
- `check_entities_needing_specialization` (matériaux water, provinces drape, terrain) et `calculate_bounds` : un pic unique
  de ~150 ms au début de la descente (afflux d'entités), moyenne ≈ 0,5-0,8 ms par type de matériau.
- Pics restants (> 33 ms, p99 ≈ 45 ms) : tous dans `run_render_schedule` (monde de rendu), par paliers de 10 ms
  typiques d'une machine surchargée (charge 80-130 pendant les mesures) : pas de système de jeu en cause.

## Restes
- Rejouer sur machine calme ; chiffres Godot p99 41 ms de référence.

## Chiffres du banc (release, d = 30, charge machine 130 avant / 60 après : indicatif)
| | pan p50 / p95 / p99 / max (ms) | images > 50 ms | descente p99 / max |
|---|---|---|---|
| avant | 56,5 / 89 / 124 / 580 | 204 / 342 | 158 / 1456 |
| après | 16,5 / 32,9 / 51,9 / 83,7 | 14 / 1141 | 77 / 205 |
Mesure relative fiable : `update_plates` 35 ms/image en moyenne (profil) -> absent du top.

## perf2 (lot suivant)
- `update_plates` : écritures Visibility/Node/BorderColor seulement si changé, HashMap id -> marqueur, `PlateStats::overlaps()` calculé au dump (O(n^2) sorti de la boucle) ; `place_markers` : `Transform::set_if_neq`.
- `prepare_buffers` (skinned, billboard) : `run_if` sur présence de lots dans le monde de rendu. En campagne les lots skinned existent et leur `version` change à chaque image (animation) : le coût (≈2-4 ms) est un vrai envoi de tampon, pas un gaspillage de conditions ; piste : ne ré-uploader que les lots visibles / `write_buffer` sur un tampon persistant au lieu de `create_buffer_with_data`.
- Banc profil (machine chargée, mesure relative) : `update_plates`, `place_markers` hors du top 30 avant et après.
