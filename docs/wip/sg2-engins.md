# SG2 — Engins de siège animés, huile, démos d'Avignon et de Bruges

Branche `worktree-agent-a89b4bf8d49010116`. Suite de SG1 (`docs/wip/sg1-sieges.md`, ADR 0023) et
L3 (`docs/wip/l3-materiaux-sieges.md`, ADR 0026).

## Objectif
1. Engins animés : trébuchet (contrepoids, verge, fronde qui lâche la pierre au tir du cœur),
   bombarde (recul, fumée, éclair), mangonneau ; bélier et beffroi (roues, oscillation) d'après la
   position du cœur. Modèles Blender procéduraux (`tools/blender_scripts/siege_engines.py`),
   animation par nœuds nommés, réglages dans `data/fx/siege_engines.json`.
2. Huile bouillante : son (banque AU1), vapeur, coulure.
3. Démos de siège d'Avignon et de Bruges (plans L2/L3), jouables depuis le menu.
4. Impacts des engins marqués sur les murs.

## État
- [x] Cœur : `Unit::reload_period`, `shot::ENGINE_RELOAD` ; pont `get_units().reload /
  reload_period` ; `debug_stage_landmark_siege` (campagne + pont) ; tests `sim-battle/tests/sg2.rs`,
  `sim-campaign/tests/sg2_landmark_demo.rs`.
- [ ] Modèles Blender (trébuchet, bombarde, mangonneau, bélier, beffroi) + manifeste.
- [ ] `game/scripts/battle/siege_engines_fx.gd` (animation), branchement SG1/BV1.
- [ ] Huile : son, vapeur, coulure.
- [ ] Menu des démos, démos Avignon et Bruges.
- [ ] Marques d'impact sur les murs.
- [ ] Captures (trébuchet à mi-tir, bélier à la porte, huile).

## Prochaine étape
Script Blender `siege_engines.py`.
