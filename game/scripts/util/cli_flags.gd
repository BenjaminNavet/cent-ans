class_name CliFlags
extends RefCounted

## Point d'accès unique aux arguments utilisateur (`godot ... -- --drapeau --option=valeur`) :
## les scripts de bataille demandent ici un drapeau ou une option au lieu de relire la ligne de
## commande. Lecture seule, cache à la première demande.

static var _args: PackedStringArray = PackedStringArray()
static var _loaded := false


static func args() -> PackedStringArray:
	if not _loaded:
		_loaded = true
		_args = OS.get_cmdline_user_args()
	return _args


## Vrai si `--drapeau` figure dans la ligne de commande.
static func has(flag: String) -> bool:
	return args().has(flag)


## Valeur de `--option=valeur` (la dernière occurrence l'emporte), `fallback` si absente.
static func value(option: String, fallback: String = "") -> String:
	var prefix := option + "="
	var found := fallback
	for arg in args():
		if arg.begins_with(prefix):
			found = arg.trim_prefix(prefix)
	return found
