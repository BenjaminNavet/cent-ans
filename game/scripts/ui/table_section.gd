class_name TableSection
extends VBoxContainer

## H9 — section « La Table » du panneau de province (squelette).

signal diet_changed(province_id: String, diet_id: String)

var province_id: String = ""


func show_for(_province_id: String, _is_player_owner: bool, _sim: Object = null) -> void:
	pass


func request_diet(_diet_id: String) -> Dictionary:
	return {}
