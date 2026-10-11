class_name ForecastText
extends RefCounted

## RG8 : mise en texte des prévisions du prochain tour (prestige, hommes, troubles, solde, trésor).
## Aucune règle ici : les valeurs viennent de `CampaignSim.get_faction_forecast`
## (`{current, next, delta, other, contributions: [{key, label, amount}]}`), le cœur a déjà trié
## les termes par poids absolu.

## Nombre de termes détaillés avant « Autres ».
const TOP := 5

## Libellés des clés de `core/` sans libellé propre (prestige : le cœur fournit `label`).
const LABELS := {
	# Hommes sous les armes (phases de fin de tour)
	"attrition": "Disette et usure en campagne",
	"replenish": "Renforts des armées au repos",
	"mercenaries": "Désertion des compagnies impayées",
	"crusade": "Croisade (ferveur, désertion)",
	# Prestige sans nom de bâtiment
	"table": "Table de la cour",
	"tech": "Techniques (bilan de l'hiver)",
	"ruler": "Souverain : traits et talents (hiver)",
	"retinue": "Compagnons du souverain (hiver)",
	# Troubles : termes du cœur (`unrest_terms`) et inertie
	"inertia": "Mécontentement actuel qui retombe",
}

const AUTRES := "Autres"


## Flèche et écart pour le texte d'une pastille : « ▲+3 », « ▼−2 », « = » si nul ; "" sans prévision.
## `bad_when_up` : vrai pour les troubles (monter est mauvais), sans effet sur le texte.
static func arrow(forecast: Dictionary) -> String:
	if forecast.is_empty():
		return ""
	var delta := int(forecast.get("delta", 0))
	if delta == 0:
		return "="
	return ("▲+%s" if delta > 0 else "▼−%s") % Money.digits(absi(delta))


## Couleur d'un écart : vert si favorable, rouge sinon ; encre si nul.
static func color_of(delta: float, bad_when_up: bool = false) -> Color:
	if absf(delta) < 0.05:
		return Money.INK_COLOR
	return Money.LOSS_COLOR if (delta > 0.0) == bad_when_up else Money.GAIN_COLOR


static func label_of(term: Dictionary) -> String:
	var label := str(term.get("label", ""))
	if label != "":
		return label
	var key := str(term.get("key", ""))
	if LABELS.has(key):
		return str(LABELS[key])
	return str(RichTooltip.UNREST_TERM_LABELS.get(key, key))


## `Actuel → Prochain tour (±Δ)` puis les cinq plus gros termes signés et « Autres ».
## `unit` : suffixe de valeur (« % » pour les troubles) ; `decimals` : un chiffre après la virgule
## pour les termes de troubles. `bad_when_up` colore les hausses en rouge.
static func tooltip(forecast: Dictionary, unit: String = "", bad_when_up: bool = false, decimals: bool = false) -> String:
	if forecast.is_empty():
		return ""
	var delta := int(forecast.get("delta", 0))
	var lines := PackedStringArray()
	lines.append(
		"[b]Actuel → Prochain tour :[/b] %s%s → %s%s ([color=#%s]%s[/color])" % [
			Money.digits(int(forecast.get("current", 0))), unit,
			Money.digits(int(forecast.get("next", 0))), unit,
			color_of(delta, bad_when_up).to_html(false), _signed(float(delta), false) + unit,
		]
	)
	var terms: Array = forecast.get("contributions", [])
	var shown := mini(terms.size(), TOP)
	var rest := float(forecast.get("other", 0.0))
	for i in terms.size():
		if i >= shown:
			rest += float(terms[i].get("amount", 0.0))
	for i in shown:
		var amount := float(terms[i].get("amount", 0.0))
		lines.append("%s : [color=#%s]%s[/color]" % [label_of(terms[i]), color_of(amount, bad_when_up).to_html(false), _signed(amount, decimals) + unit])
	if absf(rest) >= (0.05 if decimals else 0.5):
		lines.append("%s : [color=#%s]%s[/color]" % [AUTRES, color_of(rest, bad_when_up).to_html(false), _signed(rest, decimals) + unit])
	lines.append("[i]Prévision sans hasard : les événements imprévus et les décisions de l'IA n'y figurent pas.[/i]")
	return "\n".join(lines)


## « +3 », « −2 », « +1,4 » (virgule française, vrai signe moins).
static func _signed(value: float, decimals: bool) -> String:
	var text := ("%.1f" % absf(value)).replace(".", ",") if decimals else Money.digits(int(round(absf(value))))
	if round(absf(value) * (10.0 if decimals else 1.0)) == 0.0:
		return "0"
	return ("+" if value > 0.0 else Money.MINUS) + text


## Solde : principales sources de recettes (`income_lines` de `get_faction_economy`) et provinces
## qui rapportent le plus (`get_income_top_provinces`), en livres.
static func income_drivers(lines: Array, provinces: Array) -> String:
	var rows := PackedStringArray()
	var sorted := lines.duplicate()
	sorted.sort_custom(func(a, b): return absi(int(a.get("value", 0))) > absi(int(b.get("value", 0))))
	for i in mini(sorted.size(), TOP):
		var value := int(sorted[i].get("value", 0))
		rows.append("%s : [color=#%s]%s[/color]" % [IncomeLines.label_of(str(sorted[i].get("key", ""))), Money.color_of(value).to_html(false), Money.signed(value)])
	var out := PackedStringArray()
	if not rows.is_empty():
		out.append("[b]Principales recettes[/b]\n" + "\n".join(rows))
	var places := PackedStringArray()
	for i in mini(provinces.size(), 3):
		places.append("%s : %s" % [str(provinces[i].get("name", "")), Money.signed(int(provinces[i].get("value", 0)))])
	if not places.is_empty():
		out.append("[b]Provinces qui rapportent le plus[/b]\n" + "\n".join(places))
	return "\n".join(out)


## Trésor : « Actuel → Prochain tour (±Δ) » avec trésor prochain = trésor actuel + solde.
static func treasury_line(treasury: int, net: int) -> String:
	return "[b]Actuel → Prochain tour :[/b] %s → %s ([color=#%s]%s[/color])" % [Money.amount(treasury), Money.amount(treasury + net), Money.color_of(net).to_html(false), Money.signed(net)]
