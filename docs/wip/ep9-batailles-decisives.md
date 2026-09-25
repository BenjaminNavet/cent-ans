# EP9 — Batailles décisives

Lot : les batailles de campagne doivent se décider d'elles-mêmes (recette Q3, point 12).
Branche : `worktree-agent-a6207325088470391` (worktree agent). ADR visé : 0056.

## État

- [x] Squelette : sondes `core/crates/sim-battle/tests/ep9_decisive.rs` (`survey`, `survey_crecy`, `q3_demo_battle_without_orders_ends`, `trace_demo`).
- [x] Mesure « avant » (12 graines, 3 paliers).
- [x] Règles `data/rules/battle_decision.json` (+ schéma, pytest) ; `src/decision.rs`, `src/sim/decision.rs` :
  armée brisée (part en état de combattre < 35 %, 45 % sans général), bataille refusée / accalmie
  (300 s sans perte, mêlée ni approche de 20 m), `BattleOutcome::end`.
- [x] IA : le défenseur reste sur sa position tant que personne ne combat (`DEFENDER_QUIET` 60 s) ;
  l'attaquant ne tient le duel d'archers que 180 s (`ATTACKER_DUEL_LIMIT`).
- [ ] Cas longs restants : rivière (IdleAttacker large 983 s, IdleDefender épique 1426 s).
- [ ] Tests activés, ep1_scale seuil 12 → 20, digests, campagne (chronique « bataille refusée »), UI résultat, smoke.
- [ ] ADR 0056.

## Mesures (durée min/méd/max en s, 12 graines)

Avant : standard IA-IA 420/486/627 ; joueur immobile jusqu'à 1800 (plafond) avec rivière.
Large et épique : joueur immobile (attaquant ou défenseur) = presque toujours non terminée à 30 min.
Épique IA-IA plaine 493/971/1800.

Après (commit courant) : tout se termine ; IA-IA 3-9 min ; joueur attaquant immobile sans rivière
300-622 s (refusée ou brisée) ; rivière encore jusqu'à 983 / 1426 s sur quelques graines.
Crécy-like (IA-IA, Anglais sur la crête) : Anglais 11/12 avant et après.
Démo 1337 (Q3) : 580-688 s au lieu de 744-911 s.

## Prochaine étape

Graines longues avec rivière, puis activer les tests.
