# ADR 0178 — Domaine du seigneur, évènements proportionnels, garnison de la capitale hors coût fixe (lot LR-04)

Date : 2026-10-03. Statut : accepté. Mesures : `docs/wip/lr-04.md`. Suite des ADR 0114 (FE8) et
0117 (garde du seigneur), point ouvert de `docs/wip/fe.md` et `docs/wip/fe8-equilibre.md`.

## Contexte

Les comtés d'une province (royaumes irlandais, Îles, Luna, Urbino, et une trentaine d'autres sur
la carte OM) lèvent 50-100 livres d'impôt par saison. Leurs bâtiments de départ en coûtent déjà
70-125 (le château du siège à 50, le port, le champ de montre, les châteaux secondaires) : même
avec la garde du seigneur gratuite (ADR 0117), il ne reste rien pour une seule unité de garnison
(milice urbaine payée au taux de la cité, 50 livres).

Deux autres causes sont apparues à la mesure, dès que ces comtés ont eu de quoi payer :
- **Évènements de chronique** : une faction sous le revenu de référence (4 000) paie un évènement en
  proportion de son revenu, mais jamais moins de 25 % de la somme écrite. Un comté de 200 livres
  payait un incendie à 2 000 ₶ 500 ₶, deux saisons et demie de revenu, à peu près un tour sur deux
  (trace `ECON_TRACE=fac_connacht`, graine 3) : banqueroute à répétition.
- **Point fixe des garnisons de l'IA** (lot C7a) : la cible d'entretien du recrutement compte les
  garnisons « en sus » (coût fixe) puis une part du reste. Les recrues d'un comté restent dans la
  garnison de sa cité capitale : chaque recrue relevait la cible d'autant qu'elle coûtait, et la
  garnison grossissait jusqu'à manger tout le revenu net (Urbino : 206 d'entretien pour 208).

Écarté :
- **Relever population ou richesse** de ces provinces : populations estimées et relues (lot R4), et
  le problème est structurel (coûts absolus contre impôt proportionnel, ADR 0117) ; ce serait un cas
  particulier par province.
- **Baisser l'entretien des bâtiments de la cité** : changerait l'économie de tous les royaumes.
- **Étendre `starting_budget` (ADR 0165) aux petites factions** : la règle renvoie des unités, elle
  ne crée pas de quoi en payer une ; et la capitale en est exclue.
- **Plancher de revenu** (porter l'impôt à un minimum) : supprime l'intérêt marginal de l'impôt.

## Décision

1. **Domaine du seigneur** (`data/rules/economy.json` § `domain_income`, 150 livres) : toute faction
   qui tient la cité de sa capitale, non assiégée, perçoit ce revenu fixe de son domaine propre
   (cens, péages, moulins, droits), hors barème d'impôt et hors embargo, avant le coefficient de
   difficulté (`CampaignState::faction_domain_income`, dans `faction_income_effective_walk`).
   À 0 : comportement antérieur. Une faction sans cité (croisés, exilés) n'en a pas.
   Valeur : la plus petite somme ronde qui laisse aux huit comtés cités, bâtiments et cour payés,
   de quoi entretenir une milice urbaine dans leur cité (test
   `sim-campaign/tests/lr04_domain_income.rs`) ; à 100, les Îles (123 livres de bâtiments)
   restaient sous la barre.
2. **Part minimale des évènements** (`event_treasury_min_scale`) 0,25 → 0,05 : le domaine assure
   désormais un revenu plancher, le plancher d'échelle n'a plus de raison d'être aussi haut ; toute
   faction sous le revenu de référence paie un évènement en proportion de son revenu.
3. **IA, recrutement** (`ai::campaign`, cible d'entretien) : la garnison de la cité capitale, où
   se rassemblent les recrues, ne compte plus dans le coût fixe des garnisons ; celles des autres
   places restent un coût fixe (raison du lot C7a). La part de guerre ou de paix s'applique donc à
   ce que bâtiments et garnisons des autres places laissent, et la marge d'épargne demeure.

## Conséquences

- Revenu des comtés pauvres multiplié par 2 à 4 ; +0,5 % pour la France, +0,8 % pour l'Angleterre.
- `century_probe` 50 tours × 5 graines : banqueroutes 1,57 → 0,15 par faction et par décennie,
  petites factions 1,74 → 0,04, 28 factions d'avant FE 0,18 → 0,17 ; tours de banqueroute des huit
  comtés 81 → 13 ; guerre FR-EN 68 % [40-94] → 64 % [44-84] ; plus grande mineure 3-4 provinces,
  inchangé ; factions détruites 17,2 → 17,0.
- Les évènements pèsent moins sur les petites factions : un incendie coûte une demi-saison à
  toute faction sous le revenu de référence. Les gains d'évènement baissent d'autant.
- Les factions moyennes (200-400 livres) gagnent 40-75 % de revenu : à surveiller pour le point
  « aucune mineure n'explose » (ADR 0114) sur une partie séculaire.
