# 0332 — Marchand à comptoir et objectifs de victoire étendus (courte/longue)

Statut : accepté (lot TW `misc-camp`, rapport `docs/wip/tw/campagne-m2.md` § 3, top 8 et top 10).

## Contexte
Medieval II a un marchand qui s'installe sur une ressource et en tire un revenu ; ici le commerce se règle seul
(routes, accords). Côté victoire, seules quatre factions ont des objectifs propres ; les autres reçoivent des
chemins féodaux génériques et aucun objectif affiché ; il n'y a ni choix de durée, ni condition de prestige, de
trésor ou de titre.

## Décision
- **Marchand.** `AgentKind::Merchant` (300 livres, entretien 15, 2 au plus, recruté en cité ou ville) et deux
  actions de la table `data/rules/agents.json` : `trade_post` (ouvre un comptoir dans la colonie où il se tient,
  80 livres) et `outbid` (rachète le comptoir d'un marchand rival présent, 200 livres, 10 % de risque). Nouvelles
  conditions `post_free` / `rival_post`, nouveaux effets `trade_post` / `outbid`. L'état est `Agent.post`
  (`serde(default)`), perdu dès que le marchand quitte la place.
- **Revenu.** Chaque saison (`agents/merchants.rs`, avant l'entretien) : base du type de colonie (`merchant.income` :
  cité 40, ville 24) + 4 par ressource de la province, +25 % par sceau au-dessus du premier, fois le niveau des
  prix ; 60 % s'il y a un rival posté au même lieu ; 20 % vont au maître d'une colonie étrangère. Le total payé est
  consigné dans `AgentsState.merchant_income_last_turn`.
- **Chasse.** Un comptoir en terre de gens avec qui l'on est en guerre est fermé en fin de saison (journal pour le
  joueur) ; un rival le rachète par `outbid`. Les alliés et soi-même ne sont pas des rivaux.
- **IA.** `ai_merchant` : racheter un comptoir rival (chance ≥ 40 %), sinon ouvrir le sien, sinon marcher vers la
  cité amie ou neutre la plus proche sans comptoir. Recrutement avec les autres agents (une puissance mineure n'a
  qu'un espion).
- **Objectifs.** `ObjectiveCondition` gagne `province_count`, `province_growth` (relatif au nombre de provinces
  du premier tour, `CampaignState.victory_start_provinces`), `prestige`, `treasury`, `hold_title`. Un objectif a une
  `scope` (`always` par défaut, `long_only`) ; un bloc `victory` peut porter `short_end_year`. Le joueur choisit
  la durée (`CampaignState.victory_length`, `CampaignSim.set_victory_length("short"|"long")`) : la campagne courte
  ne compte que les objectifs `always`, finit à `short_end_year` (1380 par défaut) et tient les objectifs au plus
  `short_hold_turns` (4) saisons.
- **Défaut générique.** Une faction sans bloc `victory` reçoit `feudal.json` `victory.generic_objectives` (trois
  provinces gagnées, 15 000 livres, indépendance ; en campagne longue huit provinces et prestige 150) tenus
  `generic_hold_turns` (4) saisons, et seulement à partir de `generic_victory_min_year`. Les quatre factions
  historiques gardent leurs objectifs (une partie marquée `long_only` pour la campagne courte) ; neuf factions
  gagnent un bloc : Moscou, Ottomans, Hongrie, Pologne, Lituanie, Ordre teutonique, Byzance, Mamelouks, Sicile.
- **Affichage.** Aucun écran : `get_objectives` renvoie déjà la liste filtrée par durée et la progression de chaque
  condition ; le panneau de victoire existant l'affiche.

## Conséquences
- Un nouveau champ ou une nouvelle variante reste `serde(default)` : les sauvegardes existantes se chargent.
- Les agents de l'IA coûtent désormais un marchand de plus (entretien 15), compensé par le revenu des comptoirs ;
  mesure avant/après dans `docs/wip/tw/misc-camp.md`.
- Le choix court/long n'a pas d'interface de départ : le pont l'expose, le branchement dans l'écran de nouvelle
  partie reste à faire (hors périmètre « pas de nouvel écran »).
- Une condition de prestige dépend de l'échelle du prestige (0 au départ, plusieurs centaines après quarante
  saisons victorieuses) : à ajuster à l'usage.
