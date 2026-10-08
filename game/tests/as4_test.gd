extends SceneTree

## Lot AS4 : servants porteurs de munitions (rendu seulement). Vérifie :
## - les réglages `crew.haul` (schéma) : phases croissantes, tas et objets existants ;
## - l'étape du trajet suit la phase de rechargement (repos, aller, saisie, retour, charge) ;
## - le porteur s'éloigne du poste vers le tas puis revient, à une vitesse de marche plausible ;
## - l'objet n'est en main qu'à la saisie finie, au retour et au début de la charge ;
## - les tas sont dessinés même au repos ; `--no-as4` (banc A/B) ne dessine rien.
## Usage : godot --headless --path game --script res://tests/as4_test.gd [-- --no-as4]

var ok := true


func _check(cond: bool, what: String) -> void:
	if not cond:
		ok = false
		print("AS4 FAIL: ", what)


func _init() -> void:
	var off := CmdArgs.has("--no-as4")
	var crew := SiegeCrewFx.new()
	crew.cfg = SiegeEnginesFx.settings().get("crew", {})
	_check(not crew.cfg.is_empty() and crew.cfg.has("haul"), "réglages crew.haul")
	_check(crew.haul_enabled() == not off, "haul_enabled selon --no-as4")
	var haul: Dictionary = crew.cfg.get("haul", {})
	for model in ["trebuchet", "mangonel", "bombard"]:
		var spec: Dictionary = haul.get("by_engine", {}).get(model, {})
		_check(not spec.is_empty(), "%s : porteurs définis" % model)
		_check((spec.get("haulers", []) as Array).size() in [1, 2], "%s : un ou deux porteurs" % model)
		_check_engine(crew, model, spec, haul, off)
	_check_stage()
	crew.free()
	print("AS4 haulers (off=%s): %s" % [off, "OK" if ok else "FAIL"])
	quit(0 if ok else 1)


func _check_stage() -> void:
	var trip := [0.1, 0.3, 0.4, 0.6, 0.8]
	var want := {-1.0: "idle", 0.05: "idle", 0.2: "out", 0.35: "grab", 0.5: "back", 0.7: "drop", 0.9: "idle"}
	for phase in want:
		_check(str(SiegeCrewFx.haul_stage(trip, phase)["stage"]) == want[phase], "étape à la phase %.2f" % phase)
	_check(not bool(SiegeCrewFx.haul_stage(trip, 0.2)["holding"]), "rien en main à l'aller")
	_check(bool(SiegeCrewFx.haul_stage(trip, 0.5)["holding"]), "objet en main au retour")
	_check(not bool(SiegeCrewFx.haul_stage(trip, 0.78)["holding"]), "objet lâché en fin de charge")


func _check_engine(crew: SiegeCrewFx, model: String, spec: Dictionary, haul: Dictionary, off: bool) -> void:
	if off or spec.is_empty():
		crew.begin(0.0)
		crew.add_haulers(1, model, Transform3D.IDENTITY, 0.5, "attacker")
		crew.finish(null)
		_check(crew.shown == 0 and crew.props_shown == 0, "--no-as4 : rien de dessiné")
		return
	var piles: Array = spec["piles"]
	for h in spec["haulers"]:
		var trip: Array = h["trip"]
		for i in 4:
			_check(float(trip[i]) < float(trip[i + 1]), "%s : phases croissantes" % model)
		_check(int(h["pile"]) < piles.size(), "%s : tas existant" % model)
		_check(haul["objects"].has(piles[int(h["pile"])]["object"]), "%s : objet défini" % model)
		var pile := Vector2(float(piles[int(h["pile"])]["x"]), float(piles[int(h["pile"])]["z"]))
		var post := Vector2(float(h["post"][0]), float(h["post"][1]))
		# Vitesse moyenne aller et retour : de la marche (0,4 à 2,2 m/s), pas une téléportation.
		var period := 12.0
		var out_s := (float(trip[1]) - float(trip[0])) * period
		var back_s := (float(trip[3]) - float(trip[2])) * period
		_check(post.distance_to(pile) / out_s < 2.2 and post.distance_to(pile) / back_s < 2.2, "%s : vitesse de marche plausible" % model)
		_check(post.distance_to(pile) > 1.0, "%s : tas à distance du poste" % model)
	var h0: Dictionary = spec["haulers"][0]
	var trip0: Array = h0["trip"]
	var mid_back := (float(trip0[2]) + float(trip0[3])) * 0.5
	var frames: Dictionary = {}
	for phase in [-1.0, mid_back, 0.99]:
		crew.begin(1.0)
		crew.add_haulers(1, model, Transform3D.IDENTITY, phase, "attacker")
		frames[phase] = crew._props_frame.duplicate()
		crew.finish(null)
		_check(crew.shown == (spec["haulers"] as Array).size(), "%s : porteurs dessinés (phase %.2f)" % [model, phase])
	var ground_count := 0
	for pile in piles:
		ground_count += int(pile["count"])
	_check((frames[-1.0] as Array).size() == ground_count, "%s : seuls les tas au repos" % model)
	_check((frames[mid_back] as Array).size() == ground_count + 1 + (1 if (spec["haulers"] as Array).size() > 1 and _carrying_second(spec, mid_back) else 0), "%s : un objet en main au retour" % model)
	_check((frames[0.99] as Array).size() == ground_count, "%s : plus rien en main une fois chargé" % model)
	var held: Dictionary = {}
	for e in frames[mid_back]:
		if not bool(e["ground"]):
			held = e
	var hold: Array = h0["hold"]
	_check(not held.is_empty() and absf((held["xform"] as Transform3D).origin.y - float(hold[1])) < 0.01, "%s : objet porté à hauteur de main" % model)
	var pile0 := Vector2(float(piles[int(h0["pile"])]["x"]), float(piles[int(h0["pile"])]["z"]))
	var post0 := Vector2(float(h0["post"][0]), float(h0["post"][1]))
	_check(crew.props_shown >= ground_count, "%s : tas dessinés" % model)
	# Position du porteur : au poste au repos, au tas pendant la saisie.
	var at_rest := crew.haul_pose(h0["post"], pile0, SiegeCrewFx.haul_stage(trip0, -1.0))
	var at_pile := crew.haul_pose(h0["post"], pile0, SiegeCrewFx.haul_stage(trip0, (float(trip0[1]) + float(trip0[2])) * 0.5))
	_check(Vector2(at_rest.x, at_rest.y).distance_to(post0) < 0.001, "%s : au poste au repos" % model)
	_check(Vector2(at_pile.x, at_pile.y).distance_to(pile0) < 0.001, "%s : au tas pendant la saisie" % model)


func _carrying_second(spec: Dictionary, phase: float) -> bool:
	return bool(SiegeCrewFx.haul_stage(spec["haulers"][1]["trip"], phase)["holding"])
