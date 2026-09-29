# ADR 0122 — Une décision expirée applique l'option de l'IA

Date : 2026-09-29. Statut : accepté. Chantier FK (`docs/design/2026-09-29-carte-vivante-folk.md`
§ 2.1.2, note `docs/wip/fk.md`).

## Contexte

Depuis M10, une décision du joueur non tranchée à l'échéance applique sa **première** option
(`chronicle.rs`, `resolve_chronicle`). Tant que les décisions s'ouvraient en fenêtre modale en début
de tour, le cas était rare. Avec FK, les événements `random` de province deviennent des incidents
posés sur la carte, qu'on peut laisser courir : l'expiration devient fréquente, et la première
option, souvent la plus coûteuse ou la plus « héroïque », n'est pas un défaut raisonnable (un petit
royaume paierait une dépense qu'il ne peut pas couvrir).

## Décision

- À l'échéance, toute décision en attente du joueur applique l'option que l'IA choisirait pour
  ce royaume : `ai_affordable_choice` (plus fort `ai_weight` parmi les options abordables, égalités
  départagées par le générateur de la campagne), restreinte aux options offertes par la décision.
- La règle vaut pour **toutes** les décisions (fenêtre ou carte), pas seulement les incidents.
- La chronique garde la mention « (délai écoulé) » ; l'interface notifie l'option retenue.

## Conséquences

- Laisser expirer n'est plus une pénalité arbitraire : c'est déléguer au conseil du royaume.
- L'expiration consomme un tirage du générateur quand l'IA départage des égalités : les parties
  rejouées avec une graine donnée divergent de celles d'avant FK (déterminisme conservé à graine
  et version égales).
- Tests : `core/crates/sim-campaign/tests/fk_map_scenes.rs` (`expired_decision_applies_ai_option`).
