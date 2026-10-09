class_name IncomeLines
extends RefCounted

## Lignes « revenu par source » (WH econ) : libellés français des clés du cœur
## (`income_breakdown::SOURCE_KEYS` et ajustements de faction) et mise en texte. Aucune règle ici :
## les montants viennent de `CampaignSim` (`income_lines` de `get_faction_economy`,
## `income_lines` de `settlement_detail`).

const LABELS := {
	"poll_peasants": "Taille des paysans",
	"poll_burghers": "Taille des bourgeois",
	"poll_clergy": "Taille du clergé",
	"poll_nobility": "Taille de la noblesse",
	"production": "Production (moulins, ateliers)",
	"tax_rate": "Taux d'imposition",
	"building_tax": "Bâtiments (recettes)",
	"building_trade": "Bâtiments (commerce)",
	"devastation": "Dévastation",
	"full_province": "Province entière",
	"embargo": "Embargos",
	"domain": "Domaine du seigneur",
	"difficulty": "Difficulté",
	"alms": "Aumônes de la croisade",
	"seigniorage": "Seigneuriage",
	"trade_routes": "Routes commerciales",
}


static func label_of(key: String) -> String:
	return str(LABELS.get(key, key))


## Une ligne par terme non nul : « Taille des paysans : +1 204 ₶ ». `lines` : `[{key, value}]`.
static func as_text(lines: Array) -> String:
	var rows := PackedStringArray()
	var total := 0
	for line in lines:
		var value := int(line.get("value", 0))
		total += value
		if value != 0:
			rows.append("%s : %s" % [label_of(str(line.get("key", ""))), Money.signed(value)])
	if rows.is_empty():
		return "Aucun revenu."
	rows.append("Total : %s" % Money.signed(total))
	return "\n".join(rows)
