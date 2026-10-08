class_name FaUi
extends RefCounted

## Lot FA5 — ornements réels de l'interface : initiales enluminées, rinceaux, sceaux de cire et
## matières découpés dans des œuvres du domaine public (`game/assets/ui/fa/`, sources dans
## `SOURCE.md`). Le catalogue des découpes est `data/ui/fa_ui_assets.json` (schéma
## `data/schemas/ui_fa_assets.schema.json`), lu ici pour savoir quelle initiale existe.
## Rendu seulement. `enabled = false` (option `--no-fa` des scripts de capture) rend l'ancien
## habillage pour les comparaisons avant/après.

const DATA_PATH := "ui/fa_ui_assets.json"
const ASSET_DIR := "res://assets/ui/fa/"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

## Écart entre un titre et le rinceau qui le suit.
const SPRAY_GAP := 12.0

static var enabled := true
static var _data: Dictionary = {}
static var _loaded := false
static var _textures: Dictionary = {}


static func data() -> Dictionary:
	if not _loaded:
		_loaded = true
		# Toujours le catalogue du dépôt : les jeux de données réduits des tests n'en ont pas.
		var path := MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = DataFile.parse_file(path)
			if parsed is Dictionary:
				_data = parsed
		if _data.is_empty():
			push_warning("FaUi: %s missing or invalid" % path)
	return _data


## Texture `game/assets/ui/fa/<kind>/<id>.png`, ou null (absente, ou FA désactivé).
static func texture(kind: String, id: String) -> Texture2D:
	if not enabled or id.is_empty():
		return null
	var path := ASSET_DIR + kind + "/" + id + ".png"
	if not _textures.has(path):
		_textures[path] = load(path) as Texture2D if ResourceLoader.exists(path) else null
	return _textures[path]


## Réglage de pose (`display` du catalogue), `fallback` si absent.
static func display(key: String, fallback: float) -> float:
	return float((data().get("display", {}) as Dictionary).get(key, fallback))


## Initiale enluminée réelle pour la première lettre de `title` (sans accent) :
## `{"texture": Texture2D, "framed": bool}` (`framed` : champ peint opaque), ou `{}` — le titre
## garde alors la lettrine dessinée. Entre plusieurs initiales de la même lettre, le choix est
## stable pour un titre donné.
static func initial_for(title: String) -> Dictionary:
	if not enabled or title.is_empty():
		return {}
	var letter := title.substr(0, 1).to_upper()
	letter = str(Lettrine.UNACCENTED.get(letter, letter))
	var pool: Array[Dictionary] = []
	for entry: Dictionary in data().get("initials", []):
		if str(entry.get("letter", "")) == letter:
			pool.append(entry)
	if pool.is_empty():
		return {}
	var chosen := pool[absi(hash(title)) % pool.size()]
	var image := texture("initials", str(chosen.get("id", "")))
	if image == null:
		return {}
	return {"texture": image, "framed": str(chosen.get("mode", "")) == "framed"}


## Ornement détouré (`ornaments/<id>.png`), ou null.
static func ornament(id: String) -> Texture2D:
	return texture("ornaments", id)




## Dessine le rinceau sur `canvas` à partir de l'abscisse `text_end`, centré sur `centre_y`,
## réduit à la largeur libre ; rien sous `display.title_spray_min_width_px`.
static func draw_spray(canvas: Control, text_end: float, centre_y: float, height: float) -> void:
	var spray := ornament(str((data().get("display", {}) as Dictionary).get("title_spray", "")))
	if spray == null:
		return
	var room := canvas.size.x - text_end - SPRAY_GAP * 2.0
	if room < display("title_spray_min_width_px", 96.0):
		return
	var texture_size := spray.get_size()
	var width := minf(height * texture_size.x / texture_size.y, room)
	var drawn_height := width * texture_size.y / texture_size.x
	var rect := Rect2(Vector2(text_end + SPRAY_GAP, centre_y - drawn_height * 0.5), Vector2(width, drawn_height))
	canvas.draw_texture_rect(spray, rect, false, Color(1, 1, 1, display("title_spray_opacity", 0.85)))


## Sceau de cire réel d'un moment solennel : `role` est une clé de `display.seals`
## (`treaty`, `chronicle`…) ; null si le catalogue n'en désigne pas ou si FA est désactivé.
static func seal(role: String) -> Texture2D:
	var roles: Dictionary = (data().get("display", {}) as Dictionary).get("seals", {})
	return texture("seals", str(roles.get(role, "")))


## Sceau posé dans une mise en page : image haute de `height` px, à ses proportions, sans
## capter la souris ; null si le sceau manque.
static func seal_rect(role: String, height: float) -> TextureRect:
	var wax := seal(role)
	if wax == null:
		return null
	var rect := TextureRect.new()
	rect.name = "FaSeal"
	rect.texture = wax
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
	rect.custom_minimum_size = Vector2(roundf(height * wax.get_width() / wax.get_height()), height)
	rect.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return rect


## Plaque de matière bordée de métal (`materials/<id>.png`, 9 tranches : bord = `rim.width_px`
## du catalogue, centre répété), ou null si elle manque ou si FA est désactivé.
static func plate_box(id: String) -> StyleBoxTexture:
	var plate := texture("materials", id)
	if plate == null:
		return null
	var rim := 1.0
	for entry: Dictionary in data().get("materials", []):
		if str(entry.get("id", "")) == id:
			rim = float((entry.get("rim", {}) as Dictionary).get("width_px", 1))
	var box := StyleBoxTexture.new()
	box.texture = plate
	box.set_texture_margin_all(rim)
	box.axis_stretch_horizontal = StyleBoxTexture.AXIS_STRETCH_MODE_TILE
	box.axis_stretch_vertical = StyleBoxTexture.AXIS_STRETCH_MODE_TILE
	return box
