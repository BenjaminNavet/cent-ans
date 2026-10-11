class_name CrusadeWindow
extends PanelContainer

## RG2 — fenêtre « Appel à la croisade » (clic sur la pastille « Croisade » de la cloche) :
## cible de l'appel du pape, seigneur qui la tient, princes déjà partis, ce que rejoindre
## signifie (déclarer la guerre au détenteur, ADR 0327), récompense et comment gagner. Aucune
## règle ici : tout vient de `CampaignSim.get_papal_crusade` (valeurs de
## `data/rules/religion.json`), l'adhésion est l'ordre `declare_war` existant.

## L'adhésion confirmée : la carte soumet `declare_war` contre `holder`.
signal join_requested(holder: String)
## « Voir la cible » : la caméra se place sur la province `province`.
signal target_requested(province: String)
signal closed

const WIDTH := 520.0
const MUTED := Color(0.42, 0.33, 0.20)

var view: Dictionary = {}
var title_label: Label
var body_label: RichTextLabel
var reason_label: Label
var join_button: Button
var target_button: Button
var close_button: Button


func _init() -> void:
	name = "CrusadeWindow"
	theme = load("res://scenes/ui/parchment_theme.tres")
	custom_minimum_size = Vector2(WIDTH, 0)
	var box := UiBuild.vbox(8)
	add_child(box)
	var header := HBoxContainer.new()
	box.add_child(header)
	title_label = UiBuild.label("Appel à la croisade", UiType.size(UiType.HEADING))
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title_label)
	var x_button := UiBuild.button("×")
	TooltipHost.attach_plain(x_button, "close")
	x_button.pressed.connect(close)
	header.add_child(x_button)
	box.add_child(HSeparator.new())
	body_label = RichTextLabel.new()
	body_label.bbcode_enabled = true
	body_label.fit_content = true
	body_label.scroll_active = false
	body_label.custom_minimum_size = Vector2(WIDTH - 30.0, 0)
	UiType.apply(body_label, UiType.BODY)
	box.add_child(body_label)
	reason_label = UiBuild.label("", UiType.size(UiType.CAPTION), MUTED, true, WIDTH - 30.0)
	reason_label.name = "Reason"
	box.add_child(reason_label)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	row.add_theme_constant_override("separation", 10)
	box.add_child(row)
	join_button = UiBuild.button("Rejoindre la croisade", _on_join_pressed)
	join_button.name = "Join"
	join_button.theme_type_variation = &"PrimaryButton"
	row.add_child(join_button)
	target_button = UiBuild.button("Voir la cible", func() -> void: target_requested.emit(str(view.get("target", ""))))
	target_button.name = "Target"
	row.add_child(target_button)
	close_button = UiBuild.button("Fermer", close)
	close_button.name = "CloseButton"
	row.add_child(close_button)
	hide()


## Remplit la fenêtre depuis `get_papal_crusade` (`{}` : aucun appel en cours).
func show_call(call_view: Dictionary) -> void:
	view = call_view
	body_label.text = describe(call_view)
	var can_join := bool(call_view.get("can_join", false))
	join_button.disabled = not can_join
	reason_label.text = "" if can_join else str(call_view.get("join_blocker", "Aucun appel en cours."))
	reason_label.visible = reason_label.text != ""
	target_button.disabled = str(call_view.get("target", "")) == ""
	show()


func close() -> void:
	hide()
	closed.emit()


## Texte explicatif (BBCode) d'un appel ; pur, testable sans arbre de scène.
static func describe(v: Dictionary) -> String:
	if v.is_empty():
		return "Le pape n’a lancé aucun appel à la croisade pour l’instant."
	var lines := PackedStringArray()
	var holder := str(v.get("holder_name", ""))
	lines.append("Le pape appelle la chrétienté à reprendre [b]%s[/b]%s. Il reste %s pour répondre." % [
		str(v.get("target_name", "?")),
		(", aux mains de [b]%s[/b]" % holder) if holder != "" else "",
		_seasons(int(v.get("turns_left", 0)))])
	var names := PackedStringArray()
	for participant: Dictionary in v.get("participants", []):
		names.append(str(participant.get("name", participant.get("id", ""))))
	lines.append("[b]Princes partis :[/b] %s." % (", ".join(names) if not names.is_empty() else "aucun pour l’instant"))
	lines.append("[b]Rejoindre :[/b] c’est déclarer la guerre à %s. Tout prince catholique qui lui fait la guerre pendant l’appel gagne [b]+%d[/b] de faveur papale et [b]+%d[/b] de prestige, une fois par appel." % [
		holder if holder != "" else "celui qui tient la cible", int(v.get("join_favor", 0)), int(v.get("join_prestige", 0))])
	lines.append("[b]Vaincre :[/b] occuper la cible. Le premier prince catholique qui la tient clôt l’appel et reçoit [b]+%d[/b] de faveur, [b]+%d[/b] de prestige et jusqu’à [b]%d[/b] livres du trésor du pape." % [
		int(v.get("reward_favor", 0)), int(v.get("reward_prestige", 0)), int(v.get("reward_gold", 0))])
	lines.append("[b]Sans suite :[/b] si personne ne la tient avant l’échéance, l’appel tombe sans récompense. La croisade n’impose aucune pénalité propre ; la guerre, elle, garde ses conséquences habituelles.")
	return "\n\n".join(lines)


static func _seasons(count: int) -> String:
	return "%d saison%s" % [count, "s" if count > 1 else ""]


func _on_join_pressed() -> void:
	if join_button.disabled:
		return
	var holder := str(view.get("holder", ""))
	var text := "Prendre la croix, c’est déclarer la guerre à %s. Faveur papale +%d, prestige +%d." % [
		str(view.get("holder_name", holder)), int(view.get("join_favor", 0)), int(view.get("join_prestige", 0))]
	ConfirmDialog.ask(get_parent() if get_parent() != null else self, "Rejoindre la croisade ?", text,
		func() -> void: join_requested.emit(holder), "Déclarer la guerre", "Renoncer")
