extends SceneTree

## Chantier IB (ADR 0109), lot IB3 : chaîne de bulles à Alt maintenu (spec
## `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md` § 3.2 et § 5), en headless :
##  - réglages `chain` lus dans `data/ui/tooltip_style.json` ;
##  - Alt sur un contrôle à infobulle → 1 bulle verrouillée par la chaîne ;
##  - survol simulé de 3 mots-clés successifs, Alt maintenu → chaîne de 4 (délai court, sous le
##    délai de survol simple) ; mot source surligné dans la parente ;
##  - un autre mot-clé de la même bulle remplace la branche ;
##  - Alt relâché : la chaîne reste tant que la souris est sur une bulle, puis se ferme après la
##    grâce une fois dehors ; une bulle épinglée au clic droit reste ;
##  - Échap → 0 bulle ; T sans infobulle n'est pas consommé (arbre des techniques) ;
##  - Alt+Maj (formations) n'ouvre rien.
## Usage : godot --headless --path game --script res://tests/ib_chain_test.gd

const ENABLED := true
const ALIAS_ENTRIES := ["cdx_peste_noire", "cdx_crecy"]
## Entre le délai de chaîne (0,12 s) et celui du survol simple (0,35 s).
const CHAIN_WAIT := 0.22
## Hors du contrôle à infobulle (dans la fenêtre headless de 64 px).
const OUTSIDE := Vector2(58, 58)

var _failures := 0
var _bubbles: Node
var _store: Node


func _init() -> void:
	if not ENABLED:
		print("ib_chain_test: disabled (IB0 skeleton)")
		quit(0)
		return
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("_apply_ui_scale")
	_store = root.get_node_or_null("/root/CodexStore")
	_bubbles = root.get_node_or_null("/root/CodexBubbles")
	if _check(_store != null and _bubbles != null, "CodexStore / CodexBubbles autoloads missing"):
		_store.call("use_test_file")
		await _run()
		await _run_ib4()
	print("ib_chain_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("ib_chain_test: " + message)
	return condition


func _run() -> void:
	var script: Script = _bubbles.get_script()
	script.call("reload_chain_settings")
	_check(is_equal_approx(float(script.call("chain_setting", "hover_delay_s")), 0.12), "chain.hover_delay_s should come from tooltip_style.json")
	_check(int(script.call("chain_setting", "max_bubbles")) == 10, "chain.max_bubbles should come from tooltip_style.json (10)")
	_check(is_equal_approx(float(script.call("chain_setting", "close_grace_s")), 0.4), "chain.close_grace_s should be 0.4")

	var aliases := PackedStringArray()
	for id: String in ALIAS_ENTRIES:
		aliases.append(str(_store.call("title", id)))
	var holder := PanelContainer.new()
	holder.tooltip_text = "En 1348, %s ravage le royaume ; avant, %s." % [aliases[0], aliases[1]]
	holder.mouse_filter = Control.MOUSE_FILTER_STOP
	# Fenêtre headless de 64 × 64 px (non redimensionnable) : tout se passe dans le coin.
	holder.position = Vector2.ZERO
	holder.custom_minimum_size = Vector2(40, 30)
	var inner := Label.new()
	inner.text = "survol"
	inner.mouse_filter = Control.MOUSE_FILTER_PASS
	holder.add_child(inner)
	root.add_child(holder)
	await process_frame
	_bubbles.call("close_all")

	# Alt+Maj (formations) au-dessus du contrôle : rien ne s'ouvre.
	_move_mouse(holder.get_global_rect().get_center())
	await process_frame
	_key(KEY_ALT, true, true)
	await process_frame
	_check(_count() == 0 and not bool(_bubbles.call("explore_held")), "Alt+Shift should open nothing (count %d)" % _count())
	_key(KEY_ALT, false)

	# Alt seul : l'infobulle du contrôle devient une bulle verrouillée par la chaîne.
	_key(KEY_ALT, true)
	await process_frame
	if not _check(_count() == 1, "Alt over a control with a tooltip should open 1 bubble (count %d)" % _count()):
		_key(KEY_ALT, false)
		return
	var first := _bubble(0)
	_check(bool(first.get_meta("pinned", false)) and bool(first.get_meta("chain_locked", false)), "the converted tooltip should be locked by the chain")
	_check(bool(_bubbles.call("explore_held")), "Alt should be held")

	# Trois mots-clés successifs, Alt maintenu → chaîne de 4.
	var links := _links(first)
	_check(links.size() >= 2, "the converted tooltip should carry 2 links (%s)" % str(links))
	var second := await _hover(first, _linked_meta(first, links))
	var third := await _hover(second, _linked_meta(second, _links(second))) if second != null else null
	var fourth := await _hover(third, _linked_meta(third, _links(third))) if third != null else null
	_check(fourth != null and _count() == 4, "3 chained keyword hovers should give a chain of 4 (count %d)" % _count())
	for bubble in [second, third, fourth]:
		if bubble != null:
			_check(bool(bubble.get_meta("chain_locked", false)), "chained children should be locked by the chain")
	if second != null:
		_check(_bubbles.call("parent_of", second) == first, "the second bubble should be a child of the first")
	var first_text := _text(first)
	_check(first_text != null and first_text.text.contains("[bgcolor="), "the source keyword should stay highlighted in the parent")

	# Un autre mot-clé de la première bulle remplace la branche.
	var free_meta := str(links[0]) if not links.is_empty() else ""
	if second != null and links.size() >= 2:
		var other := str(links[1]) if str(links[0]) == str(second.get_meta("source_meta", "")) else str(links[0])
		free_meta = str(second.get_meta("source_meta", ""))
		var replaced := await _hover(first, other)
		_check(replaced != null and _count() == 2 and _bubbles.call("parent_of", replaced) == first,
			"hovering another keyword should replace the branch (count %d)" % _count())
		await process_frame
		var highlighted := first_text.text.count("[bgcolor=") if first_text != null else 0
		_check(highlighted >= 1 and first_text.text.contains("[url=%s][bgcolor=" % other), "the highlight should follow the new source keyword")

	# Sans Alt : délai de survol simple (rien au bout de CHAIN_WAIT).
	# Une bulle épinglée durablement (clic droit, T) reste après le relâché.
	_bubbles.call("set_pinned", first, true)
	_check(not bool(first.get_meta("chain_locked", false)), "pinning should turn a chain lock into a lasting pin")
	_key(KEY_ALT, false)
	var before := _count()
	var idle := await _hover(first, free_meta)
	_check(idle == null and _count() == before, "without Alt, a hover shorter than the idle delay opens nothing")
	# Alt relâché : la chaîne reste tant que la souris est sur une bulle…
	var top := _bubble(_count() - 1)
	_move_mouse(top.get_global_rect().get_center())
	await process_frame
	if _bubbles.call("_bubble_under_mouse") != null:
		await create_timer(0.6).timeout
		_check(_count() == before, "released chain should stay while the mouse is on a bubble (count %d)" % _count())
		_move_mouse(OUTSIDE)
	else:
		print("ib_chain_test: mouse position not tracked in headless, 'stay on bubble' check skipped")
	# … puis, souris hors des bulles, se ferme après la grâce (pas avant).
	await create_timer(0.15).timeout
	_check(_count() == before, "chain should not close before the grace delay (count %d)" % _count())
	await create_timer(0.5).timeout
	_check(_count() == 1 and _bubble(0) == first, "released chain should close after the grace, pinned bubble kept (count %d)" % _count())

	# Échap ferme tout.
	_key(KEY_ALT, true)
	_move_mouse(holder.get_global_rect().get_center())
	await process_frame
	_key(KEY_ALT, false)
	_key(KEY_ESCAPE, true)
	await process_frame
	_check(_count() == 0, "Escape should close every bubble (count %d)" % _count())
	_key(KEY_ESCAPE, false)

	# T hors infobulle : non consommé (l'arbre des techniques s'ouvre).
	_move_mouse(OUTSIDE)
	await process_frame
	_check(not bool(_bubbles.call("pin_current")), "T with no tooltip should not be consumed (tech tree)")
	_key(KEY_T, true)
	await process_frame
	_key(KEY_T, false)
	_check(_count() == 0, "T with no tooltip should open no bubble")

	_bubbles.call("close_all")
	holder.queue_free()
	await process_frame


## IB4 (spec IB § 3.3-3.4) : liens `ib:` émis par les specs ; lien d'entité → bulle riche avec
## en-tête ; `ib:rule:` → bulle de règle ; fille à côté de sa parente à 1280 × 720 ; fil d'Ariane
## au 3ᵉ niveau (clic : retour à ce niveau) ; réduction des ancêtres plutôt que fermeture.
func _run_ib4() -> void:
	_bubbles.call("close_all")
	RichTooltip.reload_texts()
	# Liens émis : libellés d'effets / stats en `ib:rule:`, noms d'entités en `ib:<kind>:`.
	var unit_view := TooltipView.build(RichTooltip.unit_spec("unit_longbowmen"), true)
	root.add_child(unit_view)
	var stats_text := ""
	for node in unit_view.find_children("Label", "RichTextLabel", true, false):
		stats_text += (node as RichTextLabel).text
	_check(stats_text.contains("[url=ib:rule:melee]"), "stat labels should link to ib:rule: (%s)" % stats_text.left(120))
	var conditions := unit_view.find_child("Conditions", true, false) as RichTextLabel
	_check(conditions != null and conditions.text.contains("[url=ib:"), "entity names in conditions should be ib: links")
	unit_view.queue_free()
	var building_id := _building_with_effects()
	var building_view := TooltipView.build(RichTooltip.building_spec(building_id), true)
	root.add_child(building_view)
	var effects := building_view.find_child("Effects", true, false) as RichTextLabel
	var headline_links := ""
	for node in building_view.find_children("Caption", "RichTextLabel", true, false):
		headline_links += (node as RichTextLabel).text
	_check(effects == null and headline_links.contains("[url=ib:rule:") or effects != null and effects.text.contains("[url=ib:rule:"),
		"%s: effect labels should link to ib:rule:" % building_id)
	building_view.queue_free()
	_check(CodexText.ib_link("rule", "army_morale", "Moral").contains("[url=ib:rule:army_morale]"), "CodexText.ib_link")
	# Défauts IB1 : chiffre vedette « Tir » avec icône ; ambiance en italique atténué.
	var recruit_view := TooltipView.build(RichTooltip.unit_spec("unit_longbowmen", {"cost": 420, "upkeep": 40}), true)
	root.add_child(recruit_view)
	var badge := recruit_view.find_child("Badge_melee_or_ranged", true, false)
	_check(badge != null and not badge.find_children("Icon_*", "", true, false).is_empty(), "recruit headline stat badge should carry an icon")
	var flavour := recruit_view.find_child("Flavour", true, false) as RichTextLabel
	_check(flavour != null and flavour.has_theme_font_override("normal_font") and flavour.get_theme_color("default_color") == Color(TooltipView.FLAVOUR_COLOR),
		"flavour text should be italic and muted")
	recruit_view.queue_free()

	# Lien d'entité → bulle riche (en-tête), clé retenue ; règle → bulle `rule`.
	_bubbles.set("area_override", Rect2(0, 0, 1280, 720))
	var entity: PanelContainer = _bubbles.call("open", "ib:unit:unit_longbowmen", Vector2(8, 8))
	await _settle()
	_check(entity != null and entity.find_child("Header", true, false) != null, "ib:unit: should open a rich bubble with a header")
	if entity != null:
		_check(str(entity.get_meta("link_key", "")) == "ib:unit:unit_longbowmen", "the bubble should remember its link key")
		var codex_id := str(_store.call("entry_for_entity", "unit_longbowmen"))
		_check(str(entity.get_meta("codex_id", "")) == codex_id, "left click should lead to the linked Codex entry (%s)" % codex_id)
	var rule: PanelContainer = _bubbles.call("open", "ib:rule:army_morale", -Vector2.ONE, 0)
	await _settle()
	_check(rule != null and _bubbles.call("parent_of", rule) == entity, "ib:rule: should open a child bubble")
	if rule != null:
		var title := rule.find_child("Title", true, false) as RichTextLabel
		var detail := rule.find_child("Detail", true, false) as RichTextLabel
		_check(title != null and title.text.contains(RichTooltip.effect_label("army_morale")), "rule bubble title")
		_check(detail != null and detail.text.length() > 20, "rule bubble text from tooltips.json")
		# Fille à côté de sa parente, sans la chevaucher, dans l'écran 1280 × 720.
		_check(not rule.get_rect().intersects(entity.get_rect()), "child should not overlap its parent: %s / %s" % [rule.get_rect(), entity.get_rect()])
		_check(Rect2(0, 0, 1280, 720).encloses(rule.get_rect()), "child should stay on screen: %s" % rule.get_rect())
		_check(rule.position.x >= entity.get_rect().end.x, "child should go right of its parent when room allows")
	_check(_count() == 2 and _bubbles.call("_child_of", entity, "ib:rule:army_morale") == rule, "reopening the same link should reuse the bubble")

	# Fil d'Ariane au 3ᵉ niveau, retiré des autres bulles ; un segment ramène à son niveau.
	_bubbles.call("close_all")
	await process_frame
	var chain: Array = []
	for key in ["ib:rule:army_morale", "ib:rule:morale", "ib:rule:unrest", "ib:rule:health"]:
		var parent_index := chain.size() - 1
		var bubble: PanelContainer = _bubbles.call("open", key, Vector2(8, 8), parent_index)
		if not _check(bubble != null, "chain bubble %s" % key):
			return
		_force_size(bubble, Vector2(380, 380))
		chain.append(bubble)
		await _settle()
		if chain.size() == 2:
			_check(_bubbles.call("breadcrumb_of", chain[1]) == null, "no breadcrumb before the 3rd level")
		if chain.size() == 3:
			var crumb: RichTextLabel = _bubbles.call("breadcrumb_of", chain[2])
			_check(crumb != null and crumb.text.contains(" › ") and crumb.text.contains("[url=crumb:0]"), "breadcrumb on the 3rd level")
			_check(_bubbles.call("breadcrumb_of", chain[1]) == null, "only the most recent bubble has a breadcrumb")
	# Place insuffisante pour le 4ᵉ niveau : les ancêtres anciennes se réduisent, rien ne se ferme.
	_check(_count() == 4, "no bubble should close for lack of room (count %d)" % _count())
	_check(bool(chain[0].get_meta("collapsed", false)), "the oldest ancestor should collapse to its header")
	_check(chain[0].find_child("CollapsedHeader", true, false) != null, "collapsed bubble shows its header")
	for index in range(1, chain.size()):
		for ancestor_index in index:
			_check(not chain[index].get_rect().intersects(chain[ancestor_index].get_rect()),
				"bubble %d should not overlap ancestor %d: %s / %s" % [index, ancestor_index, chain[index].get_rect(), chain[ancestor_index].get_rect()])
		_check(Rect2(0, 0, 1280, 720).encloses(chain[index].get_rect()), "bubble %d off screen: %s" % [index, chain[index].get_rect()])
	var top_crumb: RichTextLabel = _bubbles.call("breadcrumb_of", chain[3])
	if _check(top_crumb != null, "breadcrumb on the 4th level"):
		top_crumb.meta_clicked.emit("crumb:1")
		await _settle()
		_check(_count() == 2 and not bool(chain[1].get_meta("collapsed", false)), "breadcrumb segment should close the descendants of its level (count %d)" % _count())
	_bubbles.call("set_collapsed", chain[0], false)
	_check(not bool(chain[0].get_meta("collapsed", false)) and chain[0].find_child("CollapsedHeader", true, false) == null, "a collapsed bubble reopens")

	# Placement pur : droite, sinon gauche, sinon dessous ; jamais sur une ancêtre.
	var script: Script = _bubbles.get_script()
	var area := Rect2(0, 0, 1280, 720)
	var parent_rect := Rect2(900, 100, 360, 300)
	var left: Vector2 = script.call("place_beside", Vector2(360, 200), parent_rect, 150.0, area, [parent_rect])
	_check(is_equal_approx(left.x, 900 - 6 - 360) and is_equal_approx(left.y, 150), "no room on the right → left, aligned on the keyword line: %s" % left)
	var blocked: Vector2 = script.call("place_beside", Vector2(360, 200), Rect2(460, 100, 360, 300), 120.0, area, [Rect2(460, 100, 360, 300), Rect2(826, 0, 454, 720), Rect2(0, 0, 454, 720)])
	_check(is_equal_approx(blocked.x, 460) and blocked.y >= 406, "sides taken → below the parent: %s" % blocked)
	var none: Vector2 = script.call("place_beside", Vector2(360, 700), Rect2(460, 100, 360, 300), 120.0, area, [Rect2(0, 0, 1280, 720)])
	_check(is_nan(none.x), "no room at all → NAN")

	_bubbles.call("close_all")
	_bubbles.set("area_override", Rect2())
	await process_frame


func _building_with_effects() -> String:
	var buildings := GameCatalog.definitions("buildings")
	var ids := buildings.keys()
	ids.sort()
	for id in ids:
		if ((buildings[id] as Dictionary).get("effects", []) as Array).size() >= 2:
			return str(id)
	return "bld_castle"


## Fixe la taille d'une bulle (contenu masqué par la réduction) pour un placement déterministe.
func _force_size(bubble: PanelContainer, size: Vector2) -> void:
	var spacer := Control.new()
	spacer.name = "Spacer"
	spacer.custom_minimum_size = size
	spacer.mouse_filter = Control.MOUSE_FILTER_IGNORE
	bubble.get_node("Box").add_child(spacer)
	bubble.reset_size()


func _settle() -> void:
	for _frame in 3:
		await process_frame


func _count() -> int:
	return int(_bubbles.call("bubble_count"))


func _bubble(index: int) -> PanelContainer:
	var list: Array = _bubbles.get("bubbles")
	return list[index] if index >= 0 and index < list.size() else null


func _text(bubble: PanelContainer) -> RichTextLabel:
	return bubble.find_child("Text", true, false) as RichTextLabel if bubble != null else null


func _links(bubble: PanelContainer) -> Array:
	var found: Array = []
	var label := _text(bubble)
	if label == null:
		return found
	for match in RegEx.create_from_string("\\[url=(cdx:[^\\]]+)\\]").search_all(label.text):
		if not found.has(match.get_string(1)):
			found.append(match.get_string(1))
	return found


## Premier lien dont la fiche a elle-même un lien (pour pouvoir continuer la chaîne).
func _linked_meta(bubble: PanelContainer, links: Array) -> String:
	var own := str(bubble.get_meta("codex_id", ""))
	for meta: String in links:
		var id := meta.trim_prefix("cdx:")
		if id != own and str(_bubbles.call("_entry_bbcode", id)).contains("[url=cdx:"):
			return meta
	return str(links[0]) if not links.is_empty() else ""


## Survol simulé d'un mot-lien de `bubble` pendant CHAIN_WAIT ; la bulle ouverte, sinon null.
func _hover(bubble: PanelContainer, meta: String) -> PanelContainer:
	var label := _text(bubble)
	if label == null or meta == "":
		return null
	var count := _count()
	label.meta_hover_started.emit(meta)
	await create_timer(CHAIN_WAIT).timeout
	label.meta_hover_ended.emit(meta)
	await process_frame
	if _count() == count:
		return null
	for candidate in _bubbles.get("bubbles"):
		if str(candidate.get_meta("source_meta", "")) == meta and _bubbles.call("parent_of", candidate) == bubble:
			return candidate
	return _bubble(_count() - 1)


func _key(keycode: Key, pressed: bool, shift: bool = false) -> void:
	var event := InputEventKey.new()
	event.physical_keycode = keycode
	event.keycode = keycode
	event.pressed = pressed
	event.shift_pressed = shift
	event.alt_pressed = keycode == KEY_ALT and pressed
	root.push_input(event)


func _move_mouse(at: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = at
	motion.global_position = at
	root.push_input(motion)
