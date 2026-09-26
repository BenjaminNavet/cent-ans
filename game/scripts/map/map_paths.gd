extends Node

## Autoload `MapPaths` : résolution des chemins vers `data/`.
##
## `res://../data` ne survit pas à un export ; on calcule donc un chemin absolu
## au démarrage, surchargeable par la variable d'environnement `CENT_ANS_DATA_DIR`
## (utilisée par le smoke test pour pointer sur `game/tests/fixtures`).

const ENV_VAR := "CENT_ANS_DATA_DIR"
## Lot ZG7b (ADR 0036) : dossier contenant `pyramid/` (cache du relief fin, ~3 Go, non versionné
## et hors `.pck`) quand il n'est pas dans `data/map/`.
const RELIEF_ENV_VAR := "CENT_ANS_RELIEF_DIR"
## Nom du dossier de relief livré à part, posé à côté de l'application (`Cent Ans.app`).
const RELIEF_SIBLING_DIR := "Cent Ans relief"

var data_dir: String


func _init() -> void:
	data_dir = default_data_dir()
	# PF1 (ADR 0031) : premier autoload, donc avant toute lecture de `user://` (réglages lus par
	# `AudioDirector`) : reprise unique des fichiers du joueur dans le dossier du jeu exporté.
	var copied := UserDirMigration.migrate_legacy()
	if copied > 0:
		print("Cent Ans : %d fichiers repris de %s" % [copied, UserDirMigration.legacy_dir()])


static func default_data_dir() -> String:
	var from_env := OS.get_environment(ENV_VAR)
	if from_env != "":
		return from_env.simplify_path()
	# Jeu exporté : `data/` est copié dans `Cent Ans.app/Contents/Resources/data` (macOS) ou à
	# côté de `Cent Ans.exe` (Windows, ADR 0087).
	if OS.has_feature("template"):
		var exe_dir := OS.get_executable_path().get_base_dir()
		for bundled: String in [exe_dir.path_join("../Resources/data").simplify_path(), exe_dir.path_join("data")]:
			if DirAccess.dir_exists_absolute(bundled):
				return bundled
	return project_root().path_join("data")


## Racine du dépôt (le dossier parent de `game/`).
static func project_root() -> String:
	return ProjectSettings.globalize_path("res://").path_join("..").simplify_path()


func map_dir() -> String:
	return data_dir.path_join("map")


## Dossier contenant `pyramid/` (tuiles E1-E7, `hydro_fine/`, `roads_fine/`) pour `map_dir`.
## Ordre : variable `CENT_ANS_RELIEF_DIR` (même si le dossier manque : l'avis de cache le dira),
## `map_dir` s'il contient `pyramid/` (dépôt, ou jeu exporté avec le relief dans l'application),
## dossier « Cent Ans relief » à côté de l'application ou de l'exécutable (livraison séparée),
## `user://relief`. À défaut, `map_dir` (cache absent).
static func relief_root_for(map_dir_path: String) -> String:
	var from_env := OS.get_environment(RELIEF_ENV_VAR)
	if from_env != "":
		return from_env.simplify_path()
	if DirAccess.dir_exists_absolute(map_dir_path.path_join("pyramid")):
		return map_dir_path
	for candidate in relief_candidates():
		if DirAccess.dir_exists_absolute(candidate.path_join("pyramid")):
			return candidate
	return map_dir_path


## Emplacements du relief livré à part, par ordre de préférence.
static func relief_candidates() -> PackedStringArray:
	var out := PackedStringArray()
	if OS.has_feature("template"):
		var exe_dir := OS.get_executable_path().get_base_dir()
		# macOS : `<dossier>/Cent Ans.app/Contents/MacOS/Cent Ans` → `<dossier>/Cent Ans relief`.
		out.append(exe_dir.path_join("../../..").path_join(RELIEF_SIBLING_DIR).simplify_path())
		# Windows : `<dossier>/Cent Ans.exe` → `<dossier>/Cent Ans relief`.
		out.append(exe_dir.path_join(RELIEF_SIBLING_DIR))
	out.append(ProjectSettings.globalize_path("user://relief"))
	return out


func relief_root() -> String:
	return relief_root_for(map_dir())
