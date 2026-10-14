extends TestCase

## Test headless du lot RG2 : aucune pastille d'alerte sans libellé ni clic mort, et la fenêtre
## de croisade se construit et se grise selon la raison fournie par le cœur.
## Usage : godot --headless --path game --script res://tests/rg2_test.gd


func _init() -> void:
	await process_frame
	_test_labels_and_routes()
	_test_crusade_window()
	finish()


func _test_labels_and_routes() -> void:
	var kinds: Array = []
	for kind in SeasonReport.KIND_STYLES.keys():
		kinds.append(str(kind))
	for kind in EndTurnCluster.KIND_ORDER:
		if not kinds.has(kind):
			kinds.append(kind)
	for kind in kinds:
		check(EndTurnCluster.short_label(kind) != "Avis", "kind %s has no short label" % kind)
		check(AlertRoutes.route_of({"kind": kind}) != "", "kind %s has no route" % kind)
	# Genre inconnu et alerte sans cible : le journal, jamais rien.
	check(AlertRoutes.route_of({"kind": "future_kind"}) == "journal", "unknown kind without target must open the journal")
	check(AlertRoutes.route_of({"kind": "future_kind", "province_id": "prov_x"}) == "focus", "unknown kind with a province focuses it")
	for kind in ["trade", "mission", "royal_act", "level_up", "loyalty", "captain", "crusade"]:
		check(AlertRoutes.route_of({"kind": kind}) != "journal", "%s should have a dedicated action" % kind)
	check(AlertRoutes.route_of({"kind": "trade"}) == "trade", "trade opens the trade panel")
	check(AlertRoutes.route_of({"kind": "mission"}) == "mission", "mission opens the objectives")


func _test_crusade_window() -> void:
	var view := {
		"target": "prov_safad", "target_name": "Jérusalem", "holder": "fac_mamluks", "holder_name": "Mamelouks",
		"turns_left": 12, "participants": [{"id": "fac_france", "name": "France"}], "joined": false,
		"can_join": true, "join_blocker": "", "join_favor": 5, "join_prestige": 3,
		"reward_favor": 15, "reward_prestige": 10, "reward_gold": 1000}
	var window := CrusadeWindow.new()
	root.add_child(window)
	window.show_call(view)
	check(window.visible, "window should be shown")
	var text := window.body_label.text
	for needle in ["Jérusalem", "Mamelouks", "France", "+5", "+15", "1000", "occuper"]:
		check(text.contains(needle), "body should mention %s" % needle)
	check(not window.join_button.disabled, "join enabled when the core allows it")
	var focused: Array = []
	window.target_requested.connect(func(p: String) -> void: focused.append(p))
	window.target_button.pressed.emit()
	check(focused == ["prov_safad"], "target button focuses the target")
	view["can_join"] = false
	view["join_blocker"] = "Seuls les princes catholiques peuvent prendre la croix."
	window.show_call(view)
	check(window.join_button.disabled and window.reason_label.visible and window.reason_label.text.contains("catholiques"), "join disabled with its reason")
	window.close()
	check(not window.visible, "close hides")
	window.queue_free()
	if ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_papal_crusade"):
		check(ClassDB.instantiate("CampaignSim").call("get_papal_crusade").is_empty(), "no call without a campaign")
