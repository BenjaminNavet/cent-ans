# WH armyb — état

Lot : armees top4, top5, top6, top9 ; ui top9. ADR 0279. FAIT (branche wh/armyb), reste : mesures de la sonde (ci-dessous) à renseigner si absentes.

## Fait
- top4 : `Order::RecruitInto` + `QueuedRecruit.into_army`, livraison `economy::deliver_to_armies` (repli garnison). Bridge : `settlement_detail.recruit_armies[]` ; UI : OptionButton « destination » du panneau de colonie.
- top5 : `Order::Sortie`, `siege::sortie(.., forced)`, `siege::order_sortie` ; bridge `settlement_detail.can_sortie` ; bouton « Faire une sortie ».
- top6 : `capture.json:raid.supply_gain_percent / diminishing_devastation / diminishing_loot_percent`.
- top9 : le repos en place existait (NT6c, `rest_plan` 60 % / 85 %) ; ajout de `grid.json:postures.rest.seek_place` (`stances::should_seek_rest_place`, `army.rs::pick_rest_place`).
- ui top9 : `Order::DemandSurrender`, `siege::surrender_chance` (`capture.json:siege.surrender`), `get_assault_odds.surrender_chance/settlement`, bouton « Sommer la garnison ».
- Tests : `sim-campaign/tests/armies/wh_armyb.rs` (9), `ai/tests/movement/nt6c_rest.rs` (+1, 1 adapté), `game/tests/wh_armyb_ui_test.gd`.

## Écarts
- `RecruitInto` distinct de `Recruit` ; Sortie non interactive ; seuil de dévastation au lieu de « saisons » ; sommation déterministe (hachage tour/place) ; l'IA n'utilise aucun des nouveaux ordres.

## Sonde campaign_probe (120 tours, graines 1,2)
voir fin de fichier
`campaign_probe --turns 120 --seeds 1,2` (seek_place activé / désactivé par `CENT_ANS_DATA_DIR`) :
- graine 1 : identique (FR-EN 69 %, 96 banqueroutes, 1 révolte) ;
- graine 2 : FR-EN 62 % / 66 %, guerres actives 31,6 / 33,0, éliminées 18 / 17, révoltes 0 / 3, banqueroutes 127 / 98.
Dans la cible FR-EN 55-75 % ; pas de dérive nette (les banqueroutes de la graine 2 varient avec le flux aléatoire).
