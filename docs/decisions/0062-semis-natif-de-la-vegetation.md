# 0062 — Semis natif de la végétation de campagne (PB2)

Date : 2026-09-25. Statut : accepté.

## Contexte
Une tuile de végétation (256 px de carte, 7 000 à 21 000 arbres et buissons) coûtait
120 à 330 ms en GDScript, dont 75 à 90 % pour le semis des candidats, les haies du parcellaire
et l'empaquetage des tampons `MultiMesh`. Le démarrage à chaud attend cinq tuiles (image figée
~1 s), puis la vue initiale se complétait en ~11 s. Le recalage des arbres quand le relief
d'une tuile change de niveau (`VegetationTileJob.reground`, lot C7b) coûtait jusqu'à 440 ms par
tuile. Ce sont des millions d'opérations scalaires, où l'interpréteur GDScript est le plus lent.

## Décision
- Crate Rust pure `core/crates/vegetation` (sans Godot) : semis, haies (portage de
  `VegetationFields`), empaquetage et recalage, avec lecture de la heightmap, du lit des fleuves,
  des grilles de maillage et des pages du quadtree de relief (`ReliefQuadtree.sample_pages`).
  Optimisée même en profil `dev` (`[profile.dev.package.vegetation] opt-level = 3`).
- Classe `VegetationScatter` (godot-bridge) : pool de fils natifs, même schéma que
  `ReliefDecoder` (ADR 0036) — le fil principal convertit les champs en données Rust
  (`request`, `request_reground`), les fils ne touchent jamais l'API Godot, le fil principal
  relève les résultats (`poll`).
- La grille grossière des masques (bruit `FastNoiseLite`, pentes, splat) reste en GDScript dans
  le `WorkerThreadPool` (`VegetationTileJob.coarse_only`), puis part au pool natif.
- Repli complet en GDScript si l'extension n'expose pas la classe ; `--no-native-vegetation`
  force ce repli (comparaisons).
- Le flux aléatoire (PCG32) diffère de `RandomNumberGenerator` : le résultat est statistiquement
  équivalent (mêmes lois, mêmes bords de haie plantés par le tirage haché, nombres d'instances
  à ±1-3 % par essence), pas identique. Toute modification du parcellaire doit être reportée dans
  `VegetationFields`, `terrain.gdshader` **et** `core/crates/vegetation`.

## Conséquences
- Semis + haies + empaquetage : 90-330 ms → 2-10 ms par tuile ; démarrage à chaud 0,9-1,0 s
  → 0,13-0,16 s ; vue initiale complète 11,8-14,1 s → 6,4 s depuis le début du chargement
  (le reste est le chargement et le relief fin) ; recalage d'une tuile 440 → 6 ms au pire.
- Une troisième copie des formules du parcellaire (GDScript, GLSL, Rust) : tests Rust sur le
  hachage et sur la pose, banc `pb1_veg_job.gd` qui compare les nombres par essence.
- Aucune règle de jeu concernée (rendu seulement).
