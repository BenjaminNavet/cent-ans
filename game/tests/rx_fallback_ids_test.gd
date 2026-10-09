extends TestCase

## RX uifin : détecte les replis d'identifiant. Quand un nom manque, l'interface affiche l'id
## « lisible » (`unit_milice_urbaine` → « Milice urbaine », sans accents) ou l'id brut : aucun
## test ne le voyait. Ici :
##  1. toute définition des dossiers affichés de `data/` porte un `name.display` non vide, donc
##     `GameCatalog.display_name` ne passe jamais par son repli ;
##  2. aucun texte affiché de `data/ui/*.json` n'est un identifiant brut (`snake_case`).
## Usage : godot --headless --path game --script res://tests/rx_fallback_ids_test.gd

const DISPLAY_FOLDERS := ["unit_types", "buildings", "technologies", "traits", "skills", "resources"]
const SHOWN_KEYS := ["text", "title", "body", "label", "tagline", "intro", "objective", "strengths", "weaknesses"]
## Un identifiant brut : minuscules, chiffres et « _ », au moins un « _ », pas d'espace.
var _raw_id := RegEx.new()


func _init() -> void:
	_raw_id.compile("^[a-z][a-z0-9]*(_[a-z0-9]+)+$")
	_test_display_names()
	_test_ui_texts()
	finish()


func _test_display_names() -> void:
	for folder in DISPLAY_FOLDERS:
		var definitions := GameCatalog.definitions(folder)
		check(definitions.size() > 0, "%s: definitions loaded (%d)" % [folder, definitions.size()])
		var missing: Array[String] = []
		for id: String in definitions:
			var name: Variant = (definitions[id] as Dictionary).get("name", null)
			if not (name is Dictionary) or str(name.get("display", "")) == "":
				missing.append(id)
		check(missing.is_empty(), "%s: every definition has name.display (missing: %s)" % [folder, ", ".join(missing)])


func _test_ui_texts() -> void:
	var dir := DirAccess.open(DataFile.data_dir().path_join("ui"))
	if not check(dir != null, "data/ui opens"):
		return
	for file_name in dir.get_files():
		if not file_name.ends_with(".json") or file_name == "tooltips.json":
			continue
		var parsed: Variant = DataFile.parse_file(DataFile.data_dir().path_join("ui").path_join(file_name))
		var raw: Array[String] = []
		_collect_raw(parsed, "", raw)
		check(raw.is_empty(), "ui/%s: no raw id shown (%s)" % [file_name, ", ".join(raw)])


func _collect_raw(node: Variant, key: String, out: Array[String]) -> void:
	if node is Dictionary:
		for child_key: String in node:
			_collect_raw(node[child_key], child_key, out)
	elif node is Array:
		for child: Variant in node:
			_collect_raw(child, key, out)
	elif node is String and key in SHOWN_KEYS and _raw_id.search(node) != null:
		out.append(node)
