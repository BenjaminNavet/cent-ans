# 0108 — Plafond d'opinion par motif et ordre de démolition (lot RS-C)

Date : 2026-09-28. Statut : accepté.
Suivi : `docs/wip/rs-c-diplo.md`. Numéro pris après 0107 (0104-0106 réservés au lot GA).

## Contexte
- Les modificateurs d'opinion s'empilaient sans limite : dix mariages entre deux maisons donnaient
  +150 (« Mariage entre nos maisons », +15 chacun pendant 80 tours) ; six ambassades de héraut
  étaient vues en même temps dans les traces (`docs/wip/eq6-guerre-toutes-difficultes.md`). EQ6
  neutralise la parenté pour la seule guerre de prétention (`war.claim_war_ignores_kinship`).
- Aucun ordre ne permettait de raser un bâtiment : les Suisses gardaient des bâtiments construits
  avant le plafond d'entretien de l'IA (EQ5, 30 % du revenu brut) et faisaient banqueroute
  (2,0 par décennie sur une graine difficile, `docs/wip/eq5-ia-banqueroutes-intrusions.md`).

## Décision
1. **Plafond par motif en données** : `data/rules/diplomacy.json` (schéma `diplomacy_rules`,
   `DiplomacyRules`) `opinion_caps`, table motif → plafond (valeur absolue du total des
   modificateurs en cours qu'une faction garde envers une autre pour ce motif). Motifs connus
   (`OpinionMotive`, enum du schéma) : `marriage` 30, `herald_embassy` 20 ; `gift` et `treaty`
   possibles, non plafonnés aujourd'hui. Un motif inconnu fait échouer le chargement.
2. **Plafond appliqué à l'ajout** (`CampaignState::add_capped_modifier`) : le nouveau modificateur
   est rogné à ce qui reste sous le plafond ; s'il ne reste rien, il prolonge seulement les
   modificateurs en cours jusqu'à sa propre échéance. Pas de plafond à la lecture : les anciennes
   sauvegardes expirent d'elles-mêmes et la règle EQ6 (qui retire toute la parenté de l'attitude
   pour la guerre de prétention) n'est pas touchée. Branché sur le mariage, les deux missions du
   héraut (ambassade, trêve), les présents et les traités signés.
3. **`Order::Demolish { settlement, building }`** (`{"type": "demolish", ...}` au pont GDExtension,
   sans UI dans ce lot) : rase le bâtiment achevé, rend `economy.json` `demolition_refund_percent`
   (10 %) de son coût en argent. Refusé sous siège, hors de ses colonies, ou si un autre bâtiment
   de la place (ou son chantier) en dépend (`buildings::demolition_blocker`). Une amélioration ayant
   remplacé son niveau inférieur, raser une foire ne rend pas la maison des métiers.
4. **Déficit prolongé** : `FactionState::deficit_seasons` compte les saisons d'affilée closes avec
   un revenu inférieur à l'entretien (`resolve_economy`).
5. **IA** (`economy.json` `ai_demolition`) : après 4 saisons de déficit, tant que l'entretien des
   bâtiments qui ne se paient pas eux-mêmes dépasse 40 % du revenu brut, elle rase le moins utile
   par livre d'entretien, un par saison au plus. L'écart avec le plafond de construction (30 %)
   évite de bâtir et raser en alternance.

## Conséquences
- Mesures avant/après : voir `docs/wip/rs-c-diplo.md`.
- Le joueur a la commande au pont ; un bouton « Raser » dans l'onglet de la colonie reste à faire.
- Un nouveau motif plafonnable demande une entrée d'`OpinionMotive` et son libellé dans
  `diplomacy::opinion_motive`.
