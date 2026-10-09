extends TestCase

## Test headless du lot WH hover (ADR 0271), parties pures (aucun rendu) :
## textes des bulles, jetons de parchemin, numéros de tour, mémoire des armées perdues de vue,
## réglages de `data/map/stance_cues.json`.
## Usage : godot --headless --path game --script res://tests/wh_hover_text_test.gd


func _init() -> void:
	await process_frame
	_army_text()
	_settlement_text()
	_token_cues()
	_turn_labels()
	_memory()
	_tuning()
	finish()


func _by_type(unit_type: String) -> String:
	return {"u_archer": "ranged", "u_knight": "cavalry"}.get(unit_type, "infantry")


func _army(stance: String) -> Dictionary:
	return {"faction": "fac_england", "general_name": "Édouard", "stance": stance, "supply": 7,
		"units": [{"unit_type": "u_spear", "strength": 600}, {"unit_type": "u_archer", "strength": 300}, {"unit_type": "u_knight", "strength": 100}],
		"path": ["set_calais"]}


func _army_text() -> void:
	var own := MapHoverText.army_text(_army("raid"), {"is_player": true, "faction_name": "Angleterre", "category_of": Callable(self, "_by_type"), "destination_name": "Calais"})
	check(own.contains("Ost de Édouard"), "title names the general: %s" % own)
	check(own.contains("1 000 hommes"), "men total: %s" % own)
	check(own.contains("300 archers") and own.contains("100 cavaliers") and own.contains("600 fantassins"), "composition: %s" % own)
	check(own.contains("Chevauchée"), "own posture shown")
	check(own.contains("En marche vers Calais"), "moving state: %s" % own)
	check(own.contains("Vivres : 7"), "own supplies shown")
	var enemy := MapHoverText.army_text(_army("ambush"), {"is_player": false, "faction_name": "Angleterre", "liege_name": "Londres", "category_of": Callable(self, "_by_type")})
	check(not enemy.contains("Embuscade") and not enemy.contains("Posture"), "an enemy ambush must not be shown: %s" % enemy)
	check(not enemy.contains("Vivres"), "enemy supplies must not be shown")
	check(enemy.contains("vassal de Londres"), "vassalage shown: %s" % enemy)
	var ghost := MapHoverText.army_text({"faction": "fac_england", "general_name": ""}, {"faction_name": "Angleterre", "men": 1200, "seen_ago": 2})
	check(ghost.contains("Vue il y a 2 saisons") and ghost.contains("1 200 hommes"), "ghost text: %s" % ghost)
	check(not ghost.contains("Vivres") and not ghost.contains("Posture"), "ghost carries no live info")


func _settlement_text() -> void:
	var detail := {"name": "Rouen", "owner": "fac_england", "controller": "fac_england", "buildings": ["bld_stone_walls"], "fortification_level": 3,
		"garrison": [{"strength": 200}], "garrison_strength": 200,
		"siege": {"attacker": "fac_france", "turns_elapsed": 3, "supplies": 4, "breach": 20}}
	var seen := MapHoverText.settlement_text(detail, {"visible": true, "is_player": false, "level": 2, "owner_name": "Angleterre", "attacker_name": "France"})
	check(seen.contains("murailles"), "walls shown: %s" % seen)
	check(seen.contains("Ville") and seen.contains("Angleterre"), "level and owner: %s" % seen)
	check(seen.contains("Assiégée par France depuis 3 tours"), "siege shown when visible: %s" % seen)
	check(seen.contains("Garnison"), "garrison shown when visible")
	var hidden := MapHoverText.settlement_text(detail, {"visible": false, "owner_name": "Angleterre"})
	check(hidden.contains("Rouen") and hidden.contains("Angleterre"), "name and owner always")
	check(not hidden.contains("murailles") and not hidden.contains("Assiég") and not hidden.contains("Garnison"), "nothing out of sight: %s" % hidden)


func _token_cues() -> void:
	var enemy := ParchmentOverlay.token_cues(StanceCues.ENEMY, "ambush", "moving", false)
	check((enemy["ring"] as Color).a > 0.0, "enemy token has a hostility ring")
	check(str(enemy["glyph"]) != "", "enemy token has the enemy glyph")
	check(str(enemy["badge"]) == "", "enemy ambush badge is hidden")
	check(bool(enemy["moving"]), "moving state flagged")
	var own := ParchmentOverlay.token_cues(StanceCues.SELF, "raid", "", true)
	check(str(own["badge"]) == "raid" and (own["ring"] as Color).a == 0.0 and not bool(own["moving"]), "own token: badge, no ring, resting")
	var own_ambush := ParchmentOverlay.token_cues(StanceCues.SELF, "ambush", "", true)
	check(str(own_ambush["badge"]) == "ambush", "own ambush badge shown")
	check(str(ParchmentOverlay.token_cues(StanceCues.OTHER, "normal", "", false)["badge"]) == "", "normal posture has no badge")


func _turn_labels() -> void:
	check(ArmyMovementPath.turn_label_text(0, 3) == "1" and ArmyMovementPath.turn_label_text(1, 3) == "2" and ArmyMovementPath.turn_label_text(2, 3) == "T3", "turn label texts")
	check(ArmyMovementPath.turn_label_text(0, 1) == "1", "a single turn is just 1")


func _memory() -> void:
	var memory := ArmyMemory.new()
	var army := {"faction": "fac_england", "general_name": "Édouard", "units": [{"strength": 500}]}
	memory.note("a1", army, 5, Vector2(10, 20))
	memory.note("a2", army, 5, Vector2(30, 40))
	check(memory.ghosts(5, {"a1": true, "a2": true}, 4).is_empty(), "armies in view are not ghosts")
	var ghosts := memory.ghosts(6, {"a2": true}, 4)
	check(ghosts.size() == 1 and ghosts[0]["id"] == "a1" and int(ghosts[0]["ago"]) == 1 and int(ghosts[0]["men"]) == 500, "a1 lost from sight becomes a ghost")
	memory.note("a1", army, 7, Vector2(15, 20))
	check(memory.ghosts(7, {"a1": true, "a2": true}, 4).is_empty(), "a reappearing army is no longer a ghost")
	check(memory.ghosts(12, {}, 4).is_empty(), "ghosts expire after the memory duration")
	check(memory.ghosts(12, {}, 4).is_empty() and memory.ghosts(7, {}, 4).is_empty(), "expired entries are forgotten for good")


func _tuning() -> void:
	check(StanceCues.fog_memory_turns() > 0, "fog memory duration comes from data")
	check(StanceCues.zoc_max_rings() == 16, "ZOC ring cap from data")
	check(StanceCues.zoc_color().r > 0.7 and StanceCues.zoc_color().g < 0.3, "ZOC colour is the enemy red")
