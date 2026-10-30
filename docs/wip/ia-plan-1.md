# IA-PLAN-1 — planificateur par divisions, plan « Tenir et recevoir »

Spec : `docs/audits/2026-10-29-bataille.md` (d) IA-PLAN-1. ADR : `docs/decisions/0809-ia-plan-planificateur.md`
(numéro à la fusion). Branche : worktree-agent-a4262fad3109d43b2. Rien de poussé ni fusionné.

## État : terminé (hors fusion)
- [x] `core/crates/sim-battle/src/ai/planner/{mod,plans,roles,division}.rs`.
- [x] `data/rules/battle_plans.json` + `data/schemas/battle_plans.schema.json` ; `battle_ai.json` `planner` (défaut `legacy`).
- [x] `BattleEvent.kind = Some(BattleEventKind::RefusalLikely)` (`sim/decision.rs::warn_refusal`, `battle_decision.json`
      `refusal_warning_share` 0,25) : annoncé à 142-235 s en `passive_attacker` (contre le refus à ≈ 1 005 s).
- [x] Assertion debug `obeys_division_targets` (et correction des ordres : `retarget`).
- [x] Tests : conditions, phases, divisions (unitaires) ; `tests/ai_tactics/ia_plan1.rs` (refus annoncé < 300 s, bataille
      complète, déterminisme). `fmt`, `clippy -D warnings`, `cargo test --workspace` verts.
- [x] Banc : `battle_bench --planner legacy|plans --seeds N` ; `median_refusal_likely_s` en sortie.

## Banc, 20 graines, `legacy` contre `plans` (victoires du défenseur, lignes d'armées)
| mode | legacy | plans |
|---|---|---|
| ai_vs_ai | 65/120 | 74/120 |
| passive_attacker | 90/120 | 91/120 |
| blob | 114/120 | 113/120 |
| flank | 59/120 | 71/120 |
| total | 328/480 | 349/480 |

Pertes médianes du défenseur (moyenne des lignes) : 0,164 contre 0,165. Cartes historiques (ai_vs_ai, 20 graines, victoires
de l'attaquant de la carte) : Crécy 14 -> 9, Poitiers 12 -> 19, Azincourt 20 -> 20, Formigny 17 -> 17, Castillon 2 -> 0,
Auray 20 -> 18 (Crécy : pertes du défenseur 0,36 -> 0,46, point ouvert). Sorties : `out/bench` non versionnées (scratchpad).

Expériences (armées, défenseur, total sur 480) : posture tenue seule 349 ; discipline de cible seule (cible = barycentre de
la division) 316 ; posture tenue + discipline, cible « ennemi le plus proche d'un régiment » : 349 (portée de report 60 m :
328, 140 m : 349, 300 m : 338).

## Écarts à la spec
- `BattleEvent::RefusalLikely` : `BattleEvent` est une structure (ligne de journal) ; ajout d'un champ typé `kind`.
  L'annonce vient du simulateur (pas du planificateur) et sert donc aussi `legacy`.
- Le plan ne couvre que le défenseur (un attaquant qui « tient » bloquerait la bataille) ; l'attaquant garde `plan_field`.
- Le placement (terrain, ligne, tireurs, cavalerie) est encore `plan_field`, appelé sous une `Posture` ; le rôle `shoot`
  est déclaratif, `hold` / `reserve` agissent sur le choix de la réserve.

## Points ouverts pour IA-PLAN-2
- Crécy et Castillon se dégradent sous `plans` (défenseur) : à regarder avec le banc avant d'étendre.
- 27 duels sans contact et défenseur inactif qui perd toujours restent des constats du banc, non traités ici.
- La discipline de cible fait tenir (Halt) les régiments dont la cible de division est trop loin : à valider pour la
  cavalerie quand `strike` (charge groupée, `ai/mass.rs`) arrivera.
- Phases : pas encore de journal « changement de phase » (IA-PLAN-3) ; la mémoire (`PlannerMemo`) expose divisions et phase.
