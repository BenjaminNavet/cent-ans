# game-api

Vues UI typées et sans moteur (ADR 0357 § 2) : fonctions pures `(data, state, id) -> struct Serialize`.
Aucune règle ici : tout vient de `sim-campaign`. Remplace les `#[func]` en lecture seule de `godot-bridge`.
Chaque fonction renvoie `None` (ou un `Vec` vide) pour un identifiant inconnu, là où Godot renvoyait un `Dictionary` vide.

| Vue (fonction) | Module | Méthode(s) Godot remplacée(s) |
|---|---|---|
| `top_bar` (`TopBar`) | `hud` | `get_turn`, `get_date_label`, `get_faction_summary` |
| `economy` (`EconomyView` : budget, historique, lignes de recettes, prime de mercenaires) | `hud` | `get_faction_economy`, `get_income_breakdown` |
| `forecast` (`ForecastView` : prestige, hommes, troubles) | `hud` | `get_faction_forecast` |
| `income_top_provinces` | `hud` | `get_income_top_provinces` |
| `difficulty_levels`, `current_difficulty` | `hud` | `get_difficulty_levels`, `get_difficulty` |
| `province_sheet` (`ProvinceSheet` : propriétaire, colonies, bâtiments, garnison, siège, ordre public par classe, impôt, revenu, fil d'Ariane féodal) | `province` | `get_province_state`, `get_province_city` (sans `buildable`), `get_province_tax`, `get_province_breadcrumb` |
| `faction_summary` | `faction` | `get_faction_summary` (trésor, provinces) |
| `faction_sheet` (`FactionSheet` : souverain, titres, suzerain, vassaux, loyauté, objectifs, relations) | `faction` | `get_feudal_sheet`, `get_diplomacy` (résumé) |
| `army_ids`, `army_sheet` (`ArmySheet` : unités, effectif, général, points de mouvement, ravitaillement, inactivité, blocage de recrutement, fusions) | `army` | `get_army_ids`, `get_army` |
| `recruit_view` (`RecruitView` : cartes de recrutement avec prix, solde, délai, réserve, prérequis ; file ; places ; armées cibles), `mercenary_view`, `army_extras` (solde, classe par régiment), `stance_options` | `recruit` | `get_recruitable`, `settlement_detail` (file, places), `get_mercenaries`, `get_stance_options` |
| `turn_readiness` (`TurnReadiness` + `Alert`) | `turn` | `CampaignAlerts.collect` (`ui/alerts.gd`) |

## Écarts par rapport à Godot

- `get_province_city` : la liste `buildable` (options de construction avec prérequis) n'est pas portée ; à ajouter avec l'écran de construction.
- Alertes : sièges, armées ennemies (tous les ennemis proches ; le brouillard est filtré par le client), dette, recherche inactive, armées sans ordre, emplacements libres, offres, décisions de chronique. Non portées : missions, rançons, Table/médecine/herbier (dépendent des événements du dernier tour).
- Textes d'alerte : français simple, sans le formatage `Money`/`FrText` du client.
- `get_faction_forecast` simule le tour suivant (coûteux) : le client doit le mettre en cache par révision d'état.
