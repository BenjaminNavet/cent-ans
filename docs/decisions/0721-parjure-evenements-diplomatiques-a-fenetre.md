# 0721 — Parjure : événements diplomatiques en table, à fenêtre (lot M6)

Statut : accepté.

## Contexte
Fiche M6 (`rea-tww3/docs/wip/integration-cent-ans.md`). Le parjure existe en dur dans `diplomacy/war.rs` (`PERJURY_PRESTIGE`, -40 sur 40 tours, `PERJURY_REASON`) pour la seule rupture de trêve ou de pacte. TWW3 pilote des événements diplomatiques nommés (gravité MAJOR / MODERATE, paires A puis B dans `max_turn_difference` tours) et non des scores.

## Décision
- Table `events[]` + `truce_window` dans `data/rules/diplomacy.json` (schéma `diplomacy_rules`, `DiplomacyEvent` dans data-model) : `id`, `severity` (major / moderate), `reason_fr`, `opinion`, `duration` (tours = saisons), `prestige`, `chivalry_prestige` (perte en plus si l'ordre de chevalerie du fautif tient, `chivalry.rs`). Aucun nouveau score : l'événement écrit le modificateur d'opinion existant chez tous les autres seigneurs (`perjury::record_event`), plus le prestige du souverain.
- Événements : `truce_broken_recent` (majeur, trêve signée depuis moins de 6 saisons : -25 sur 20 tours, prestige -30), `truce_broken` (modéré : -10 sur 10 tours, prestige -15), `pact_broken` (modéré, prestige -30), `ally_attacked` (majeur : déclarer la guerre à un allié, vassal ou suzerain), `captive_executed_on_parole` (majeur : exécuter un captif dont les termes sont la parole, lien ADR 0303).
- Âge d'une trêve : l'état ne garde que la fin ; l'âge se déduit de la durée standard `TRUCE_TURNS` (20). Une trêve de durée non standard est donc approchée.
- Les raisons de tous ces événements commencent par « Parjure » (`perjury::is_perjury_reason`) : réputation de cour, pas rancune personnelle (alignement IA, contexte de négociation).
- Aperçu : `declaration_prestige_cost`, aperçu de guerre du pont (`game-api/diplomacy.rs`) et `ExecutionPreview.perjury_*` lisent la table. IA : `plan_diplomacy` s'abstient de toute déclaration qui commettrait un parjure (`perjury_event_for_war`).

## Conséquences
- Les anciennes constantes `PERJURY_PRESTIGE` et le -40/40 tours en dur disparaissent : une rupture de trêve récente coûte -25 (au lieu de -40) sur 20 tours (au lieu de 40) ; l'ancienne trêve est plus clémente (-10). Saves : aucun champ sérialisé nouveau.
- Attaquer un vassal ou un suzerain sans trêve passe de « agression sans motif » (-20) à `ally_attacked` (-25).
- Non fait : coût de parjure dans l'infobulle d'exécution du client (`ExecutionView`), `treaty_explain.rs` (traités, pas guerres), sauf-conduit (inexistant en Rust : seule la parole existe).
