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

## Constats reçus
- Tranche 1 (sim-campaign A) : 12 constats — otages de traité libérables (Hold), enfants de mariage
  inter-factions dans la faction de la mère, apply_treaty partiel (mariage), malus otages mal attribué,
  héraut ignore Hold / reste bloqué, ordre des buts de guerre, clone du state dans evaluate(Marriage),
  tribut d'une faction morte, cogues louées coulées, blocus compté double, doublons via JSON.
