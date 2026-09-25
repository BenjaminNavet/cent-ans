class_name UserDirMigration
extends RefCounted

## PF1 (ADR 0031) : le jeu exporté a son propre dossier utilisateur
## (`~/Library/Application Support/Cent Ans`, réglage `application/config/use_custom_user_dir`
## propre à la fonctionnalité `template`), séparé de celui de l'éditeur et des tests
## (`~/Library/Application Support/Godot/app_userdata/Cent Ans`). Au premier lancement du jeu
## exporté, les fichiers du joueur de l'ancien dossier partagé y sont copiés une fois (jamais
## déplacés ni effacés) : sauvegardes, réglages, codex. Les fichiers des tests (`smoke`,
## `*_test*`, `settings_smoke.cfg`, dossiers `rl1_journey_*`) ne sont pas repris.
## Appelé par le premier autoload (`MapPaths`), avant toute lecture de `user://`.

const MARKER := "user://.migrated_from_shared_userdata"
const FILES := ["settings.cfg", "codex.json"]
const SAVES := "saves"


## Ancien dossier partagé (dossier par défaut de Godot pour ce nom de projet).
static func legacy_dir() -> String:
	var name := str(ProjectSettings.get_setting("application/config/name", "Cent Ans"))
	var base := OS.get_data_dir()  # ~/Library/Application Support sur macOS
	return base.path_join("Godot").path_join("app_userdata").path_join(name)


## Vrai si ce processus utilise un dossier utilisateur distinct de l'ancien dossier partagé.
static func isolated() -> bool:
	return OS.get_user_data_dir().simplify_path() != legacy_dir().simplify_path()


static func is_test_file(file: String) -> bool:
	var lower := file.to_lower()
	return lower.begins_with("smoke") or lower.contains("_test") or lower.contains("journey") or lower.ends_with("_smoke.cfg")


## Copie unique ; renvoie le nombre de fichiers copiés (0 si rien à faire).
static func migrate_legacy() -> int:
	if not isolated() or FileAccess.file_exists(MARKER):
		return 0
	var legacy := legacy_dir()
	var copied := 0
	if DirAccess.dir_exists_absolute(legacy):
		for file: String in FILES:
			copied += _copy(legacy.path_join(file), "user://".path_join(file))
		var saves := legacy.path_join(SAVES)
		if DirAccess.dir_exists_absolute(saves):
			DirAccess.make_dir_recursive_absolute("user://".path_join(SAVES))
			for file in DirAccess.get_files_at(saves):
				if not is_test_file(file):
					copied += _copy(saves.path_join(file), "user://".path_join(SAVES).path_join(file))
	var marker := FileAccess.open(MARKER, FileAccess.WRITE)
	if marker != null:
		marker.store_line("from=%s copied=%d" % [legacy, copied])
	return copied


## Copie sans écraser un fichier déjà présent dans le nouveau dossier.
static func _copy(from: String, to: String) -> int:
	if not FileAccess.file_exists(from) or FileAccess.file_exists(to):
		return 0
	return 1 if DirAccess.copy_absolute(from, to) == OK else 0
