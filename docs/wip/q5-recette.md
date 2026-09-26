# WIP Q5 — recette « comme un joueur » (2026-09-26)

Demande du joueur : tester le jeu comme un joueur, identifier les problèmes, puis les corriger
en autonomie. Pilote `game/tests/q3_playtest.gd` sur main f82a03a6. Rapport :
`docs/audit/q5-recette.md`. Branche `fix/q5-recette`, worktree `../gp-q5`.
Le joueur : une seule partie suffit (pas de seconde partie de vérification).

## État
- [x] partie France 1920×1080 ; partie Angleterre 1280×720
- [x] tri des constats, rapport
- [x] 8 corrections (voir rapport) + tests `sim-campaign/tests/q5_recette.rs`
- [x] fmt / clippy / tests (61 binaires sim-campaign, ai) / smoke Godot vert
- [x] ff dans main (après fusion de main : clippy, 61 tests, smoke verts)

## Prochaine étape
Vérifications puis fusion ; les restes (batailles trop faciles, mort du roi) vont au lot EQ.
