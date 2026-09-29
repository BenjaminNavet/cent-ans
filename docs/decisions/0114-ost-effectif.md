# 0114 — Ost effectif, lien féodal sans alliance, commise de Guyenne (lot FE F8)

Date : 2026-09-28. Statut : accepté. Mesures : `docs/wip/fe8-equilibre.md`. Complète l'ADR 0098
(féodalité) et l'ADR 0110 (IA féodale) ; suite de l'ADR 0113.

## Contexte

Mesure RS (ADR 0113) et F8 (`century_probe`, tableau « FE8 ») sur main 639b7e49 :

- **Ost impérial sans distance.** `summon_host` convoquait tous les vassaux directs déduits de
  l'Empire (38), dont Milan, Gênes, Vérone, Mantoue (`de_jure_liege: tit_empire`), à 300-470 km de
  toute terre française : 34 à 67 réponses à l'ost impérial contre la France en 50 tours, 65 à 130
  tours × faction d'Italiens en guerre contre la France.
- **Suzerain allié à ses vassaux.** L'état initial (et chaque hommage, vassalité M5 ou fief concédé)
  inscrivait le suzerain et le vassal comme alliés : l'Empire avait 38 alliés, donc presque toujours
  le casus belli « défense d'un allié » contre la France (un vassal rhénan en guerre aux côtés de
  l'Angleterre suffit) ; le vassal répondait deux fois (appel aux armes de l'allié, ost du suzerain).
- **Commise de Guyenne jamais prononcée** (0 sur 5 graines) : `plan_commise` comparait la France
  seule à la coalition entière du félon (Angleterre, alliés, vassaux), avec un rapport exigé de 2 ;
  et l'Angleterre, en guerre avec la France dès le tour 1 (prétention au trône, ADR 0085), n'ouvrait
  jamais de cas de félonie (l'alliance avec l'ennemi ne compte qu'en paix).

## Décision

1. **Ost effectif** (`feudal_rules.host`, `feudal::can_serve`) : un vassal direct n'est convoqué que
   s'il a une place à moins de `max_muster_km` (200) de la capitale de son suzerain ou d'une place
   de l'ennemi, et s'il n'est pas indépendant de fait (puissance < `independent_power_ratio` = 0,5
   de celle du suzerain). Non convoqué, il ne refuse rien : pas de félonie. À 0, chaque limite est
   levée (comportement antérieur).
2. **Le lien féodal tient lieu d'alliance** : plus d'alliance entre un suzerain et son vassal
   direct (état initial, hommage, vassalité, fief concédé) ; proposition ou article d'alliance entre
   eux refusés (`FEUDAL_TIE_ALLIANCE`). En contrepartie, la cible d'une déclaration de guerre
   convoque aussi son ost (le vassal doit l'ost quand son suzerain entre en guerre, spec FE § 4.1),
   et les deux camps restent du même côté : `is_allied` (batailles, passage, appels) et la puissance
   d'une coalition (`coalition_members`, `coalition_power`, IA féodale) comptent le suzerain et les
   vassaux directs comme des alliés (lus dans le cache `suzerain`). Seule la liste `allies` change :
   plus de casus belli « défense d'un allié » ni d'appel aux armes d'allié pour le lien féodal.
3. **Rapport de force de la commise** (`ai::feudal::commise_power_ratio`) : coalition du suzerain
   (lui, ses alliés, ses vassaux directs) contre celle du félon, chacune sans l'autre.
4. **Félonie de 1337** (`feudal_rules.start_felonies`) : Édouard III donne asile à Robert d'Artois,
   banni par Philippe VI (motif `harboured_felon`). Le cas est ouvert au début de la campagne
   (fenêtre ordinaire de `felony_window_turns`), ce qui permet la commise de Guyenne de mai 1337.
5. **Banqueroutes des petites factions** (même lot, hors ost) : la colonne de `century_probe` n'a
   pas changé de définition, mais de population (28 factions avant FE, 91 après, dont une trentaine
   d'un seul comté) ; les 28 factions d'avant FE restent à ≈ 0,1 / faction / décennie. Deux causes
   dans les petites : `agents::plan_agents` prenait « jouable » pour « grande puissance » (chaque
   comté entretenait espion, émissaire et prédicateur, ≈ 50 livres par saison pour 60-80 de revenu),
   d'où `agent_rules.ai_network_min_income` (500) ; et la garde de mutation monétaire de l'IA ne
   comptait que les bâtiments parmi les coûts fixes que l'inflation renchérit : les garnisons y
   entrent (Connacht : prix 100 → 360 en 50 tours).

## Conséquences

- Voir les tableaux avant/après de `docs/wip/fe8-equilibre.md`.
- Le tableau de la sonde compte le lien féodal direct comme une alliance pour les paires
  historiques (Bourgogne-France).
- Les sauvegardes antérieures gardent leurs alliances suzerain-vassal jusqu'à la rupture ; aucune
  migration (spec FE : nouvelle version de format sans migration).
