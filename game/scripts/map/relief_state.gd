class_name ReliefState
extends RefCounted

## État du relief exagéré, partagé par tout le jeu (lot SC MC11) : échelle verticale (ADR 0036,
## lot ZG4), gain de relief local et fond de vallée (lot ZG8, SZ1). Statique : un seul état par
## processus. `MapData` en garde l'API publique par délégation (`MapData.display_height`…).

## Facteur vertical courant (unités monde par mètre), propriétaire unique de l'échelle verticale
## (ADR 0036, lot ZG4) : `MapData.HEIGHT_SCALE` (×4,3) en vue stratégique, ramené vers ×1,5 au zoom le
## plus rapproché quand la pyramide de relief est en cache (`CloseCameraProfile`). Lu par les
## shaders (paramètre global `campaign_vertical_scale`), le quadtree et `surface_height_at`.
## Les maillages E0 du repli sans cache restent cuits à `MapData.HEIGHT_SCALE` (l'échelle ne bouge pas).
static var _vertical_scale: float = MapData.HEIGHT_SCALE
## Lot ZG8 : relief exagéré (hauteur affichée = s·(h + g·max(h − fond, 0)), voir
## `ReliefExaggerationProfile`). Gain de relief local courant (fonction de l'échelle, publié avec
## elle), fond de vallée (m, grille `ReliefFloor`, lecture seule une fois publiée : lisible depuis
## les fils de travail). Sans fond publié, le gain est nul : comportement du lot ZG4.
static var _relief_gain: float = 0.0
static var _floor: PackedFloat32Array = PackedFloat32Array()
## Lot SZ1 : base (fond non plafonné, m) et facteur d'écrasement des montagnes k (0..1) par cellule
## de la même grille ; poids courant c de l'écrasement (fonction de l'échelle, publié avec elle).
static var _floor_base: PackedFloat32Array = PackedFloat32Array()
static var _floor_squash: PackedFloat32Array = PackedFloat32Array()
static var _floor_squash_max: float = 0.0
static var _relief_squash: float = 0.0
static var _floor_side: Vector2i = Vector2i.ZERO
static var _floor_cell: float = 8.0
## Lot PB2 : incrémenté à chaque publication du fond (le semis natif le recopie alors).
static var _floor_version: int = 0


static func vertical_scale() -> float:
	return _vertical_scale


## Lot ZG4 : change l'échelle verticale (propriétaire unique) et la publie aux shaders par le
## paramètre global `campaign_vertical_scale` (terrain, quadtree, fleuves, maquettes). Rend vrai
## si la valeur a changé. Appelé par `TerrainBuilder.set_vertical_scale`, qui recale les calques ;
## ne pas l'appeler directement ailleurs (sinon les objets posés ne suivent pas).
static func set_vertical_scale(value: float) -> bool:
	value = maxf(value, 1e-6)
	if is_equal_approx(value, _vertical_scale):
		return false
	_vertical_scale = value
	RenderingServer.global_shader_parameter_set("campaign_vertical_scale", value)
	_publish_gain()
	return true


# --- Relief exagéré (lot ZG8) ------------------------------------------------------------


## Plancher de l'échelle verticale de près (unités monde par mètre), selon les profils.
static func near_vertical_scale() -> float:
	var camera := CloseCameraProfile.load_default()
	return MapData.HEIGHT_SCALE * camera.near_exaggeration() / camera.exaggeration_far


## Gain de relief local pour une échelle donnée (nul sans fond publié).
static func relief_gain_for_scale(scale: float) -> float:
	if _floor.is_empty():
		return 0.0
	return ReliefExaggerationProfile.load_default().gain_for_scale(scale, near_vertical_scale())


## Gain de relief local courant.
static func relief_gain() -> float:
	return _relief_gain


## Plus grand gain possible (boîtes englobantes conservatrices : y ≤ s·(1 + g_max)·h).
static func relief_max_gain() -> float:
	return 0.0 if _floor.is_empty() else ReliefExaggerationProfile.load_default().max_gain()


static func _publish_gain() -> void:
	var gain := relief_gain_for_scale(_vertical_scale)
	_relief_gain = gain
	RenderingServer.global_shader_parameter_set("campaign_relief_gain", gain)
	var squash := relief_squash_for_scale(_vertical_scale)
	_relief_squash = squash
	RenderingServer.global_shader_parameter_set("campaign_relief_squash", squash)


## Lot SZ1 : échelle (unités monde par mètre) à partir de laquelle l'écrasement des montagnes est
## entier (`ReliefExaggerationProfile.mountain_squash_full_exaggeration`).
static func squash_full_vertical_scale() -> float:
	var camera := CloseCameraProfile.load_default()
	var relief := ReliefExaggerationProfile.load_default()
	return MapData.HEIGHT_SCALE * relief.mountain_squash_full_exaggeration / camera.exaggeration_far


## Lot SZ1 : poids c de l'écrasement des montagnes pour une échelle (nul sans fond publié).
static func relief_squash_for_scale(scale: float) -> float:
	if _floor.is_empty() or _floor_squash_max <= 0.0:
		return 0.0
	return ReliefExaggerationProfile.load_default().squash_weight_for_scale(scale, squash_full_vertical_scale())


## Lot SZ1 : poids courant de l'écrasement des montagnes.
static func relief_squash() -> float:
	return _relief_squash


## Lot SZ1 : plus grand écrasement c·k pour une échelle (boîtes englobantes : y ≥ s·(1 − c·k)·h).
static func relief_squash_max_for_scale(scale: float) -> float:
	return relief_squash_for_scale(scale) * _floor_squash_max


## Publie le fond de vallée (`ReliefFloor.compute`) aux shaders et au double GDScript ; grille
## vide : relief exagéré désactivé (gain nul).
static func set_relief_floor(grid: Dictionary) -> void:
	var data: PackedFloat32Array = grid.get("data", PackedFloat32Array())
	var side: Vector2i = grid.get("side", Vector2i.ZERO)
	if data.is_empty() or side.x * side.y != data.size():
		_floor = PackedFloat32Array()
		_floor_base = PackedFloat32Array()
		_floor_squash = PackedFloat32Array()
		_floor_squash_max = 0.0
		_floor_side = Vector2i.ZERO
	else:
		_floor = data
		var base: PackedFloat32Array = grid.get("base", PackedFloat32Array())
		var squash: PackedFloat32Array = grid.get("squash", PackedFloat32Array())
		_floor_base = base if base.size() == data.size() else data
		if squash.size() != data.size():
			squash = PackedFloat32Array()
			squash.resize(data.size())
		_floor_squash = squash
		_floor_squash_max = 0.0
		for v in squash:
			_floor_squash_max = maxf(_floor_squash_max, v)
		_floor_side = side
		_floor_cell = float(grid.get("cell", 8.0))
		RenderingServer.global_shader_parameter_set("campaign_relief_floor", ReliefFloor.texture_of(grid))
		RenderingServer.global_shader_parameter_set("campaign_relief_floor_info", Vector4(_floor_cell, 0.5 * (_floor_cell - 1.0), 0.0, 0.0))
	_floor_version += 1
	_publish_gain()


## Lot PB2 : fond de vallée publié {data, side, cell, version} (lecture seule ; semis natif).
static func relief_floor_grid() -> Dictionary:
	return {"data": _floor, "base": _floor_base, "squash": _floor_squash, "side": _floor_side, "cell": _floor_cell, "version": _floor_version}


## Vrai si un fond de vallée est publié.
static func has_relief_floor() -> bool:
	return not _floor.is_empty()


## Fond de vallée (m) au point carte (x, z) : bilinéaire entre centres de cellules, bords
## répliqués. Même calcul que `campaign_relief_floor_m` (texelFetch, pas de filtrage matériel).
static func relief_floor_at(x: float, z: float) -> float:
	return relief_fields_at(x, z).x


## Lot SZ1 : champs du relief au point carte (x, z) : (fond, base, facteur d'écrasement k), même
## interpolation que `campaign_relief_fields` (campaign_relief.gdshaderinc). Sans fond : zéro.
static func relief_fields_at(x: float, z: float) -> Vector3:
	if _floor.is_empty():
		return Vector3.ZERO
	var fx := clampf((x - 0.5 * (_floor_cell - 1.0)) / _floor_cell, 0.0, _floor_side.x - 1.0)
	var fz := clampf((z - 0.5 * (_floor_cell - 1.0)) / _floor_cell, 0.0, _floor_side.y - 1.0)
	var i := mini(int(fx), _floor_side.x - 2)
	var j := mini(int(fz), _floor_side.y - 2)
	var tx := fx - i
	var tz := fz - j
	var o := j * _floor_side.x + i
	var w := _floor_side.x
	return Vector3(
		_bilerp(_floor, o, w, tx, tz), _bilerp(_floor_base, o, w, tx, tz), _bilerp(_floor_squash, o, w, tx, tz))


static func _bilerp(grid: PackedFloat32Array, o: int, w: int, tx: float, tz: float) -> float:
	return lerpf(lerpf(grid[o], grid[o + 1], tx), lerpf(grid[o + w], grid[o + w + 1], tx), tz)


## Hauteur affichée (unités monde) d'une altitude `h_m` (m) au point carte (x, z), à l'échelle, au
## gain et à l'écrasement courants. Double GDScript de `campaign_display_height`
## (campaign_relief.gdshaderinc) : tout ce qui pose un objet au sol passe par ici.
##     y = s·(h − K·max(h − base, 0) + g·(1 − K)·max(h − fond, 0)), K = c·k
static func display_height(h_m: float, x: float, z: float) -> float:
	return display_height_with(h_m, x, z, _vertical_scale, _relief_gain, _relief_squash)


## `display_height` à une échelle, un gain et un écrasement donnés (maillages cuits, fils de travail).
static func display_height_with(h_m: float, x: float, z: float, scale: float, gain: float, squash: float) -> float:
	if gain == 0.0 and squash == 0.0:
		return h_m * scale
	return display_height_fields(h_m, relief_fields_at(x, z), scale, gain, squash)


## `display_height_with` avec les champs (fond, base, k) déjà lus au même point.
static func display_height_fields(h_m: float, fields: Vector3, scale: float, gain: float, squash: float) -> float:
	var k := squash * fields.z
	return scale * (h_m - k * maxf(h_m - fields.y, 0.0) + gain * (1.0 - k) * maxf(h_m - fields.x, 0.0))


## Inverse de `display_height` : altitude (m) dont la hauteur affichée en (x, z) vaut `y`.
static func height_from_display(y: float, x: float, z: float) -> float:
	return height_from_display_with(y, x, z, _vertical_scale, _relief_gain, _relief_squash)


## Inverse par morceaux (fonction croissante de h, coudes en base ≤ fond).
static func height_from_display_with(y: float, x: float, z: float, scale: float, gain: float, squash: float) -> float:
	var v := y / maxf(scale, 1e-9)
	if gain == 0.0 and squash == 0.0:
		return v
	var fields := relief_fields_at(x, z)
	var f := fields.x
	var b := minf(fields.y, f)
	var k := squash * fields.z
	if v <= b:
		return v
	var v_f := f - k * (f - b)
	if v <= v_f:
		return (v - k * b) / (1.0 - k)
	var g := gain * (1.0 - k)
	return (v - k * b + g * f) / (1.0 - k + g)
