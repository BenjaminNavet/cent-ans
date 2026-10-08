# 0201 — Suppression de la bataille navale 3D

## Contexte
Le bouton « Combattre » de l'écran d'avant-bataille navale est masqué depuis le 25/09 (`PLAYABLE_3D := false`) :
le joueur veut des batailles navales en résolution automatique seulement, sans combat de mer en 3D. La scène
3D (`naval_scene`, vues de navires, HUD, mer, effets, feu, vagues, 4 shaders, 4 modèles de navires, 2 tests)
et la classe `NavalBattleSim` du pont ne servaient plus qu'au scénario de debug `--naval-scenario` : environ
3 000 lignes de code mort.

## Décision
- Supprimer la scène, les scripts, les shaders, les modèles et les tests de la bataille navale 3D, le flag
  `--naval-scenario`, la classe `NavalBattleSim` et `CampaignSim.resolve_naval_battle` (pont).
- Garder `naval_campaign.gd` et `naval_pre_battle_dialog.gd` : interception, chances estimées par le cœur,
  « Résolution automatique » et « Retraite ». Le bouton « Combattre » (hérité de `PreBattleDialog`) y reste caché.
- L'aperçu du vent de l'écran d'avant-bataille disparaît avec `NavalBattleSim` ; saison et pluie restent affichées.
- La résolution automatique navale du cœur (`sim_battle::naval::auto_resolve`, `auto_resolve_naval_battle`) est inchangée.

## Conséquences
- Plus de bataille navale jouable ; la rétablir demanderait de reprendre l'historique git (commit précédant ce lot).
- Le moteur naval temps réel de `sim-battle` est retiré à son tour (lot SC « naval ») : `NavalSim`, l'IA, les
  commandes, les scénarios historiques `data/naval/scenarios` et leur schéma, l'événement `NavalEvent`, les
  champs de placement et de vent de `NavalSetup` (position, cap, vaisseau amiral, rivage, vent imposé).
  Restent : `ship`, `combat` (formules de tir, d'abordage et de feu), `fleet` (déploiement, vent et avantage du
  vent tirés de la graine), `auto` (résolution par phases), `outcome` et `setup`.
- L'auto-résolution est découpée en phases nommées (`Battle::volleys_while_closing`, `fireships`,
  `boarding_rounds`, `winner`, `flight_of`) ; ses constantes numériques sont dans `data/naval/rules.json`
  (`NavalRules`) et les règles de manœuvre temps réel (vent, grappins, assaut général, durée) en ont disparu.
  Résultats identiques à l'ancien code (vérifié sur 180 combats).
- `NavalSetup.rules` et `NavalData.rules` sont des `Arc<NavalRules>` : une bataille ne copie plus les règles.
- `resolve_naval_battle` de `sim-campaign` reste : il applique le résultat de l'auto-résolution.
