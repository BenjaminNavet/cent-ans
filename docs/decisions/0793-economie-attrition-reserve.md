# 0793 — ECO : économie resserrée, attrition hors territoire, réserve de recrutement

Statut : accepté (29/10/2026) ; seuil de victoire revu par l’ADR XXXX (43). Numéro à fixer à la fusion.

## Contexte
Le juge de l'itération 1 (`out/juge2/iter-01-design.md`) et les seed bots (ADR 0348) ont relevé :
1. France au départ : 28 000 livres, +14 382 net par saison, 43 888 après un tour sans dépense. Le vrai goulot est la
   réserve de recrutement (ADR 0102), que l'interface ne montrait pas.
2. Une armée vivait 5 saisons hors territoire ami sans perdre un homme (20 points de vivres perdus par saison sur 100).
3. Les bots « raisonnables » recrutaient et bâtissaient sans regarder la bourse : jusqu'à 89 banqueroutes par partie.
4. Les bandes de dérive étaient trop larges (min/max observés ± 25 %) et ne détectaient rien.
5. Invariant `army_refs` : une bataille en attente nommait une armée détruite ou fusionnée entre-temps.

Constats vérifiés par rejeu (économie, bots) ; le rejeu exact du constat 5 n'est pas possible (la sauvegarde du juge
contenait un ordre `levy_extraordinary` absent de `main`), la cause est établie par lecture : usure, désertion,
dissolution de compagnie et fusion peuvent retirer une armée citée par `pending_battles`.

## Décision
- **army_refs** : `battle_request::purge_stale_pending` retire en fin de saison les batailles dont une armée ou la colonie
  n'existe plus ; `MergeArmies` et `GarrisonUnits` (qui vide l'armée) sont refusés (`WouldEmptyArmy`) pour une armée citée
  par une bataille en attente. Test de régression `destroyed_army_leaves_no_pending_battle`.
- **Économie** (`data/rules/economy.json`, `finance.json`, `fac_france.json`) : trésor de départ de la France 28 000 → 16 000 ;
  `tax_efficiency` 0,082 → 0,070 ; administration par province 0,007 → 0,010 (plafond 0,32) ; plafond de la maison du roi 40 % → 32 %. Les petits comtés (peu de provinces) paient peu de plus ; les grands royaumes plus.
- **Attrition** : `supply_loss` 20 → 30, `supply_loss_winter` 35 → 45 (une armée sans ravitaillement s'épuise en 3 saisons,
  2 l'hiver) et nouvelle perte de maladie/désertion `attrition_percent` 2 % par saison, `attrition_percent_winter` 5 %,
  appliquée à chaque unité hors territoire ami, indépendamment des vivres (schéma `economy_rules`).
  `upkeep_months_per_season = 4` est conservé : le 4e mois couvre soldes en retard, équipement et suite ; documenté ici.
- **Réserve de recrutement visible** : `sim_campaign::recruit_pool::RecruitReserve` et
  `game_api::hud::recruit_reserve(data, state, faction)` (champs `settlements`, `available`, `cap`, `ready_settlements`,
  `gain_next_season`). Le client l'affiche (lot TEXTES).
- **Bots** : `prudent_candidates` (aggressor, builder, diplomat, sailor, turtle) n'offre plus recrutement ni chantier tant que la
  bourse est sous une réserve (1 000 livres, ou 8 saisons de déficit + 2 000 si le solde projeté est négatif). `chaos` et
  `random-legal` restent naïfs pour tester les refus.
- **Bandes de dérive** : resserrées sur une cible de design, y compris `provinces_held_end`, `treasury_end`, `reign_score`.
- **Victoire** : `victory_min_score` 72 → 50. Mesure (seed bots, 150 tours) : la meilleure politique, l'IA à la place du
  joueur, plafonne à 14-25 ; un passif à 11. Le seuil de 72 n'était atteint par aucune politique. À 50, il faut environ
  trois fois le score de l'IA : atteignable par un joueur qui remplit objectifs et pressions des actes, pas par un passif.
  Le calibrage final demande une partie humaine (le score des bots est mesuré en milieu d'acte).
- **Ajustements d'économie** (révision après mesure) : la maison du roi garde ses paliers mais son plafond passe de 40 % à 32 %
  du revenu (à 40 % elle absorbait presque tout le revenu d'un royaume réduit à 7 provinces, cause du déficit structurel des
  bots passifs). Les bots prudents répondent aussi au déficit (taxe haute, une unité licenciée par saison).

## Conséquences
- Les tests à graine (f7_events 1341, ev_history) peuvent bouger ; ajustés avec justification dans le commit.
- Toute autre faction voit son revenu baisser de 15 % (`tax_efficiency`) ; les hoards sont déjà frappés par l'opulence.
- Les chiffres avant/après des seed bots sont dans `docs/wip/lot-eco.md`.
