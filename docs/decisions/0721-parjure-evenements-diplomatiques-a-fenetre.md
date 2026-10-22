# 0721 — Parjure : événements diplomatiques en table, à fenêtre (lot M6)

Statut : accepté.

## Contexte
Fiche M6 (`rea-tww3/docs/wip/integration-cent-ans.md`). Le parjure existe en dur dans `diplomacy/war.rs` (`PERJURY_PRESTIGE`, -40 sur 40 tours, `PERJURY_REASON`) pour la seule rupture de trêve ou de pacte. TWW3 pilote des événements diplomatiques nommés (gravité MAJOR / MODERATE, paires A puis B dans `max_turn_difference` tours) et non des scores.

## Décision
- Table `events[]` dans `data/rules/diplomacy.json` (schéma `diplomacy_rules`) : `id`, `severity` (major / moderate), `reason_fr`, `opinion`, `duration`, `prestige`, `chivalry_prestige`, `window` (saisons, si pertinent). Aucun nouveau score : l'événement écrit un modificateur d'opinion existant chez tous les seigneurs.
- Événements : `truce_broken` (trêve de moins de `window` saisons rompue), `ally_attacked` (attaque d'un allié sous serment), `captive_executed_under_parole` (captif sous parole / sauf-conduit exécuté, lien ADR 0303).
- Valeurs de départ : fenêtre 6 saisons ; majeur -25 sur 20 tours, modéré -10 sur 10 tours.
- Module `sim-campaign/src/perjury.rs` : `record_event`, `preview` (coût chiffré lu par `treaty_explain` et l'IA, `ai/src/diplomacy_eval.rs`).
- L'IA évalue le coût avant d'agir.

## Conséquences
À compléter à l'implémentation (impact sur `wr_ai_diplo`).
