# ADR 0350 — Faillites : réserve de trésor de l'IA, ventes forcées annulées, plafond des coûts d'événements

Statut : accepté (13/10, enquête `docs/wip/faillites-enquete.md`).

## Contexte
L'enquête a montré que 92 % des entrées en découvert des factions IA venaient de coûts d'événements de la chronique choisis par l'IA
(règle d'abordabilité `trésor + 2 × revenu`) et que les prix forcés de ventes (`pay_sale_price`) pouvaient mettre le payeur à −5 000 ₶.

## Décision
1. **Réserve IA** : une option d'événement est abordable pour l'IA si son coût ≤ trésor − `ai_event_reserve_seasons` × dernier revenu
   (donnée de `data/rules/economy.json`, 1,0 saison au départ). Si aucune option n'est abordable, l'IA prend la moins chère.
2. **Vente forcée** : si le payeur n'a pas le prix en trésor, la vente (province ou titre) est annulée, avec un message de chronique.
   Choisi contre l'échelonnement par `ledger.tributes` : un tribut exige un bénéficiaire vivant (le prix « quitte la carte » quand le vendeur
   disparaît ou n'est pas jouable), est supprimé en cas de guerre avec lui et peut lui-même creuser un découvert ; l'annulation est le plus simple et ne
   touche pas à la diplomatie. Contrepartie : un événement historique de vente peut ne pas avoir lieu si l'acheteur est à sec (il reste planifié par ses
   propres règles de déclenchement).
3. **Plafond** : un coût d'événement ne dépasse pas `event_treasury_cap_seasons` (1,5) saisons de revenu ; revenu de référence 4000 → 2500.

## Conséquences
Les découverts créés par les événements disparaissent presque ; restent ceux de la phase diplomatie (indemnités de traités) et les dépenses d'ordres de l'IA
(propositions 4 et 5 de l'enquête non appliquées). Les trois valeurs sont des données réglables.
