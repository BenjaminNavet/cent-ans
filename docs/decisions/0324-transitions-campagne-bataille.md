# 0324 — Transitions campagne / bataille (lot TW trans)

Statut : accepté.

## Contexte
Medieval II et Warhammer III montrent un écran de fin après toute bataille, y compris auto-résolue, annoncent le risque avant de la déléguer, laissent passer les chargements et ouvrent la campagne par un briefing. Chez nous, la résolution automatique n'affichait que des lignes de journal, le bouton Auto ne disait rien du sort du général, le chargement durait toujours 3 s, une bataille historique ne se comparait pas à l'Histoire et le tour 1 n'avait pas de briefing.

## Décision
- Résumé d'auto-résolution : `CampaignState::auto_resolve_pending_report` rend `(événements, Option<AutoResolveReport>)` (`battle_request.rs`) ; `auto_resolve_pending` en est l'enveloppe inchangée. Le rapport (vainqueur, province, par camp : faction, effectifs avant, pertes, général pris ou tué, déroute) est construit par `auto_fight_with_opening` à partir du résultat du résolveur, avant que les armées ne bougent. Un assaut n'a pas de rapport (journal seul). Pont : `CampaignSim.auto_resolve_battle_report(index) -> {events, result}` ; `auto_resolve_battle` (tableau d'événements) reste. Godot : `BattleResultScreen.show_auto_summary`, qui réutilise bannière, bilan et cartes de suites avec les données de `BattleAftermath` ; appel depuis `campaign_map._on_battle_auto`, avant `_offer_pending_battles`.
- Risque sur le bouton Auto : `BattleForecast.{attacker,defender}_general_loss_pct` = part des résolutions simulées de la prévision où le général est pris ou tué (générateurs privés : le RNG de campagne n'est pas touché). Pastille `AutoRisk` à côté du bouton.
- Chargement passable : un clic ferme l'écran à tout moment ; le réglage `interface/skip_loading` (défaut faux) supprime l'attente minimale, sauf pour le premier chargement de la partie (3 s gardées).
- « Dans l'Histoire » : carte de l'écran de fin d'une bataille historique jouée hors campagne, d'après `historical_winner` de `BattleSim.get_historical()` (déjà exposé) ; quatre verdicts (renversée / répétée × victoire / défaite).
- Briefing : fenêtre `CampaignBriefing` au tour 0 (avant le tutoriel, qui attend sa fermeture) : objectifs (`get_objectives`), guerres déclarées (`get_faction_summary.at_war_with`), trois conseils. Case « Ne plus afficher » et réglage `interface/campaign_briefing`.
- Tous les textes : `data/ui/transitions.json` (schéma `transitions_ui.schema.json`).

## Conséquences
- Le rapport d'auto-résolution ne porte pas les captifs de troupe ni la poursuite (ils n'existent qu'en bataille jouée) ; l'écran n'affiche donc que chefs, déroute, rançons.
- Le briefing apparaît aussi à la reprise d'une sauvegarde du tour 0 ; sans conséquence.
