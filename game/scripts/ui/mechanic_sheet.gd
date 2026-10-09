class_name MechanicSheet
extends RefCounted
## Fiches explicatives « Mécaniques » (`data/ui/encyclopedia.json`) : chargement unique et
## texte prêt à afficher, réutilisable par toute UI qui veut une fiche de mécanique.
## Texte d'interface seulement ; les règles vivent dans `core/`.

const DATA_PATH := "ui/encyclopedia.json"

static var _by_id: Dictionary = {}


## `id → fiche {id, name, icon, text, extra?}`, dans l'ordre du fichier.
static func by_id() -> Dictionary:
	if not _by_id.is_empty():
		return _by_id
	var path := DataFile.data_dir().path_join(DATA_PATH)
	var parsed: Variant = DataFile.parse_file(path) if FileAccess.file_exists(path) else null
	if parsed is Dictionary:
		for mechanic in parsed.get("mechanics", []):
			if mechanic is Dictionary and mechanic.has("id"):
				_by_id[str(mechanic["id"])] = mechanic
	return _by_id


## Corps de la fiche : jetons `{rule.*}` résolus puis liens du Codex.
static func body(definition: Dictionary) -> String:
	return CodexText.format(RuleValues.format(str(definition.get("text", ""))), true)
