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
