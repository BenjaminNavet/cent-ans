# JR — Croisés (orchestration)

Spec `docs/superpowers/specs/2026-10-02-jr-croises-jerusalem-design.md`, ADR 0165.
Worktree d'intégration `../gp-jr` (`feat/jr`). Lancé le 2026-10-02, autonomie totale
(enchaîner les lots, relire, fusionner dans main sans jalon de validation). Budget 0 $.

## Lots
- JR0 squelette : spec, ADR, note, `crusade.json` + schéma, modules vides, tests désactivés.
- JR1 règle (cent-ans-dev) : `sim-campaign/src/crusade.rs`, crochets, fin de tour, sauvegarde, tests.
- JR2 données (cent-ans-mech) : faction, chef, Limassol, armée et flotte de départ, relations
  réciproques, objectifs, codex, armes, tests de comptage.
- JR3 pont + encart Godot (cent-ans-dev) : vue, commande, encart « Ferveur », toasts, test headless.
- JR4 IA + équilibre (cent-ans-dev) : prêche automatique, débarquement, sonde 5 graines × 50 tours.
- JR5 relecture, capture de contrôle, fusion dans main.

Ordre : JR0 → (JR1 ∥ JR2) → (JR3 ∥ JR4) → JR5.

## État
- Spec, ADR, note écrits. Exploration des points d'intégration en cours.

## Prochaine étape
- JR0 : squelette, puis lancer JR1 et JR2.

## Points ouverts
- L'IA commune sait-elle débarquer ? (sinon objectif outre-mer explicite en JR4)
- Capitale dans une province dont la cité est à un autre : acceptée par les validateurs ?
