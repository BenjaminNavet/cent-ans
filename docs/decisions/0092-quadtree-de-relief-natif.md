# 0092 — Sélection native du quadtree de relief (PB3g)

Date : 2026-09-26. Statut : accepté.

## Contexte
Après SZ6, le quadtree de relief (ZG2, ADR 0036) restait le plus gros coût de script de la
carte de campagne en zoom : 9 à 12 ms par image sous charge pour la sélection récursive
(`_select`, ~700 nœuds, dictionnaires par nœud), l'application (`_apply_items` : paramètres
d'instance comparés nœud par nœud) et le tri des pages voulues. Par ailleurs
`VegetationScatter.request_reground` recopiait vers Rust toutes les pages de l'instantané d'une
tuile (jusqu'à 256 × 512 Ko, ~70 ms en une image).

## Décision
- Crate Rust pure `core/crates/relief-lod` (sans Godot, opt-level 3 en dev) : présence des
  tuiles de la pyramide (`has_tile`, `max_level_under`, `finest_ancestor`, tuiles cassées),
  sélection CDLOD (portées, critère `max_vertex_px`, budget `max_items` et hystérésis de
  `px_scale`, pages fines et grossières, pages voulues triées), résidence des pages (couches,
  `last_used`, choix LRU de l'éviction, ordre d'insertion sur égalité) et **différence avec
  l'image précédente** : nœuds ajoutés (géométrie, boîte), retirés, ombre portée changée,
  paramètres d'instance changés (masque des 7 paramètres, fondu, cache par version de résidence).
- Classe `ReliefLod` (godot-bridge, fil principal) : `update(camera, planes, view, config)` rend
  des tableaux Packed ; `ReliefQuadtree` ne fait plus que créer, déplacer et masquer les
  `MeshInstance3D` signalés et envoyer les paramètres marqués. Décodage, téléversement,
  creusement ZG5b, `surface_height_at` et instantanés restent en GDScript.
- Numérique calquée sur GDScript (scalaires f64, vecteurs f32) : mêmes nœuds sélectionnés, à
  l'égalité exacte (clés, pages, distances) — `pb3g_quadtree_test.gd` compare les deux
  sélections sur cinq caméras et plusieurs images, pages en cours de chargement comprises.
- `ReliefLod` garde aussi les octets des pages résidentes (`Arc`, une copie au téléversement) ;
  `surface_snapshot` porte `qt_store`, et `request_reground` (`ground_of`) y prend les pages au
  lieu de les recopier (repli : copie, pour un instantané sans magasin).
- Repli GDScript complet sans l'extension, avec `use_native_select = false` ou
  `--no-native-quadtree` (comparaisons A/B).

## Conséquences
- Voir les mesures dans `docs/wip/pb3g-quadtree-natif.md`.
- Deux implémentations de la sélection (GDScript de repli, Rust) : toute modification de la
  sélection doit être reportée dans les deux, le test de comparaison le vérifie.
- Rendu seulement, aucune règle de jeu.
