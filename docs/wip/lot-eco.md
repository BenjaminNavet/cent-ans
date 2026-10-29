# Lot ECO (boucle juge 2, itération 1)

État : terminé, en attente de fusion. ADR : `docs/decisions/0793-economie-attrition-reserve.md` (à numéroter).

Fait : army_refs (purge des batailles en attente + garde fusion/garnison + test) ; économie resserrée ; attrition hors
territoire ami ; réserve de recrutement exposée (`game_api::hud::recruit_reserve`, `RecruitReserveView`) ; bots prudents
(`prudent_candidates`, `austerity_orders`) ; bandes longues resserrées ; `victory_min_score` 72 → 50 ; starting_fit régénéré.

Chiffres seed bots (passe longue, 32 parties de 150 tours), avant -> après :
- turtle : banqueroutes 89 -> 39, trésor fin -33 000 -> -2 400, provinces 4,8 -> 5,5
- aggressor : banqueroutes 55 -> 19, provinces 2,5 -> 1,0 (belliciste, reste fragile)
- diplomat : banqueroutes 18 -> 3 ; builder : 10 -> 2 ; mirror : provinces 6,5 -> 8,0, banqueroutes 9 -> 5
- score de règne : 8-14 (mirror 14) ; seuil 50.
- France au départ : trésor 28 000 -> 16 000, net/saison 14 336 -> environ 10 000.

Écarts : le rejeu du constat army_refs n'a pas été possible (sauvegarde d'un état non committé du juge) ; cause établie par lecture.
Points ouverts : `upkeep_months_per_season` conservé à 4 ; seuil de victoire à valider par partie humaine ;
aggressor/random-legal/turtle dépassent encore des bandes (voulu, l'oracle détecte).
