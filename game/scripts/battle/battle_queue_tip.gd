class_name BattleQueueTip
extends PanelContainer

## CB-M3 : infobulle du curseur quand Maj est tenue et que la file d'ordres d'un régiment
## sélectionné est pleine (le curseur passe à `forbidden`). Rendu seulement : la borne vient du
## cœur (`data/rules/battle_queue.json`, lue par RuleValues), le texte de `BattleInput`.

const OFFSET := Vector2(20, 18)

var _label: Label


func _ready() -> void:
	name = "QueueTip"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	_label = Label.new()
	_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	_label.add_theme_color_override("font_color", Color(0.55, 0.12, 0.10))
	add_child(_label)


## Affiche `text` près de la souris (`at`, coordonnées d'écran).
func show_at(at: Vector2, text: String) -> void:
	if _label == null:
		return
	_label.text = text
	position = at + OFFSET
	visible = true


func hide_tip() -> void:
	visible = false


## Texte affiché (tests).
func text() -> String:
	return _label.text if _label != null and visible else ""
