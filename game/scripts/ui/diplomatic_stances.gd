class_name DiplomaticStances
extends RefCounted

## Lot DP2 : couleurs de la carte diplomatique (mode « Diplomatie » de la carte 3D, de la
## minicarte et de la carte du panneau de diplomatie). Rendu seulement : la position de chaque
## faction vient de `CampaignSim.get_province_stances` (règle dans `sim_campaign::stance`).
##
## Vert allié, bleu accord (commerce, accès militaire, mariage), jaune neutre, orange tension
## (hostilité, embargo, intrusion), rouge guerre, gris vassal ou suzerain.

## Couleurs de relation de la carte diplomatique (aussi lues par la légende).
const RELATION_COLORS := {
	"self": Color(0.85, 0.75, 0.30), "war": Color(0.72, 0.12, 0.10), "truce": Color(0.85, 0.70, 0.20),
	"peace": Color(0.55, 0.55, 0.52), "alliance": Color(0.20, 0.40, 0.78), "vassal": Color(0.50, 0.25, 0.65),
	"suzerain": Color(0.50, 0.25, 0.65),
}

const COLORS := {
	"self": Color(0.96, 0.93, 0.84),
	"ally": Color(0.22, 0.60, 0.26),
	"agreement": Color(0.24, 0.46, 0.82),
	"neutral": Color(0.88, 0.78, 0.26),
	"tension": Color(0.93, 0.50, 0.12),
	"war": Color(0.78, 0.12, 0.10),
	"vassal": Color(0.56, 0.56, 0.56),
}
const LABELS := {
	"self": "Nous", "ally": "Alliés", "agreement": "Accords", "neutral": "Neutres",
	"tension": "Tensions", "war": "Guerre", "vassal": "Vassaux",
}
const TOOLTIPS := {
	"self": "Vos terres",
	"ally": "Alliance",
	"agreement": "En paix, avec un accord : commerce, accès militaire ou mariage",
	"neutral": "En paix, sans accord particulier",
	"tension": "En paix, mais hostiles : mauvaise attitude, embargo, intrusion ou casus belli",
	"war": "En guerre",
	"vassal": "Vassal ou suzerain",
}
const ORDER := ["self", "ally", "agreement", "neutral", "tension", "war", "vassal"]
## Symbole daltonien (U12) de chaque position, repris des relations.
const SYMBOL_KEYS := {"self": "self", "ally": "alliance", "war": "war", "vassal": "vassal", "neutral": "neutral"}


static func available(sim: Object) -> bool:
	return sim != null and sim.has_method("get_province_stances")


## Clés de position par province (`ids`) ; vide si la simulation ne les calcule pas.
## `viewer` (lot DZ) : faction dont on lit les relations ("" : le joueur).
static func stances(sim: Object, ids: PackedStringArray, viewer: String = "") -> PackedStringArray:
	if not available(sim):
		return PackedStringArray()
	if viewer != "" and sim.has_method("get_province_stances_for"):
		return sim.call("get_province_stances_for", viewer, ids)
	return sim.call("get_province_stances", ids)


## Lot DZ : position de `viewer` envers chaque faction ({id: clé}, rebelles en guerre) ; vide si
## la simulation ne la calcule pas.
static func faction_stances(sim: Object, viewer: String) -> Dictionary:
	if sim == null or viewer == "" or not sim.has_method("get_faction_stances_for"):
		return {}
	return sim.call("get_faction_stances_for", viewer)


static func color_of(key: String) -> Color:
	if not COLORS.has(key):
		return Color(0, 0, 0, 0)
	return COLORS[key]


## Couleur de chaque province (alpha 0 : inconnue ou rebelle).
static func colors_for(sim: Object, ids: PackedStringArray) -> PackedColorArray:
	var colors := PackedColorArray()
	for key in stances(sim, ids):
		colors.append(color_of(key))
	return colors


static func symbol(key: String) -> String:
	return Accessibility.relation_symbol(str(SYMBOL_KEYS.get(key, ""))) if Accessibility.colorblind() else ""


## Légende en pastilles (une par position) ; `wrap` : sur plusieurs lignes si la place manque.
## `variation` : PO phase 2 (P2b, ADR 0097) — une des quatre tailles `UiType` (plus de taille ad
## hoc), `UiType.CAPTION` par défaut (légende).
static func legend(variation: String = UiType.CAPTION, swatch: float = 14.0, wrap: bool = true) -> Container:
	var flow: Container = HFlowContainer.new() if wrap else HBoxContainer.new()
	flow.name = "StanceLegend"
	flow.add_theme_constant_override("h_separation", 12)
	flow.add_theme_constant_override("separation", 12)
	flow.add_theme_constant_override("v_separation", 2)
	for key in ORDER:
		var chip := UiBuild.hbox(4)
		chip.tooltip_text = TOOLTIPS[key]
		chip.mouse_filter = Control.MOUSE_FILTER_PASS
		var rect := ColorRect.new()
		rect.custom_minimum_size = Vector2(swatch, swatch)
		rect.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		rect.color = COLORS[key]
		rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
		chip.add_child(rect)
		var label := HudStyle.label(("%s %s" % [symbol(key), LABELS[key]]).strip_edges(), UiType.size(variation), HudStyle.INK)
		UiType.apply(label, variation)
		label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		chip.add_child(label)
		flow.add_child(chip)
	return flow
