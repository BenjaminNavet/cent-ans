# Revue de code globale (2026-09-26)

État : vague 1 lancée — 6 relecteurs en lecture seule (pas de build).
Tranches :
1. sim-campaign A : diplomacy, negotiation, agents, chronicle, dynasty, naval
2. sim-campaign B : orders, movement, state, battle_auto, battle_request, reste
3. sim-battle
4. ai, data-model, vegetation, godot-bridge
5. GDScript battle/ + map/
6. GDScript ui/, sim/, audio/, naval/ + tools/ Python

Prochaine étape : trier les constats, corriger les confirmés sur la branche `fix/code-review`
(worktree ../gp-review), tests, ff dans main.

## Corrections sim-campaign

Branche `fix/code-review`. Tests de régression : `core/crates/sim-campaign/src/review_tests.rs`.
Prochaine étape : constats 2 à 18 dans l'ordre (voir liste de la tranche 1-2).

| n° | statut | note |
|----|--------|------|
| 1 | corrigé | `movement::assign_captor` extrait ; appelé après un assaut (assaillant pris → défenseur) et une sortie (assiégeant pris → garnison) ; le gouverneur à la tête d'une garnison pris (`general_captured` du côté garnison) passe par `chronicle::capture_character`. |
| 6 | corrigé | la sortie affronte toute la coalition assiégeante (`settlement_coalition`, forces sommées) ; le siège n'est levé que s'il ne reste aucun assiégeant. |
