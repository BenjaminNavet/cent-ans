# RG8 — prévisions du prochain tour (barre du haut)

## État
- Cœur : `sim-campaign/src/forecast.rs` — `faction_forecast` : valeur suivante = résolution de fin
  de tour réelle jouée sur une copie (chronique aléatoire coupée) ; contributions : prestige
  (`table::table_prestige`, `dynasty::court_prestige_parts`, `chivalry::yearly_order_prestige`),
  troubles (`population::unrest_pull`), hommes (phases attrition / renforts / mercenaires / croisade
  jouées isolément). Reste inexpliqué = `other` (« Autres »).
- Refactors partagés : `dynasty::yearly_court_prestige` passe par `CourtPrestigeParts`,
  `resolve_chivalry` par `yearly_order_prestige`, `resolve_lent` par `best_diet_effect`.
- Pont : `get_faction_forecast`, `get_income_top_provinces` (`campaign_sim_econ.rs`).
- UI : `ui/forecast_text.gd`, `map_ui.gd` (`set_forecast`, `set_status_chips`), appel dans
  `campaign_map._refresh_top_bar`. Test : `game/tests/rg8_forecast_test.gd`.

## Limites
- Ne prévoit ni les décisions des autres factions (jouées avant la résolution) ni le hasard.
- Une prévision = copie d'état + fin de tour complète à chaque rafraîchissement de la barre.

## Prochaine étape
Vérifier coût en release, lancer tests Godot.
