# IA-REFUS : ordres IA refusés par le cœur

État : TERMINÉ. Métrique `ai_refused_orders` passe longue : 430-515 -> 23-38 par bot (bande [0 ; 40]).

Causes corrigées (côté IA, `core/crates/ai/`) :
- AssignGeneral / ChooseArmyTradition / RecruitInto / MergeArmies sur une armée dont toutes les unités sont licenciées dans le même plan (`drop_orders_on_emptied_armies`).
- MergeArmies de contingents féodaux (`host_service`) : exclus de `merge_colocated`.
- MoveArmy / Embark : jambes au-delà du mouvement restant (`march_orders_with_cap`), mouvement suivi sur tout le plan (fusion, attaque, embarquement ; `drop_moves_without_movement`), premier tronçon vérifié par `find_path` (NoPath).
- AgentAction : les agents sont planifiés avant la diplomatie (une paix/guerre du même plan rendait Incite/Parley illégal).
- DeclareCommise : seulement contre un vassal encore direct.
- Recruit / RecruitInto / Build / RoyalAct : solde courant sur tout le plan (`drop_unaffordable_orders`).
- Diagnostic : histogramme `ai refusal kind` (seed-bots), 2 échantillons par type.
- Donnée : `bu2_amb_conseil` prestige 15 -> 20 (test d'ambitions à 9,4 % < 10 % après décalage de trajectoire).

Reste (petit) : Recruit/RecruitInto fonds (prix réel > estimé), RoyalAct, LevyExtraordinary NoProvince, SummonHost HostNoWar, HireMercenary sur armée entièrement licenciée.
Test : `core/crates/seed-bots/tests/ai_refusals.rs` (ignoré, `--release -- --ignored`).
