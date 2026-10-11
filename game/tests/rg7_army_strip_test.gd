extends TestCase

## RG7 : état des boutons « Recruter » / « Fusionner avec… » du bandeau d'ost.
## Usage : godot --headless --path game --script res://tests/rg7_army_strip_test.gd

const PREVIEW := preload("res://tests/hud_preview.gd")

var _received: Array = []


func _init() -> void:
	await process_frame
	var strip: ArmyStrip = (load("res://scenes/ui/army_strip.tscn") as PackedScene).instantiate()
	root.add_child(strip)
	var catalog := ArmyStrip.load_unit_catalog(ProjectSettings.globalize_path("res://../data"))
	var army: Dictionary = PREVIEW.demo_army()
	strip.set_army(army, 20, catalog, "Ost")
	await process_frame
	var recruit_button: Button = strip.find_child("RecruitButton", true, false)
	var merge_button: MenuButton = strip.find_child("MergeButton", true, false)
	check(recruit_button != null and merge_button != null, "RG7 buttons exist")
	check(not recruit_button.visible, "foreign army: no Recruter button")
	var own := army.duplicate(true)
	own["recruit_blocker"] = "Hors de votre territoire"
	strip.set_army(own, 20, catalog, "Ost")
	check(recruit_button.visible and recruit_button.disabled, "blocked army: Recruter disabled")
	check(recruit_button.tooltip_text == "Hors de votre territoire", "blocker is the tooltip")
	check(not merge_button.visible, "no merge target: no Fusionner button")
	own["recruit_blocker"] = ""
	own["merge_targets"] = [{"id": "army_2", "name": "Ost de Bourgogne", "units": 3}]
	strip.set_army(own, 20, catalog, "Ost")
	check(not recruit_button.disabled, "own territory: Recruter enabled")
	check(merge_button.visible and merge_button.get_popup().item_count == 1, "merge menu lists the target")
	strip.recruit_requested.connect(func() -> void: _received.append("recruit"))
	strip.merge_requested.connect(func(id: String) -> void: _received.append(id))
	recruit_button.pressed.emit()
	strip._on_merge_chosen(0)
	check(_received == ["recruit", "army_2"], "RG7 signals: %s" % [_received])
	strip.queue_free()
	finish()
