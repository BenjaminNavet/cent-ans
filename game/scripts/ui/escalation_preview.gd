class_name EscalationPreview
extends VBoxContainer

## Lot FE6 (spec FE § 4.3, § 6) : « Qui peut entrer en guerre » avant une déclaration. Chaîne
## d'escalade lue dans `CampaignSim.get_war_escalation_preview(attaquant, cible)` : un maillon par
## suzerain appelé (ou l'arbitre d'une guerre privée), avec son estimation (probable, incertain,
## improbable) et la raison principale. Aucune règle ici.

const LIKELIHOOD_COLORS := {
	"likely": Color(0.60, 0.10, 0.08),
	"uncertain": Color(0.62, 0.45, 0.08),
	"unlikely": Color(0.22, 0.45, 0.22),
}
const LIKELIHOOD_HEX := {"likely": "#8b1a1a", "uncertain": "#8a6a10", "unlikely": "#2a6a2a"}

var steps: Array = []
var _rows: VBoxContainer


func _init() -> void:
	name = "EscalationPreview"
	add_theme_constant_override("separation", 4)
	var title := HudStyle.label("Qui peut entrer en guerre", UiType.size(UiType.HEADING), HudStyle.RUBRIC)
	title.name = "Title"
	add_child(title)
	_rows = VBoxContainer.new()
	_rows.name = "Rows"
	_rows.add_theme_constant_override("separation", 3)
	add_child(_rows)


static func available(sim: Object) -> bool:
	return sim != null and sim.has_method("get_war_escalation_preview")


static func read(sim: Object, attacker: String, target: String) -> Array:
	if not available(sim) or attacker == "" or target == "":
		return []
	return sim.call("get_war_escalation_preview", attacker, target)


## Remplit la chaîne d'escalade ; caché s'il n'y a aucun maillon (cible souveraine sans suzerain).
func show_for(sim: Object, attacker: String, target: String) -> void:
	steps = read(sim, attacker, target)
	for child in _rows.get_children():
		_rows.remove_child(child)
		child.queue_free()
	visible = not steps.is_empty()
	for index in steps.size():
		_rows.add_child(_row(steps[index] as Dictionary, index))


func _row(step: Dictionary, index: int) -> Control:
	var row := HBoxContainer.new()
	row.name = "Step%d" % index
	row.add_theme_constant_override("separation", 6)
	var arms := TextureRect.new()
	arms.texture = PortraitLoader.heraldry_texture(str(step.get("faction", "")))
	arms.custom_minimum_size = Vector2(22, 24)
	arms.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	arms.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	row.add_child(arms)
	var key := str(step.get("likelihood", ""))
	var badge := HudStyle.label(str(step.get("likelihood_label", key)).capitalize(), UiType.size(UiType.CAPTION), LIKELIHOOD_COLORS.get(key, HudStyle.INK))
	badge.name = "Likelihood"
	badge.custom_minimum_size = Vector2(76, 0)
	row.add_child(badge)
	var text := HudStyle.label("%s%s — %s" % ["↳ " if index > 0 else "", str(step.get("name", "")), str(step.get("reason", ""))], UiType.size(UiType.BODY), HudStyle.INK)
	text.name = "Reason"
	text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(text)
	return row


## Même chaîne en BBCode (infobulle de l'action « Déclarer la guerre » de la diplomatie).
static func bbcode(sim: Object, attacker: String, target: String) -> String:
	var chain := read(sim, attacker, target)
	if chain.is_empty():
		return ""
	var parts := PackedStringArray()
	for step in chain:
		var key := str(step.get("likelihood", ""))
		parts.append("%s ([color=%s]%s[/color] : %s)" % [str(step.get("name", "")), LIKELIHOOD_HEX.get(key, "#3a2a10"),
			str(step.get("likelihood_label", key)), str(step.get("reason", ""))])
	return "[b]Qui peut entrer en guerre :[/b] " + " → ".join(parts)
