# NNNN — Domaine du seigneur : un revenu fixe pour toute faction qui tient sa capitale (lot LR-04)

Date : 2026-10-03. Statut : accepté. Mesures : `docs/wip/lr-04.md`. Suite des ADR 0114 (FE8) et
0117 (garde du seigneur), point ouvert de `docs/wip/fe.md` et `docs/wip/fe8-equilibre.md`.

## Contexte

Les comtés d'une province (royaumes irlandais, Îles, Luna, Urbino, et une trentaine d'autres sur
la carte OM) lèvent 50-100 livres d'impôt par saison. Leurs bâtiments de départ en coûtent déjà
70-125 (le château du siège à 50, le port, le champ de montre, les châteaux secondaires) : même
avec la garde du seigneur gratuite (ADR 0117), il ne reste rien pour une seule unité de garnison
(milice urbaine payée au taux de la cité, 50 livres). Sur 50 tours × 5 graines, Luna est la pire
faction de la graine 1 (16 tours de trésor négatif sur 50).

Écarté :
- **Relever population ou richesse** de ces provinces : populations estimées et relues (lot R4),
  et le problème est structurel (coûts absolus contre impôt proportionnel), comme l'ADR 0117 l'a
  établi ; ce serait un cas particulier par province.
- **Baisser l'entretien des bâtiments de la cité** : changerait l'économie de tous les royaumes
  (40 cités pour la France).
- **Étendre `starting_budget` (ADR 0165) aux petites factions** : la règle renvoie des unités, elle
  ne crée pas de quoi en payer une ; et la capitale en est exclue.
- **Plancher de revenu** (porter l'impôt à un minimum) : supprime l'intérêt marginal de l'impôt
  sous le plancher.

## Décision

`data/rules/economy.json` § `domain_income` : toute faction qui tient la cité de sa capitale, non
assiégée, perçoit chaque saison ce revenu fixe de son domaine propre (cens, péages, moulins,
droits seigneuriaux), hors barème d'impôt et hors embargo, avant le coefficient de difficulté
(`CampaignState::faction_domain_income`, ajouté dans `faction_income_effective_walk`). Réglage :
**150 livres**. Absent ou 0 : comportement antérieur. Une faction sans cité (croisés, exilés)
n'en a pas.

Valeur : la plus petite somme ronde qui laisse aux huit comtés cités, une fois bâtiments et cour
payés, de quoi entretenir une milice urbaine dans leur cité (test
`sim-campaign/tests/lr04_domain_income.rs`). À 100, les Îles (123 livres de bâtiments) restaient
sous la barre.

## Conséquences

- Revenu des comtés pauvres multiplié par 2 à 4 ; +0,5 % pour la France, +0,8 % pour l'Angleterre.
- Les factions moyennes (200-400 livres) gagnent 40-75 % : à surveiller pour le point « aucune
  mineure n'explose » (ADR 0114).
- Mesures avant/après (century_probe, m3) : `docs/wip/lr-04.md`.
