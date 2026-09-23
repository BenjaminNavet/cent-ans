# ADR 0001 — Cœur de jeu headless en Rust, exposé à Godot via GDExtension

Date : 2026-09-23. Statut : accepté.

## Contexte

Cent Ans combine une carte de campagne au tour par tour et des batailles en temps réel
avec pause (~40 unités de 60-120 soldats). Le jeu doit être déterministe (rejouabilité,
sauvegarde = état initial + ordres), testable sans lancer le moteur, et développé par
plusieurs agents en parallèle sans conflits entre simulation et présentation.
Godot 4.7 est retenu pour le rendu et l'UI ; GDScript seul serait trop lent pour la
simulation de bataille et difficile à tester hors moteur.

## Décision

- Toute la logique de jeu vit dans le workspace Rust `core/` : `data-model` (types serde,
  chargement de `data/`), `sim-campaign` (état de campagne, tours, RNG semé ChaCha8),
  `sim-battle` (tick fixe), `ai`. Ces crates n'ont aucune dépendance à Godot.
- Un seul crate `godot-bridge` (cdylib, crate `godot` 0.5 / godot-rust, feature `api-4-7`)
  expose des classes `RefCounted` (`CampaignSim`, plus tard `BattleSim`) à GDScript.
  Seules des valeurs simples et des `PackedArray`s traversent la frontière ; jamais de
  pointeurs partagés.
- `game/` (Godot, GDScript) ne contient que rendu, UI et entrées. Il ne dérive jamais
  une règle de jeu ; il lit l'état par identifiants et soumet des ordres.
- `core/build.sh` compile la dylib et la copie dans `game/bin/` ; le fichier
  `cent_ans.gdextension` la référence (macOS arm64, profils debug et release).
- Le déterminisme est une invariante testée : même seed + mêmes ordres = même état.

## Conséquences

- Positives : tests unitaires Rust rapides (`cargo test`) sans moteur ; smoke test Godot
  headless (`game/tests/smoke.gd`) qui vérifie l'intégration ; simulation et présentation
  évoluent indépendamment ; performances des batailles maîtrisées.
- Négatives : deux langages et deux chaînes d'outils ; toute nouvelle API doit être
  déclarée côté Rust (`#[func]`) avant d'être visible dans Godot ; la dylib n'est pas
  versionnée, il faut lancer `core/build.sh` après tout changement Rust.
- Contraintes : le crate `godot` doit suivre la version de Godot (`api-4-7` pour 4.7.x) ;
  macOS arm64 uniquement pour la v1.
