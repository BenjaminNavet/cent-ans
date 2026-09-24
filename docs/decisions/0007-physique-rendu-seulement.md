# 0007 — Physique Godot réservée au rendu (Jolt), jamais remontée au cœur

Date : 2026-09-24 (lot S1)

## Contexte
Les sièges (M8) montrent des pans de courtine qui passent d'intact à « éboulis » d'une image à
l'autre. Le lot S1 veut un effondrement spectaculaire : blocs qui basculent, planches de la porte
qui tombent, pierres du parapet qui se détachent, poussière. La façon naturelle de le faire est un
moteur physique à corps rigides. Or toute règle de jeu vit dans `core/` (Rust, déterministe,
ADR 0001) : PV des pans, brèches, blocage des régiments, pertes des défenseurs.

## Décision
- Le moteur physique 3D de Godot est **Jolt** (intégré depuis Godot 4.4 ; le projet tourne sur
  4.7) : `physics/3d/physics_engine="Jolt Physics"` dans `game/project.godot`. Plus stable que
  GodotPhysics pour des piles de blocs et moins coûteux à nombre de corps égal.
- La physique Godot sert **uniquement aux effets visuels**, déterministes ou non. Aucune position,
  collision ou durée issue d'un corps physique n'est lue par la simulation, ni envoyée au pont
  GDExtension. Le cœur décide ; Godot illustre après coup (`BattleSiege.update` lit `hp/max_hp`
  et déclenche l'effet, jamais l'inverse).
- Les débris vivent sur une couche de collision dédiée (`collision_layer` de
  `data/fx/siege_fx.json`) : ils ne touchent qu'eux-mêmes et un sol local posé pour l'occasion
  (le terrain de bataille n'a pas de collision). Les figurines (MultiMesh animées par shader)
  n'ont aucun corps et ne sont jamais heurtées.
- Les réglages (nombre de blocs, impulsions, durées, plafonds) sont des données
  (`data/fx/siege_fx.json`, schéma `data/schemas/siege_fx.schema.json`).
- Budget : un plafond de corps actifs (les plus vieux sont gelés) et de corps conservés (les plus
  vieux sont libérés). Les gravats définitifs restent les maillages statiques de `BattleSiege`.

## Conséquences
- Une rediffusion ou deux machines peuvent montrer des blocs tombés différemment : c'est
  acceptable, l'état de jeu (pan effondré ou non) reste identique.
- Toute future physique « de jeu » (projectiles balistiques, collisions d'unités) devra être
  écrite dans `core/`, pas dans Godot.
- Changer de moteur physique ne touche que le rendu ; revenir à GodotPhysics est un réglage.
