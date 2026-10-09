# WH armyb — état

Lot : armees top4 (recruter dans l'armée), top5 (ordre Sortie), top6 (chevauchée), top9 (IA repos) ; ui top9 (sommation). ADR 0279.

Plan : (1) `Order::RecruitInto` + `QueuedRecruit.into_army` ; (2) `Order::Sortie` (`siege::sortie_forced`) ;
(3) `capture.json:raid.supply_gain_percent` / `diminishing_after_seasons` ; (4) IA `Rest` ; (5) `Order::DemandSurrender`
+ pont + UI.

Écarts prévus : variante d'ordre distincte `RecruitInto` plutôt qu'un champ sur `Recruit` (évite ~30 sites d'appel) ;
Sortie toujours auto-résolue (pas de BattleRequest interactif).

## Fait
(rien encore : squelette)
