class_name ReliefCacheStatus
extends RefCounted

## État du cache de relief fin (lot ZG7b, ADR 0036) : pyramide E1-E7 (`pyramid/E{k}/…png`) et
## tuiles fines des fleuves et des routes (`pyramid/hydro_fine`, `pyramid/roads_fine`), hors git.
## Sans ce cache, la carte retombe sur le relief E0 et la caméra s'arrête vers 7 unités : le
## joueur doit le savoir (avis `ReliefCacheNotice`) et avoir la commande qui le régénère.
##
## Contrôle bon marché au chargement de la campagne : le manifeste `relief_pyramid.json` est lu,
## ses listes RLE développées, puis au plus `SAMPLES_PER_LEVEL` tuiles par étage (réparties
## régulièrement, première et dernière comprises) sont cherchées sur le disque ; même contrôle sur
## les index `rivers_fine.json` et `fine_anchors.json` (`roads`). Rendu seulement.

enum State {
	## Aucun manifeste, ou manifeste sans tuiles : jeu de données sans pyramide (fixtures, essais).
	DISABLED,
	## Cache complet (pour les tuiles échantillonnées).
	COMPLETE,
	## Une partie des étages ou des tuiles fines manque : zoom rapproché dégradé par endroits.
	PARTIAL,
	## Aucune tuile de la pyramide : pas de zoom rapproché du tout.
	MISSING,
}

## Commande unique qui régénère tout le cache, dans l'ordre et avec reprise (docs/geo.md).
const REGEN_COMMAND := "uv run --project tools cent-ans geo relief-all"
const CHECK_COMMAND := "uv run --project tools cent-ans geo relief-all --check"
const SAMPLES_PER_LEVEL := 24
const MAX_LEVEL := 7
const RIVERS_MANIFEST := "rivers_fine.json"
const ANCHORS_MANIFEST := "fine_anchors.json"

var state: State = State.DISABLED
var manifest_path: String = ""
## Dossier des tuiles E1+ contrôlé.
var tiles_dir: String = ""
var tiles_dir_exists: bool = false
## Étage → nombre de tuiles listées par le manifeste.
var expected: Dictionary = {}
## Étage → nombre de tuiles échantillonnées / absentes parmi elles.
var sampled: Dictionary = {}
var missing: Dictionary = {}
## Couches fines : "rivers" / "roads" → { "expected", "sampled", "missing", "dir" }.
var fine: Dictionary = {}


## Contrôle le cache de `map_dir` ; `relief_root` remplace `map_dir` pour les tuiles hors git
## (dossier contenant `pyramid/`, voir `MapPaths.relief_root_for`).
static func check(map_dir: String, relief_root: String = "") -> ReliefCacheStatus:
	var status := ReliefCacheStatus.new()
	status._run(map_dir, relief_root if relief_root != "" else map_dir)
	return status


func is_complete() -> bool:
	return state == State.COMPLETE


## Vrai si l'avis doit être montré (cache absent ou partiel).
func needs_notice() -> bool:
	return state == State.PARTIAL or state == State.MISSING


## Étages listés dont au moins une tuile échantillonnée manque.
func incomplete_levels() -> Array[int]:
	var out: Array[int] = []
	for level: int in expected:
		if int(missing.get(level, 0)) > 0:
			out.append(level)
	out.sort()
	return out


## Couches fines ("rivers", "roads") listées mais absentes, en tout ou partie.
func incomplete_fine_layers() -> Array[String]:
	var out: Array[String] = []
	for layer: String in ["rivers", "roads"]:
		if fine.has(layer) and int((fine[layer] as Dictionary).get("missing", 0)) > 0:
			out.append(layer)
	return out


## Résumé d'une ligne pour le journal (anglais, comme les autres `print` du moteur).
func summary() -> String:
	var name: String = State.keys()[state]
	if state == State.DISABLED:
		return "ReliefCache: %s (no pyramid manifest with tiles in %s)" % [name, manifest_path]
	var parts: PackedStringArray = []
	for level: int in _sorted_keys(expected):
		parts.append("E%d %d/%d missing of %d" % [level, int(missing.get(level, 0)), int(sampled.get(level, 0)), int(expected[level])])
	for layer: String in ["rivers", "roads"]:
		if fine.has(layer):
			var f: Dictionary = fine[layer]
			parts.append("%s %d/%d missing" % [layer, int(f["missing"]), int(f["sampled"])])
	return "ReliefCache: %s in %s (sampled: %s)" % [name, tiles_dir, ", ".join(parts)]


## Titre de l'avis au joueur.
func notice_title() -> String:
	return "Relief rapproché limité" if state == State.MISSING else "Relief rapproché incomplet"


## Texte de l'avis au joueur (français). `exported` : jeu exporté (pas de dépôt ni d'outils).
func notice_text(exported: bool = false) -> String:
	var what := ""
	if state == State.MISSING:
		what = "Le cache du relief fin (pyramide E1-E7, environ 3 Go) est introuvable : la carte garde le relief d'ensemble et le zoom s'arrête vers 7 unités d'altitude, sans vallées ni rues au plus près."
	else:
		var gaps: PackedStringArray = []
		var levels := incomplete_levels()
		if not levels.is_empty():
			gaps.append("étages %s" % ", ".join(levels.map(func(l: int) -> String: return "E%d" % l)))
		for layer in incomplete_fine_layers():
			gaps.append("fleuves fins" if layer == "rivers" else "routes fines")
		what = "Le cache du relief fin est incomplet (%s) : par endroits, le zoom rapproché s'arrête plus haut ou montre un relief grossier." % " ; ".join(gaps)
	if exported:
		return what + "\nRéinstallez le jeu complet, ou placez le dossier « Cent Ans relief » à côté de l'application."
	return what + "\nPour le régénérer (données ouvertes, reprise possible, plusieurs heures) :"


# --- Contrôle ---------------------------------------------------------------------------------


func _run(map_dir: String, relief_root: String) -> void:
	manifest_path = map_dir.path_join("relief_pyramid.json")
	var manifest: Variant = _read_json(manifest_path)
	if not (manifest is Dictionary):
		state = State.DISABLED
		return
	tiles_dir = relief_root.path_join(str((manifest as Dictionary).get("dir", "pyramid")))
	tiles_dir_exists = DirAccess.dir_exists_absolute(tiles_dir)
	var pattern := str((manifest as Dictionary).get("pattern", "E{level}/{col}_{row}.png"))
	var listed_total := 0
	var missing_total := 0
	var sampled_total := 0
	for entry: Variant in (manifest as Dictionary).get("levels", []):
		if not (entry is Dictionary):
			continue
		var level := int(entry.get("level", 0))
		if level < 1 or level > MAX_LEVEL:
			continue
		var keys := _expand_rle(entry.get("tiles_rle", []), 16 << level)
		if keys.is_empty():
			continue
		expected[level] = keys.size()
		listed_total += keys.size()
		var picks := _sample(keys.size())
		var absent := 0
		for i in picks:
			var key := keys[i]
			var path := tiles_dir.path_join(pattern.replace("{level}", str(level)).replace("{col}", str(key & 0xffff)).replace("{row}", str(key >> 16)))
			if not tiles_dir_exists or not FileAccess.file_exists(path):
				absent += 1
		sampled[level] = picks.size()
		missing[level] = absent
		sampled_total += picks.size()
		missing_total += absent
	if listed_total == 0:
		state = State.DISABLED
		return
	_check_fine(map_dir, relief_root)
	var fine_missing := 0
	for layer: String in fine:
		fine_missing += int((fine[layer] as Dictionary)["missing"])
	if missing_total == sampled_total:
		state = State.MISSING
	elif missing_total > 0 or fine_missing > 0:
		state = State.PARTIAL
	else:
		state = State.COMPLETE


func _check_fine(map_dir: String, relief_root: String) -> void:
	var rivers: Variant = _read_json(map_dir.path_join(RIVERS_MANIFEST))
	if rivers is Dictionary:
		_check_fine_layer("rivers", rivers, relief_root)
	var anchors: Variant = _read_json(map_dir.path_join(ANCHORS_MANIFEST))
	if anchors is Dictionary and (anchors as Dictionary).get("roads") is Dictionary:
		_check_fine_layer("roads", (anchors as Dictionary)["roads"], relief_root)


func _check_fine_layer(layer: String, index: Dictionary, relief_root: String) -> void:
	var tiles: Array = index.get("tiles", [])
	if tiles.is_empty():
		return
	var dir := relief_root.path_join(str(index.get("dir", "")))
	var pattern := str(index.get("pattern", "E2/{col}_{row}.bin"))
	var picks := _sample(tiles.size())
	var absent := 0
	for i in picks:
		var tile: Dictionary = tiles[i]
		var path := dir.path_join(pattern.replace("{col}", str(int(tile.get("col", 0)))).replace("{row}", str(int(tile.get("row", 0)))))
		if not FileAccess.file_exists(path):
			absent += 1
	fine[layer] = {"expected": tiles.size(), "sampled": picks.size(), "missing": absent, "dir": dir}


## Clés `row << 16 | col` des tuiles d'une liste RLE (ordre du manifeste).
static func _expand_rle(rows: Array, cols: int) -> PackedInt32Array:
	var keys := PackedInt32Array()
	for row_entry: Variant in rows:
		if not (row_entry is Dictionary):
			continue
		var row := int(row_entry.get("row", -1))
		if row < 0 or row >= cols:
			continue
		for run: Variant in row_entry.get("runs", []):
			var start := int(run[0])
			for col in range(maxi(start, 0), mini(start + int(run[1]), cols)):
				keys.append((row << 16) | col)
	return keys


## Indices échantillonnés parmi `count` : tous si peu nombreux, sinon réguliers (bornes comprises).
static func _sample(count: int) -> PackedInt32Array:
	var picks := PackedInt32Array()
	if count <= SAMPLES_PER_LEVEL:
		for i in count:
			picks.append(i)
		return picks
	for s in SAMPLES_PER_LEVEL:
		picks.append(int(round(float(s) * float(count - 1) / float(SAMPLES_PER_LEVEL - 1))))
	return picks


static func _read_json(path: String) -> Variant:
	if not FileAccess.file_exists(path):
		return null
	return JSON.parse_string(FileAccess.get_file_as_string(path))


static func _sorted_keys(dict: Dictionary) -> Array:
	var keys := dict.keys()
	keys.sort()
	return keys
