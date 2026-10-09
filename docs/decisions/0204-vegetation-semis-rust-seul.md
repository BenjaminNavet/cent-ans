# 0204 — Semis de végétation de campagne : Rust seul

Date : 2026-10-09 (lot SC PF-03 / MB3 / AD7)

## Contexte
Le semis des arbres de campagne existait en deux versions : le crate Rust `vegetation`
(`VegetationScatter`, pool natif, lot PB2) et un repli GDScript dans `vegetation_tile_job.gd`
(~650 l), utilisé quand l'extension manquait ou refusait la requête. Le crate gardait aussi la
voie « V4 legacy » (chêne, hêtre, conifère) quand aucune table d'essences n'était fournie. Le
jeu livré passe toujours par Rust avec la table `data/art/tree_species.json`.

## Décision
- Le semis passe uniquement par Rust. Le repli GDScript (candidats, haies, empaquetage) est supprimé.
- `VegetationTileJob` reste, réduit à son rôle de requête : grille grossière des masques
  (échantillonnée hors du fil principal), `native_params`, `apply_native`, dégagements HC1,
  recalage (`reground`) et constantes de format. Il ne peut pas être supprimé : `Vegetation`,
  `ForestDetail`, `TreeClearance` et les tests en dépendent.
- Crate `vegetation` : la voie V4 (`species: None`) est supprimée ; sans table, `scatter_tile`
  ne plante rien (le champ reste `Option` pour les requêtes de recalage). Le pont refuse
  `request` sans table d'essences.
- `Vegetation` : sans extension, sans table d'essences ou requête refusée, pas d'arbres et un
  `push_error` ; plus de semis dégradé. Drapeau `use_native_scatter` supprimé.
- Tests : `hb4_species_test.gd` ne teste plus que le semis natif (plus de comparaison au V4) ;
  `pb1_veg_job.gd` (banc GD contre natif) supprimé.

## Conséquences
- Une seule implémentation des règles de semis, à vérifier par `cargo test -p vegetation`
  (sommes pinned inchangées pour les cas à essences).
- Sans la dylib `godot-bridge`, la carte n'a plus d'arbres (le jeu exige déjà la dylib).
- Gain : ~450 l GDScript, ~90 l Rust.
