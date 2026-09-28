extends SceneTree

## Chantier IB (ADR 0109), lot IB2 : infobulles brutes migrées (spec § 2.4).
## - `RichTooltip.attach_plain(control, key)` pose une clé `ib:plain:<key>` qui rend en sections
##   (titre + corps), pas un simple texte natif ;
## - un contrôle natif sans script dédié reçoit `plain_tooltip_host.gd` (`_make_custom_tooltip`) ;
## - aucun `tooltip_text = "…"` littéral ne reste dans `game/scripts` hors la liste d'exceptions
##   ci-dessous (textes purement dynamiques sans titre possible, documentés dans `docs/wip/ib2.md`) ;
## - chaque clé `ib:plain:<key>` référencée depuis le GDScript existe dans `data/ui/tooltips.json`
##   (bloc `plain`) et a un titre.
## Usage : godot --headless --path game --script res://tests/ib_plain_test.gd

const ENABLED := true

## Fichiers où un `tooltip_text = "…"` littéral peut légitimement rester (§ 2.4 : textes purement
## dynamiques sans titre possible, ou migration IB2 restante — voir docs/wip/ib2.md « reste »).
## Chemins relatifs à `res://`. Se réduit lot par lot au fil de la migration.
const LITERAL_EXCEPTIONS: Array[String] = [
	"res://scripts/audio/advisor.gd",
	"res://scripts/battle/battle_alerts_column.gd",
	"res://scripts/battle/battle_formation_picker.gd",
	"res://scripts/battle/battle_hud.gd",
	"res://scripts/battle/battle_minimap.gd",
	"res://scripts/battle/battle_replay_bar.gd",
	"res://scripts/battle/battle_result_screen.gd",
	"res://scripts/battle/pre_battle_dialog.gd",
	"res://scripts/battle/roster_card.gd",
	"res://scripts/codex/codex_window.gd",
	"res://scripts/naval/naval_hud.gd",
	"res://scripts/naval/naval_pre_battle_dialog.gd",
	"res://scripts/ui/army_strip.gd",
	"res://scripts/ui/budget_table.gd",
	"res://scripts/ui/character_sheet.gd",
	"res://scripts/ui/chivalry_section.gd",
	"res://scripts/ui/chronicle_window.gd",
	"res://scripts/ui/court_panel.gd",
	"res://scripts/ui/diplomacy_panel.gd",
	"res://scripts/ui/encounter_window.gd",
	"res://scripts/ui/encyclopedia.gd",
	"res://scripts/ui/end_turn_cluster.gd",
	"res://scripts/ui/faction_panel.gd",
	"res://scripts/ui/general_seal.gd",
	"res://scripts/ui/mercenary_panel.gd",
	"res://scripts/ui/retinue_row.gd",
	"res://scripts/ui/season_report.gd",
	"res://scripts/ui/tutorial.gd",
]

var _failures := 0
var _checks := 0


func _init() -> void:
	if not ENABLED:
		print("ib_plain_test: disabled")
		quit(0)
		return
	_run.call_deferred()


func _check(condition: bool, message: String) -> bool:
	_checks += 1
	if not condition:
		_failures += 1
		push_error("ib_plain_test: " + message)
		print("FAIL: " + message)
	return condition


func _run() -> void:
	await process_frame
	_check_plain_entry()
	_check_attach_on_rich_button()
	_check_attach_on_native_control()
	_check_dynamic_override()
	_check_no_stray_literals()
	print("ib_plain_test: %d checks, %s" % [_checks, "OK" if _failures == 0 else "%d failure(s)" % _failures])
	quit(0 if _failures == 0 else 1)


## `plain_spec` lit `tooltips.json` (clé de test posée par le pytest de schéma : ib2_smoke_test).
func _check_plain_entry() -> void:
	var spec := RichTooltip.plain_spec("ib2_smoke_test", {"title": "Titre de test", "body": "Corps de test"})
	_check(spec.get("kind", "") == "plain", "plain_spec kind")
	_check(str(spec.get("title", "")) == "Titre de test", "plain_spec title from live")
	_check(not (spec.get("effects", []) as Array).is_empty(), "plain_spec body becomes a body line")


## Un `RichButton` (déjà scripté) garde son script ; `attach_plain` ne fait que poser la clé.
func _check_attach_on_rich_button() -> void:
	var button := RichButton.new()
	RichTooltip.attach_plain(button, "ib2_smoke_test", {"title": "Fermer", "hint": "Échap"})
	_check(button.tooltip_text.begins_with("ib:plain:ib2_smoke_test"), "RichButton tooltip_text carries the ib:plain: key")
	var panel := button._make_custom_tooltip(button.tooltip_text)
	_check(panel is Control, "RichButton still renders a sectioned tooltip")
	button.free()


## Un `Button` natif sans script reçoit `plain_tooltip_host.gd` et rend, lui aussi, une spec.
func _check_attach_on_native_control() -> void:
	var button := Button.new()
	_check(button.get_script() == null, "plain native control starts without a script")
	RichTooltip.attach_plain(button, "ib2_smoke_test", {"title": "Fermer"})
	_check(button.get_script() != null, "attach_plain scripts a native control")
	var panel: Object = button.call("_make_custom_tooltip", button.tooltip_text)
	_check(panel is Control, "native control renders a sectioned tooltip once hosted")
	var title := (panel as Control).find_child("Title", true, false) as RichTextLabel
	_check(title != null and title.text.contains("Fermer"), "hosted tooltip shows the title")
	button.free()


## `live["body"]` l'emporte sur le texte de `tooltips.json` (textes dynamiques, ex. « Vitesse ×%d »).
func _check_dynamic_override() -> void:
	var spec := RichTooltip.plain_spec("ib2_smoke_test", {"title": "Vitesse", "body": "Vitesse ×4 (+ / −)"})
	var text := ""
	for effect in spec.get("effects", []):
		text += str(effect.get("text", ""))
	_check(text.contains("×4"), "live body overrides the static tooltips.json text")


## Aucun `tooltip_text = "…"` littéral ne reste dans `game/scripts`, hors `LITERAL_EXCEPTIONS`.
func _check_no_stray_literals() -> void:
	var offenders := PackedStringArray()
	var regex := RegEx.new()
	regex.compile("tooltip_text\\s*\\+?=\\s*\"")
	_scan_dir("res://scripts", regex, offenders)
	var kept := PackedStringArray()
	for path in offenders:
		if not LITERAL_EXCEPTIONS.has(path):
			kept.append(path)
	_check(kept.is_empty(), "tooltip_text literals outside the exception list: %s" % ", ".join(kept))


func _scan_dir(path: String, regex: RegEx, offenders: PackedStringArray) -> void:
	var dir := DirAccess.open(path)
	if dir == null:
		return
	dir.list_dir_begin()
	var entry := dir.get_next()
	while entry != "":
		if entry.begins_with("."):
			entry = dir.get_next()
			continue
		var full := path.path_join(entry)
		if dir.current_is_dir():
			_scan_dir(full, regex, offenders)
		elif entry.ends_with(".gd") and _has_stray_literal(full, regex):
			offenders.append(full)
		entry = dir.get_next()


## Vrai si un code (non un commentaire) de `path` assigne `tooltip_text` à une chaîne littérale.
func _has_stray_literal(path: String, regex: RegEx) -> bool:
	for line in FileAccess.get_file_as_string(path).split("\n"):
		if not line.strip_edges().begins_with("#") and regex.search(line) != null:
			return true
	return false
