# 0320 — Simulation de bataille : mur de lances, moral par type, discours, cadence, ébranlé (TW bsim)

Statut : accepté.

## Contexte
Le rapport « simulation de bataille » du chantier TW (`docs/wip/tw/bataille-simulation.md`) relevait que seules deux unités (piques) avaient un bonus anti-cavalerie, que la perte de moral par pertes était identique pour tous, que les discours étaient décoratifs, que la cadence de tir était unique et que l'état « hésite » ne servait qu'à l'affichage.

## Décision
- **spear_wall** : capacité passive `Ability::SpearWall` (lanciers gallois, goedendag, coutiliers, milice urbaine ; pas les unités à `pike_square`, déjà couvertes). Face à une cavalerie montée placée devant eux : coups x1,3 ; la charge de face se brise en partie (perte de 4 % des cavaliers, -5 moral, choc x0,5, bonus de charge à moitié). Flanc et dos : aucun effet. Valeurs : `data/rules/battle_spear_wall.json` (module `spear_wall.rs`). Choix : capacité passive plutôt que `data/battle_abilities/` (qui sont des capacités actives à recharge et conditions) — plus simple et sans action du joueur.
- **Moral par type** : `battle_morale.json` `loss_by_morale` (facteur `1 + (référence - stats.morale) x pente`, borné 0,6-1,5, référence 55). Champ absent = facteur 1 (valeurs inchangées).
- **Discours** : `battle_speeches.json` `morale_bonus` (fort/égal/faible + durée) ; au `start_battle`, chaque camp reçoit le bonus selon le rapport d'effectifs et `odds_thresholds`, par le mécanisme du cri de guerre (plafond relevé puis retombée). Le plus faible est le plus galvanisé. Absent = aucun effet.
- **Cadence** : `stats.reload_s` (entier, secondes) optionnel par type, prioritaire sur les constantes (6 s, 9 s pavois, 12 s engins). Arc long 4 s ; arbalètes à pavois inchangées (9 s déjà plus lentes).
- **Ébranlé** : l'état `wavering` existait déjà (seuil `unit_modes.json` `status.wavering_morale` = 35, pastille `wavering` du HUD, exposé par le pont). On le rend effectif : `wavering_damage_factor` 0,8 (mêlée et tir) et `wavering_no_charge`. Pas de second seuil dans `battle_pace.json`.

## Conséquences
Équilibre de bataille modifié (moral des levées, milices plus coriaces contre la cavalerie, tir longbow +50 %) : à recalibrer par le lot `balance` (ADR 0328). Les sauvegardes restent lisibles (`reload_s` et règles optionnels).
