extends SceneTree

## EP7 : batailles historiques (Crécy, Azincourt, Poitiers) — menu et pont GDExtension.
## Usage : godot --headless --path game --script res://tests/ep7_historical_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

const IDS := ["crecy", "azincourt", "poitiers"]

var _failures := 0


func _init() -> void:
	await process_frame
	if not ClassDB.class_exists("BattleSim"):
		print("ep7: extension absente, test ignoré")
		quit(0)
		return
	var battles := HistoricalBattlesMenu.load_battles()
	_check(battles.size() == 3, "3 historical battles listed, got %d" % battles.size())
	var listed: Array = []
	for entry in battles:
		listed.append(str(entry["id"]))
		_check(str(entry["date_fr"]) != "" and str(entry["summary"]) != "", "%s: date and summary" % entry["id"])
		_check(str(entry["historical_winner"]) == "defender", "%s: English (defender) historical winners" % entry["id"])
	for id in IDS:
		_check(listed.has(id), "%s listed" % id)
	var args := HistoricalBattlesMenu.args_for("crecy", "defender")
	_check(args.has("--historical=crecy") and args.has("--historical-side=defender"), "args: %s" % str(args))
	# Chaque carte se construit dans la simulation, avec son scénario et ses vagues.
	for id in IDS:
		var sim: Object = ClassDB.instantiate("BattleSim")
		var ok: bool = sim.call("setup_historical", HistoricalBattlesMenu.data_dir(), id, "", 7)
		_check(ok, "%s: setup_historical" % id)
		if not ok:
			continue
		var info: Dictionary = sim.call("get_historical")
		_check(str(info.get("id", "")) == id and not bool(info.get("site_only", true)), "%s: scripted historical battle" % id)
		var waves: Array = sim.call("get_waves", "attacker")
		_check(waves.size() >= 3, "%s: successive French battles (%d)" % [id, waves.size()])
		_check(bool(waves[0]["released"]) and not bool(waves[waves.size() - 1]["released"]), "%s: first wave out, last held" % id)
		_check((sim.call("get_units") as Array).size() > 20, "%s: regiments deployed" % id)
		for _i in 60:
			sim.call("tick", 0.1)
	# Menu : une ligne et trois boutons par bataille.
	var menu := HistoricalBattlesMenu.new()
	get_root().add_child(menu)
	await process_frame
	for id in IDS:
		for side in ["attacker", "defender", ""]:
			_check(menu.buttons.has("%s:%s" % [id, side]), "button %s:%s" % [id, side])
	menu.queue_free()
	await process_frame
	if _failures == 0:
		print("ep7 historical OK")
	quit(1 if _failures > 0 else 0)


func _check(cond: bool, msg: String) -> bool:
	if not cond:
		_failures += 1
		push_error("ep7: " + msg)
	return cond
