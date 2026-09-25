# EP9 — Batailles décisives

Lot : les batailles de campagne doivent se décider d'elles-mêmes (recette Q3, point 12).
Branche : `worktree-agent-a6207325088470391` (worktree agent). ADR visé : 0056.

## État

- [x] Squelette : sondes `core/crates/sim-battle/tests/ep9_decisive.rs` (ignorées).
- [ ] Mesure « avant » (12 graines, 3 paliers, plaine / collines / rivière, IA-IA, joueur attaquant immobile, joueur défenseur immobile).
- [ ] Correctif : attaquant qui attaque toujours, bataille refusée, déroutes qui vont au bout.
- [ ] ADR 0056, tests activés, smoke.

## Mesures

Bataille de démo 1337 (France attaquante = joueur, sans ordre) : 744 s, victoire anglaise
(l'IA anglaise ne sort qu'après `DEFENDER_PATIENCE` = 480 s ; la recette Q3 s'est arrêtée à 7 min 14 s).

## Prochaine étape

Lancer `survey` et lire les durées.
