extends "res://tests/smoke_ui.gd"

## Sections « bataille » : bataille rangée, siège, déploiement, embuscade (CV3).
## Découpage de `smoke.gd` (SC GT7) : les sections se chaînent par héritage et partagent l'état.
## Ne se lance pas seul : point d'entrée `res://tests/smoke.gd`.

## M7 (docs/design/m7-battles.md § 4) : bataille réelle France–Angleterre mise en scène par
## `debug_stage_battle`, ≤ 12 000 ticks headless de `BattleSim` (IA des deux camps), fin atteinte,
## `resolve_battle` accepté ; puis la boucle complète par la carte : dialogue d'avant-bataille,
## « Livrer bataille », scène `battle.tscn` 60 images, fin de bataille, « Retour à la campagne ».
func _run_battle() -> void:
	if not ClassDB.class_exists("BattleSim") or not ClassDB.instantiate("CampaignSim").has_method("debug_stage_battle"):
		_fail("battle: BattleSim / CampaignSim.debug_stage_battle not registered (run core/build.sh)")
		return
	var data_dir := _project_root().path_join("data")
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", data_dir, "fac_france", 1337), "battle: new_campaign failed"):
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	if not _check(armies.size() == 2, "battle: no French or English army"):
		return
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var pending: Array = sim.call("get_pending_battles")
	_check(index == 0 and pending.size() == 1, "battle: debug_stage_battle should record 1 pending battle, got %d" % pending.size())
	_check(str(pending[0].get("player_side", "")) == "attacker", "battle: France should attack")
	var setup: Dictionary = sim.call("get_battle_setup", index)
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not _check(battle.call("setup", setup, int(pending[0]["seed"])), "battle: BattleSim.setup refused the campaign setup"):
		return
	battle.call("set_ai", "attacker", true)
	var units: Array = battle.call("get_units")
	_check(units.size() == (setup["attacker"]["units"] as Array).size() + (setup["defender"]["units"] as Array).size(), "battle: unit count mismatch")
	var terrain: Dictionary = battle.call("get_terrain")
	_check((terrain["heights"] as PackedFloat32Array).size() == int(terrain["nx"]) * int(terrain["nz"]), "battle: terrain grid size")
	var soldiers: PackedFloat32Array = battle.call("get_soldier_transforms", "attacker")
	_check(soldiers.size() > 0 and soldiers.size() % 4 == 0, "battle: soldier transforms empty")
	var refused: Dictionary = battle.call("issue_command", {"type": "halt", "units": [units.size() - 1]})
	_check(not refused.get("ok", true), "battle: commanding an enemy unit should be refused")
	# F10b : ordres du chef (catalogue de data/battle_orders, cri de guerre propre à la faction).
	var orders: Array = battle.call("get_leader_orders", "attacker")
	# CB4 : quatre ordres, le pavois est devenu une capacité des arbalétriers.
	_check(orders.size() == 4, "battle: expected 4 leader's orders, got %d" % orders.size())
	# CB4 : capacités actives (catalogue de data/battle_abilities, état dans get_units).
	var catalog: Dictionary = battle.call("get_ability_catalog")
	_check(catalog.size() == 5, "battle: expected 5 abilities, got %d" % catalog.size())
	var with_abilities := 0
	for unit: Dictionary in units:
		_check(unit.has("abilities"), "battle: get_units entry without abilities")
		with_abilities += 1 if not (unit.get("abilities", []) as Array).is_empty() else 0
	_check(with_abilities > 0, "battle: no regiment has an ability")
	if not orders.is_empty():
		_check(str(orders[0]["label"]) == "Montjoie ! Saint-Denis !", "battle: French war cry label is %s" % orders[0]["label"])
		var cry: Dictionary = battle.call("issue_command", {"type": "leader_order", "order": "order_war_cry", "units": []})
		_check(bool(cry.get("ok", false)) or str(cry.get("error", "")).contains("impossible"), "battle: war cry command malformed: %s" % cry.get("error", "?"))
	var ticks := 0
	for _i in 12000:
		battle.call("tick", 0.1)
		ticks += 1
		if battle.call("is_finished"):
			break
	if not _check(battle.call("is_finished"), "battle: not finished after 12000 ticks"):
		return
	var outcome: Dictionary = battle.call("get_outcome")
	var events: Array = battle.call("get_events")
	_check(not events.is_empty(), "battle: no battle journal")
	var result: Dictionary = sim.call("resolve_battle", index, outcome)
	_check(result.get("ok", false), "battle: resolve_battle refused: %s" % result.get("error", "?"))
	_check((sim.call("get_pending_battles") as Array).is_empty(), "battle: pending battle should be gone")
	print("smoke battle: %d ticks, winner %s, losses %d / %d, %d journal lines" % [ticks, outcome["winner"], int(outcome["attacker"]["total_losses"]), int(outcome["defender"]["total_losses"]), events.size()])
	# EP9 (ADR 0056) : la même bataille sans aucun ordre du joueur (camp attaquant immobile, IA
	# adverse) se décide d'elle-même en moins de 20 minutes simulées (recette Q3 ; 12 min avant
	# le rythme plus lent des batailles rangées A6-L13b, ADR 0184 : horloges de refus ×1,9).
	var idle: Object = ClassDB.instantiate("BattleSim")
	if _check(idle.call("setup", setup, int(pending[0]["seed"])), "battle: idle BattleSim.setup refused"):
		for _i in 12000:
			idle.call("tick", 0.1)
			if idle.call("is_finished"):
				break
		_check(idle.call("is_finished"), "battle: no-order battle still undecided after 1200 simulated s")
		var idle_outcome: Dictionary = idle.call("get_outcome")
		_check(str(idle_outcome.get("end", "")) in ["rout", "broken", "refused", "lull"], "battle: no-order battle end is %s" % idle_outcome.get("end", "?"))
		print("smoke battle without orders: over at %d s, %s, winner %s" % [int(idle.call("get_elapsed")), idle_outcome.get("end", "?"), idle_outcome.get("winner", "?")])
	await _check_ambush_opening_cv3(setup, pending[0])

	# Boucle complète par la carte de campagne (vraies données).
	facade.set_data_dir(data_dir)
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null and facade.is_real, "battle: campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var map_armies: Array = BattleScene.main_armies(map.sim, "fac_france", "fac_england")
	map.sim.call("debug_stage_battle", map_armies[0], map_armies[1])
	map._offer_pending_battles()
	var dialog: Node = map._battle_dialog
	if not _check(dialog != null and dialog.visible, "battle: pre-battle dialog should be visible"):
		map.queue_free()
		return
	_check(str(dialog.body_label.text).contains("Météo prévue"), "battle: dialog should show the weather forecast")
	_check(str(dialog.body_label.text).contains("Site : ") and dialog.site_label_text != "", "battle: dialog should show the battle site (B6)")
	dialog.fight_button.emit_signal("pressed")
	# AR1 : la bataille se construit derrière l'écran de chargement illustré (quelques images).
	var scene: Node = null
	for _frame in 120:
		await process_frame
		for child in root.get_children():
			if child is BattleScene:
				scene = child
		if scene != null:
			break
	var loading_card_shown := false
	for child in root.get_children():
		if child is BattleLoadingCard:
			loading_card_shown = true
	_check(loading_card_shown, "battle: illustrated loading card (AR1) should cover the battle build")
	if not _check(scene != null, "battle: battle.tscn not opened by « Livrer bataille »"):
		map.queue_free()
		return
	_check(not map.visible and map.process_mode == Node.PROCESS_MODE_DISABLED, "battle: campaign map should sleep during the battle")
	for _i in 60:
		await process_frame
	_check(scene.units.size() > 0, "battle scene: no units")
	_check(scene.terrain.get_child_count() > 0, "battle scene: no terrain")
	# B6 : même site dans le dialogue d'avant-bataille et dans le HUD.
	var site_text := str(scene.battle.call("get_site_label"))
	_check(site_text != "" and scene.hud.site_label.text == site_text and scene.hud.site_label.visible, "battle HUD: site line « %s » vs « %s »" % [scene.hud.site_label.text, site_text])
	_check(site_text == dialog.site_label_text, "battle: dialog site « %s » differs from the battle « %s »" % [dialog.site_label_text, site_text])
	var drawn := 0
	for key in scene._mm:
		drawn += (scene._mm[key] as MultiMeshInstance3D).multimesh.visible_instance_count
	_check(drawn > 0, "battle scene: no soldier instances")
	await _check_battle_deployment_f5c(scene)  # F5c
	_check_battle_hud_f5b(scene)
	_check_battle_markers_b2(scene)
	await _check_battle_music_camera_b3(scene)  # B3
	# Un ordre du joueur via l'API de la scène, puis fin de bataille accélérée.
	var own: int = -1
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side:
			own = int(unit["id"])
			break
	var order: Dictionary = scene.issue({"type": "move", "units": [own], "x": 600.0, "z": 330.0, "run": false})
	_check(order.get("ok", false), "battle scene: move order refused: %s" % order.get("error", "?"))
	scene.battle.call("set_ai", scene.player_side, true)
	for _i in 36000:
		scene.battle.call("tick", 0.1)
		if scene.battle.call("is_finished"):
			break
	await process_frame
	await process_frame
	_check(scene.finished_shown and scene.result_screen != null and scene.result_screen.visible, "battle scene: end screen should be visible")
	if scene.result_screen != null:
		var screen: BattleResultScreen = scene.result_screen
		_check(screen.title_label.text.begins_with("Victoire") or screen.title_label.text.begins_with("Défaite"), "battle result: verdict title is %s" % screen.title_label.text)
		var regiments := 0
		for unit in scene.units:
			if not bool(unit.get("synthetic", false)):
				regiments += 1
		_check(screen.row_count() == regiments, "battle result: %d loss rows for %d regiments" % [screen.row_count(), regiments])
		_check(screen.mentions_box.get_child_count() > 0, "battle result: no notable mentions")
		_check(BattleResultScreen.verdict(true, 0.1, 0.7) == "Victoire décisive" and BattleResultScreen.verdict(false, 0.8, 0.1) == "Défaite écrasante", "battle result: verdict thresholds")
		screen.return_button.emit_signal("pressed")  # « Retour à la campagne »
	else:
		scene._on_return()
	await process_frame
	_check(map.visible and map.process_mode == Node.PROCESS_MODE_INHERIT, "battle: campaign map should be back after the battle")
	_check((map.sim.call("get_pending_battles") as Array).is_empty(), "battle: pending battle should be resolved after « Retour à la campagne »")
	_check(map.ui.log_line_count() > 0, "battle: campaign journal should list the battle")
	if failures == 0:
		print("smoke OK: battle (headless %d ticks, scene 60 frames, %d soldiers drawn, resolved through the map)" % [ticks, drawn])
	map.queue_free()
	await process_frame


## F5b : cartes compactes rangées par « bataille », groupe Ctrl+1 enregistré puis rappelé,
## minicarte présente et peuplée, boutons de vitesse, noms coupés entre deux mots.
func _check_battle_hud_f5b(scene: BattleScene) -> void:
	var hud: BattleHud = scene.hud
	var own := 0
	var first := -1
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side:
			own += 1
			if first < 0:
				first = int(unit["id"])
	_check(hud.card_count() == own and own > 0, "battle HUD: %d cards for %d own units" % [hud.card_count(), own])
	_check(BattleGroups.ORDER.has(hud.card_battle(first)), "battle HUD: card not in a battle column")
	var card: UnitCard = hud._cards[first]
	_check(card.custom_minimum_size.x <= 72.0 and card.tooltip_text.contains("Formation"), "battle HUD: card should be compact with formation in its tooltip")
	var font := card.get_theme_default_font()
	var fitted := UnitCard.fit_name("Arbalétriers génois de la compagnie Grimaldi", font, 10, 63.0, 2)
	_check(fitted.split("\n").size() <= 2 and not fitted.contains("Grimaldi"), "battle HUD: name fitting should stop on a whole word: %s" % fitted)
	scene.selected.clear()
	scene.selected.append(first)
	scene.handle_group_key(1, true)
	scene.selected.clear()
	scene.handle_group_key(1, false)
	_check(scene.selected.size() == 1 and scene.selected[0] == first, "battle HUD: group 1 should be recalled (got %s)" % [scene.selected])
	_check(hud.minimap != null and hud.minimap.is_visible_in_tree(), "battle HUD: minimap missing")
	scene._refresh_view(true)
	_check(hud.minimap.dot_count() > own, "battle HUD: minimap should show both sides")
	scene._on_speed_pressed(1)
	_check(hud.active_speed() == 1 and not scene.paused, "battle HUD: ×2 speed button not active")
	scene._on_speed_pressed(0)
	_check(hud.help_panel != null and not hud.help_panel.visible, "battle HUD: F1 help should start hidden")


## B2 : bannières flottantes (un repère par régiment présent à l'écran, clic = sélection, touche
## de masquage) et cartes-vignettes (effectif, infobulle riche).
func _check_battle_markers_b2(scene: BattleScene) -> void:
	var markers: BattleUnitMarkers = scene.markers
	if not _check(markers != null and markers.is_inside_tree(), "battle markers: missing"):
		return
	scene._refresh_view(true)
	_check(markers.marker_count() > 0, "battle markers: no floating banner drawn")
	var own := -1
	for unit in scene.units:
		var id := int(unit["id"])
		if str(unit["side"]) == scene.player_side and markers.marker_rect(id).size.x > 0.0 and markers.marker_at(markers.marker_rect(id).get_center()) == id:
			own = id
			break
	if _check(own >= 0, "battle markers: no clickable banner for the player's regiments"):
		var rect := markers.marker_rect(own)
		_check(markers._has_point(rect.get_center()) and not markers._has_point(Vector2(-5000, -5000)), "battle markers: hit test")
		var click := InputEventMouseButton.new()
		click.button_index = MOUSE_BUTTON_LEFT
		click.pressed = true
		click.position = rect.get_center()
		scene.selected.clear()
		markers._gui_input(click)
		_check(scene.selected.size() == 1 and scene.selected[0] == own, "battle markers: clicking a banner should select its regiment (got %s)" % [scene.selected])
	# Pas de chevauchement entre repères.
	var ids := markers._placed.keys()
	var overlaps := 0
	for i in ids.size():
		for j in range(i + 1, ids.size()):
			if markers.marker_rect(ids[i]).intersects(markers.marker_rect(ids[j])):
				overlaps += 1
	_check(overlaps <= ids.size() / 4, "battle markers: %d overlapping banners out of %d" % [overlaps, ids.size()])
	# B7 : vue très lointaine, toutes les troupes au même point écran → une pastille par camp ;
	# un clic sur celle du joueur sélectionne tout le groupe.
	var saved_selection: Array = scene.selected.duplicate()
	var same := {}
	var own_count := 0
	for unit in scene.units:
		if bool(unit["present"]):
			same[int(unit["id"])] = Vector2(400, 300)
			own_count += 1 if str(unit["side"]) == scene.player_side else 0
	markers.update(scene.units, same, [], 800.0)
	_check(markers.clustered and markers.marker_count() == 2, "battle markers: far view should cluster into one pastille per side (got %d)" % markers.marker_count())
	var own_key := -1
	for key in markers._placed.keys():
		if str(markers._placed[key]["unit"]["side"]) == scene.player_side:
			own_key = key
	if _check(own_key >= 0, "battle markers: no cluster for the player's side"):
		var group_click := InputEventMouseButton.new()
		group_click.button_index = MOUSE_BUTTON_LEFT
		group_click.pressed = true
		group_click.position = markers.marker_rect(own_key).get_center()
		scene.selected.clear()
		markers._gui_input(group_click)
		_check(scene.selected.size() == own_count, "battle markers: clicking a cluster should select its %d regiments (got %d)" % [own_count, scene.selected.size()])
	markers.update(scene.units, same, [], 100.0)
	_check(not markers.clustered, "battle markers: near view should not cluster")
	scene.selected.clear()
	scene.selected.append_array(saved_selection)
	markers.toggle()
	scene._refresh_view(true)
	_check(not markers.visible and markers.marker_count() == 0, "battle markers: toggle should hide the banners")
	markers.toggle()
	scene._refresh_view(true)
	_check(markers.marker_count() > 0, "battle markers: toggle should show the banners again")
	var card: UnitCard = scene.hud._cards.values()[0]
	_check(card.custom_minimum_size.y >= 80.0 and card.art != null, "battle cards: thumbnail card expected")
	_check(card.tooltip_text.contains("État"), "battle cards: rich tooltip should carry the state")


## B3 (T4 musique dynamique, T6 caméra de suivi) : changement d'état musical déclenché par un
## contact simulé (fonction pure `BattleMusicDirector.compute_state`) et son hystérésis ; la
## caméra suit la position d'un régiment sur plusieurs ticks, s'oriente vers lui, se libère au
## clavier, et le double-clic sur une carte recentre la caméra dessus.
func _check_battle_music_camera_b3(scene: BattleScene) -> void:
	const MUSIC := preload("res://scripts/battle/battle_music.gd")
	if not _check(scene.music != null, "battle music: BattleMusicDirector missing on the scene"):
		return
	_check(scene.music.current_state == "approach", "battle music: should start calm (approach), got %s" % scene.music.current_state)

	# Contact simulé : la fonction pure d'intensité doit distinguer les quatre états.
	var calm := [{"side": "attacker", "state": "marching", "morale": 0.9, "present": true}, {"side": "defender", "state": "idle", "morale": 0.9, "present": true}]
	_check(MUSIC.compute_state(calm, false, false) == "approach", "battle music: no contact should stay calm")
	var contact := [{"side": "attacker", "state": "melee", "morale": 0.8, "present": true}, {"side": "defender", "state": "melee", "morale": 0.8, "present": true}]
	_check(MUSIC.compute_state(contact, false, false) == "engagement", "battle music: melee contact should be an engagement")
	var breaking := [{"side": "attacker", "state": "melee", "morale": 0.8, "present": true}, {"side": "defender", "state": "routing", "morale": 0.1, "present": true}, {"side": "defender", "state": "routing", "morale": 0.1, "present": false}]
	_check(MUSIC.compute_state(breaking, false, false) == "critical", "battle music: a side close to routing should be critical")
	_check(MUSIC.compute_state([], true, true) == "victory" and MUSIC.compute_state([], true, false) == "defeat", "battle music: a finished battle should give victory / defeat")

	# Hystérésis : monter en intensité est immédiat, redescendre exige une intensité stable.
	scene.music.current_state = "approach"
	scene.music._pending_state = "approach"
	scene.music._pending_elapsed = 0.0
	scene.music._advance("critical", 10.0)
	_check(scene.music.current_state == "critical", "battle music: escalation should be immediate")
	scene.music._advance("engagement", 1.0)
	_check(scene.music.current_state == "critical", "battle music: de-escalation should wait out the hysteresis")
	scene.music._advance("engagement", 3.0)
	_check(scene.music.current_state == "engagement", "battle music: de-escalation should commit once the new state is stable")

	# Caméra de suivi (T6) : verrouillage sur la sélection, suivi doux, libération manuelle.
	var own := -1
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side and bool(unit["present"]):
			own = int(unit["id"])
			break
	if not _check(own >= 0, "battle camera: no player regiment to follow"):
		return
	scene.selected = [own]
	scene.camera_rig.target = Vector3(0, 0, 0)
	var start_distance := scene.camera_rig.target.distance_to(scene._unit_world_position(own))
	scene._toggle_camera_follow()
	_check(scene.camera_rig.is_following() and scene.camera_rig.follow_id == own, "battle camera: C should lock onto the selected regiment")
	# Ticks manuels (delta fixe) plutôt que des images réelles : la convergence du lissage ne doit
	# pas dépendre du débit d'images de l'exécution headless.
	for _i in 20:
		scene.camera_rig._process(0.3)
	var followed_distance := scene.camera_rig.target.distance_to(scene._unit_world_position(own))
	_check(followed_distance < start_distance * 0.5, "battle camera: target should have closed in on the followed regiment (%.1f -> %.1f)" % [start_distance, followed_distance])
	scene._toggle_camera_follow()
	_check(not scene.camera_rig.is_following(), "battle camera: C again should release the follow")

	# Double-clic sur une carte d'unité : centre la caméra sur ce régiment (jump, sans suivi).
	# Comparaison au sol (x, z) : `look_at_point` / `_apply()` replaquent `target.y` sur le terrain.
	scene.camera_rig.target = Vector3(0, 0, 0)
	scene._on_card_double_clicked(own)
	var after_pos: Vector3 = scene._unit_world_position(own)
	var ground_gap := Vector2(scene.camera_rig.target.x, scene.camera_rig.target.z).distance_to(Vector2(after_pos.x, after_pos.z))
	_check(ground_gap < 1.0, "battle camera: double-click on a card should center the camera on the regiment (gap %.2f)" % ground_gap)
	_check(not scene.camera_rig.is_following(), "battle camera: double-click should not itself start a follow")
	await process_frame


## M8 § 2 : bataille de siège réelle (armée française devant la Guyenne anglaise), headless puis
## dans la scène 3D.
func _run_siege_battle() -> void:
	if not ClassDB.instantiate("CampaignSim").has_method("debug_stage_siege"):
		_fail("siege battle: CampaignSim.debug_stage_siege not registered (run core/build.sh)")
		return
	var data_dir := _project_root().path_join("data")
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", data_dir, "fac_france", 1337), "siege battle: new_campaign failed"):
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	var pending: Array = sim.call("get_pending_battles")
	if not _check(index == 0 and pending.size() == 1 and bool(pending[0].get("siege", false)), "siege battle: debug_stage_siege should record 1 pending siege battle"):
		return
	_check(int(pending[0].get("defender_strength", 0)) > 0, "siege battle: the garrison should have soldiers")
	var setup: Dictionary = sim.call("get_battle_setup", index)
	_check(setup.has("siege"), "siege battle: setup without siege parameters")
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not _check(battle.call("setup", setup, int(pending[0]["seed"])), "siege battle: BattleSim.setup refused the siege setup"):
		return
	battle.call("set_ai", "attacker", true)
	var siege: Dictionary = battle.call("get_siege")
	_check((siege.get("pieces", []) as Array).size() >= 10, "siege battle: wall pieces missing")
	_check((battle.call("get_terrain") as Dictionary).has("siege"), "siege battle: get_terrain should carry the walls")
	var on_wall := 0
	for unit in battle.call("get_units"):
		if bool(unit.get("on_wall", false)):
			on_wall += 1
	_check(on_wall > 0, "siege battle: no defender on the walls")
	var ticks := 0
	for _i in 36000:
		battle.call("tick", 0.1)
		ticks += 1
		if battle.call("is_finished"):
			break
	if not _check(battle.call("is_finished"), "siege battle: not finished after 36000 ticks"):
		return
	var outcome: Dictionary = battle.call("get_outcome")
	var integrity := float((battle.call("get_siege") as Dictionary).get("integrity", 1.0))
	var result: Dictionary = sim.call("resolve_battle", index, outcome)
	_check(result.get("ok", false), "siege battle: resolve_battle refused: %s" % result.get("error", "?"))
	_check((sim.call("get_pending_battles") as Array).is_empty(), "siege battle: pending battle should be gone")
	print("smoke siege battle: %d ticks, winner %s, losses %d / %d, walls %d %%" % [ticks, outcome["winner"], int(outcome["attacker"]["total_losses"]), int(outcome["defender"]["total_losses"]), int(integrity * 100.0)])

	# La scène 3D sur un second assaut.
	var sim2: Object = ClassDB.instantiate("CampaignSim")
	sim2.call("new_campaign", data_dir, "fac_france", 1338)
	var armies2: Array = BattleScene.main_armies(sim2, "fac_france", "fac_england")
	var index2: int = sim2.call("debug_stage_siege", armies2[0], "prov_guyenne")
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim2, index2, 7)
	root.add_child(scene)
	for _i in 40:
		await process_frame
	if not _check(scene.siege_view != null and scene.siege_view.get_child_count() > 10, "siege scene: walls not built"):
		scene.queue_free()
		return
	_check(scene.hud.siege_panel.visible, "siege scene: siege status should be shown")
	await _check_siege_f5c(scene)  # F5c
	scene.battle.call("set_ai", scene.player_side, true)
	for _i in 36000:
		scene.battle.call("tick", 0.1)
		if scene.battle.call("is_finished"):
			break
	for _i in 3:
		await process_frame
	_check(scene.finished_shown, "siege scene: end screen should be visible")
	scene._on_return()
	await process_frame
	_check((sim2.call("get_pending_battles") as Array).is_empty(), "siege scene: pending siege should be resolved")
	if failures == 0:
		print("smoke OK: siege battle (headless %d ticks, scene with %d wall nodes, resolved)" % [ticks, scene.siege_view.get_child_count()])
	scene.queue_free()
	await process_frame


# --- F3 : écrans et flux -----------------------------------------------------------


## CV3-2 : campagne factice qui ne sert que le setup d'une bataille (dialogue d'avant-bataille).
class _SetupOnlyCampaign extends RefCounted:
	var battle_setup: Dictionary = {}

	func get_battle_setup(_index: int) -> Dictionary:
		return battle_setup


## CV3-2 : le setup de campagne passé en embuscade (le défenseur surpris en colonne) puis en camp
## retranché : BattleSim (ouverture, zones, colonne, journal, palissade) et dialogue d'avant-bataille
## (titre « Embuscade ! », mentions des postures).
func _check_ambush_opening_cv3(base_setup: Dictionary, pending_battle: Dictionary) -> void:
	var ambush: Dictionary = base_setup.duplicate(true)
	ambush["opening"] = {"kind": "ambush", "victim": "defender"}
	var seed := int(pending_battle["seed"])
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not _check(battle.call("setup", ambush, seed), "ambush: BattleSim.setup refused the ambush setup"):
		return
	var opening: Dictionary = battle.call("get_opening")
	_check(str(opening.get("kind", "")) == "ambush" and str(opening.get("victim", "")) == "defender", "ambush: get_opening is %s" % opening)
	_check(not bool(opening.get("defender_can_deploy", true)) and bool(opening.get("attacker_can_deploy", false)), "ambush: only the ambusher deploys")
	_check((battle.call("get_deployment_zone", "defender") as Dictionary).is_empty(), "ambush: the column has a deployment zone")
	var zones: Array = battle.call("get_deployment_zones", "attacker")
	_check(zones.size() in [1, 2], "ambush: ambusher zones %d" % zones.size())
	var column := 0
	var victims := 0
	for unit in battle.call("get_units"):
		if str(unit["side"]) == "defender" and bool(unit.get("present", true)):
			victims += 1
			if str(unit.get("formation", "")) == "column":
				column += 1
	_check(victims > 0 and column == victims, "ambush: %d of %d victim regiments in column" % [column, victims])
	var journal := false
	for event in battle.call("get_events"):
		if str(event.get("text_fr", "")).begins_with("Embuscade !"):
			journal = true
	_check(journal, "ambush: no « Embuscade ! » journal line")
	var entrenched: Dictionary = base_setup.duplicate(true)
	(entrenched["defender"] as Dictionary)["entrenched"] = true
	var camp: Object = ClassDB.instantiate("BattleSim")
	if _check(camp.call("setup", entrenched, seed), "entrenched: BattleSim.setup refused"):
		var palisades := 0
		for o in (camp.call("get_terrain") as Dictionary).get("obstacles", []):
			if str(o["kind"]) == "palisade":
				palisades += 1
		_check(palisades > 0, "entrenched: no palisade in get_terrain")
	# Dialogue d'avant-bataille sur ce setup (campagne factice).
	var fake := _SetupOnlyCampaign.new()
	fake.battle_setup = ambush
	var dialog: PreBattleDialog = (load("res://scenes/battle/pre_battle_dialog.tscn") as PackedScene).instantiate()
	root.add_child(dialog)
	await process_frame
	var entry := pending_battle.duplicate()
	entry["player_side"] = "attacker"
	dialog.show_battle(fake, entry)
	_check(str(dialog.title_label.text).begins_with("Embuscade !"), "ambush: dialog title « %s »" % dialog.title_label.text)
	_check(str(dialog.subtitle_label.text).contains("colonne de marche"), "ambush: dialog subtitle « %s »" % dialog.subtitle_label.text)
	_check(str(dialog.modifiers_label.text).contains("Embuscade !"), "ambush: dialog modifiers « %s »" % dialog.modifiers_label.text)
	fake.battle_setup = entrenched
	dialog.show_battle(fake, entry)
	_check(not str(dialog.title_label.text).begins_with("Embuscade"), "entrenched: dialog title « %s »" % dialog.title_label.text)
	_check(str(dialog.modifiers_label.text).contains("Camp retranché"), "entrenched: dialog modifiers « %s »" % dialog.modifiers_label.text)
	dialog.queue_free()
	await process_frame
	if failures == 0:
		print("smoke OK: ambush opening (%d victim regiments in column, %d flank zone(s)), entrenched camp, dialog « Embuscade ! »" % [victims, zones.size()])


## Phase de déploiement ouverte par une bataille du joueur : temps gelé, zone dessinée, un
## placement valide et un refusé (toast), « Commencer la bataille », puis le temps avance.
func _check_battle_deployment_f5c(scene: BattleScene) -> void:
	var controller: DeploymentController = scene.deployment
	if not _check(controller != null and controller.active and bool(scene.battle.call("is_deploying")), "deployment: phase should be open for a player battle"):
		return
	_check(controller.zone_view != null and controller.zone_view.get_child_count() == 1, "deployment: zone not drawn (A1-02: one shader mesh)")
	_check(controller.banner != null and controller.banner.is_visible_in_tree(), "deployment: banner missing")
	_check(float(scene.battle.call("get_elapsed")) == 0.0, "deployment: time should be frozen")
	var zone: Dictionary = controller.zone
	var own := -1
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side and bool(unit["present"]):
			own = int(unit["id"])
			break
	var inside := Vector3((float(zone["x0"]) + float(zone["x1"])) * 0.5, 0, (float(zone["z0"]) + float(zone["z1"])) * 0.5)
	var cam := inside + Vector3(0, 200, -300)
	_check(controller.place([own], inside, inside, cam) == 1, "deployment: placement inside the zone refused")
	scene._refresh_view(true)
	var moved: Dictionary = controller._unit(own)
	_check(Vector2(float(moved["x"]), float(moved["z"])).distance_to(Vector2(inside.x, inside.z)) < 1.0, "deployment: unit not moved to the zone centre")
	var outside := Vector3(inside.x, 0, float(zone["z1"]) + 200.0 if scene.player_side == "attacker" else float(zone["z0"]) - 200.0)
	_check(controller.place([own], outside, outside, cam) == 0, "deployment: placement outside the zone accepted")
	_check(scene.hud.toast_label != null and scene.hud.toast_label.text.contains("zone de déploiement"), "deployment: refusal toast missing")
	_check(controller.finish() and not controller.active, "deployment: start_battle failed")
	_check(not bool(scene.battle.call("is_deploying")), "deployment: still deploying after start_battle")
	for _i in 10:
		await process_frame
	scene.battle.call("tick", 0.5)
	_check(float(scene.battle.call("get_elapsed")) > 0.0, "deployment: battle does not progress after start")
	if failures == 0:
		print("smoke OK: deployment (zone %s, placement valid + refused, battle started)" % [zone])


## Siège (F5c) : les maisons rendues sont exactement les disques de la simulation, puis le
## déploiement est validé comme en bataille rangée.
func _check_siege_f5c(scene: BattleScene) -> void:
	var houses: Array = (scene.battle.call("get_siege") as Dictionary).get("houses", [])
	var sites: Array = scene.siege_view.house_sites
	var same := houses.size() == sites.size() and not houses.is_empty()
	for i in mini(houses.size(), sites.size()):
		var p: Vector2 = sites[i]["p"]
		same = same and p.distance_to(Vector2(float(houses[i]["x"]), float(houses[i]["z"]))) < 0.01
	_check(same, "siege scene: %d houses rendered for %d simulation houses" % [sites.size(), houses.size()])
	await _check_battle_deployment_f5c(scene)
	_check(not scene.hud.siege_label.text.contains("sortie"), "siege scene: no sortie at the start")
	# FB1 : plafond de figurines (taille des unités abaissée au-delà du plafond).
	_check(is_equal_approx(BattleScene.capped_figure_scale(2.5, 4000, 15000), 2.5), "figure budget: small battle kept at Ultra")
	_check(is_equal_approx(BattleScene.capped_figure_scale(2.5, 10000, 15000), 1.5), "figure budget: large battle not capped")
	_check(is_equal_approx(BattleScene.capped_figure_scale(1.0, 3000, 0), 1.0), "figure budget: zero budget should mean no cap")
	_check(Money.digits(15000) == "15" + Money.NBSP + "000", "settings: thousands separator")
	_check(BattleScene.siege_status({"pieces": [], "sortie": true}).contains("sortie de la garnison"), "siege scene: sortie not shown in the siege status")
