extends TestCase

## Lot AS8d : mouvements d'engins, de feu et de drapeaux mesurés sur vidéos libres (rendu seulement).
## Vérifie :
## - la courbe de bascule du trébuchet (`trebuchet.swing_curve.lut`) : départ 0, fin 1, valeurs
##   bornées, montée franche, et `_trebuchet_swing` la suit (arc de la verge de cocked à overswing) ;
## - sans courbe, la forme procédurale d'origine reste utilisée ;
## - les durées de l'éclair et de la fumée de bombarde (`bombard.flash_s`, `smoke_s`) ;
## - `map_fire_wind.json` : planche de flammes importable (grille `frames`), fréquence de
##   vacillement, champ `source` sur les valeurs mesurées ; planche vidéo 8x8 de 512 px au plus.
## Usage : godot --headless --path game --script res://tests/as8d_test.gd


func _init() -> void:
	var settings := SiegeEnginesFx.settings()
	var tre: Dictionary = settings.get("trebuchet", {})
	check(not tre.is_empty(), "réglages trébuchet")
	_check_curve(tre)
	_check_bombard(settings.get("bombard", {}))
	_check_fire_wind()
	finish()


func _check_curve(tre: Dictionary) -> void:
	var curve: Dictionary = tre.get("swing_curve", {})
	check(not curve.is_empty() and str(curve.get("source", "")) != "", "swing_curve avec source")
	var lut: Array = curve.get("lut", [])
	check(lut.size() >= 8, "lut assez fine")
	if lut.size() < 2:
		return
	check(is_zero_approx(float(lut[0])) and is_equal_approx(float(lut[lut.size() - 1]), 1.0), "lut de 0 à 1")
	var half := float(lut[lut.size() / 2])
	check(half > 0.3 and half < 0.9, "la verge a franchi l'essentiel de l'arc à mi-course (%.2f)" % half)
	for v in lut:
		check(float(v) >= 0.0 and float(v) <= 1.2, "valeur de lut bornée")
	var cocked := float(tre["cocked_deg"])
	var over := float(tre["overswing_deg"])
	var start: Vector2 = SiegeEnginesFx._trebuchet_swing(tre, 0.0)
	var end: Vector2 = SiegeEnginesFx._trebuchet_swing(tre, 1.0)
	check(is_equal_approx(rad_to_deg(start.x), cocked), "pose armée à u = 0")
	check(is_equal_approx(rad_to_deg(end.x), over), "pose de fin de dépassement à u = 1")
	var mid: Vector2 = SiegeEnginesFx._trebuchet_swing(tre, 0.5)
	check(rad_to_deg(mid.x) > cocked and rad_to_deg(mid.x) < over, "verge entre armée et fin à u = 0,5")
	var fallback := tre.duplicate()
	fallback.erase("swing_curve")
	check(is_equal_approx(SiegeEnginesFx._swing_progress(fallback, 0.5), 0.5 * 0.5 * (2.2 - 0.6)), "courbe procédurale sans lut")


func _check_bombard(bombard: Dictionary) -> void:
	var flash := float(bombard.get("flash_s", 0.0))
	var smoke := float(bombard.get("smoke_s", 0.0))
	check(flash > 0.05 and flash < 0.5, "durée d'éclair plausible (%.2f s)" % flash)
	check(smoke > 2.0 and smoke < 10.0, "durée de fumée plausible (%.2f s)" % smoke)
	check(str(bombard.get("source", "")) != "", "bombarde : source")
	check(BattleEffects._bombard_time("flash_s", -1.0) == flash, "BattleEffects lit flash_s")
	check(BattleEffects._bombard_time("smoke_s", -1.0) == smoke, "BattleEffects lit smoke_s")


func _check_fire_wind() -> void:
	var fire: Dictionary = MapFireWind.data().get("fire", {})
	check(not fire.is_empty(), "section fire")
	var frames: Array = fire.get("frames", [8, 8])
	for path in [str(fire.get("flipbook", "")), "res://assets/textures/fx/flame_flipbook_video.png"]:
		check(ResourceLoader.exists(path), "planche importée : %s" % path)
	var video := load("res://assets/textures/fx/flame_flipbook_video.png") as Texture2D
	if video != null:
		check(video.get_width() <= 512 and video.get_height() <= 512, "planche vidéo <= 512 px")
		check(video.get_width() == video.get_height(), "planche vidéo carrée (grille 8x8)")
	check(int(frames[0]) * int(frames[1]) >= 16, "grille de la planche")
	var hz := float(fire.get("flicker_hz", 0.0))
	check(hz > 0.3 and hz < 5.0, "vacillement mesuré (%.2f Hz)" % hz)
	check(str(fire.get("source", "")) != "", "fire : source")
	var banner: Dictionary = MapFireWind.data().get("maquette_banner", {})
	check(str(banner.get("source", "")) != "", "bannière : source")
	check(float(banner.get("wave_speed", 0.0)) > 3.0 and float(banner.get("wave_speed", 0.0)) < 7.0, "vitesse d'onde de bannière plausible")
	check(float(banner.get("wave_length", 0.0)) > 0.2 and float(banner.get("wave_length", 0.0)) < 1.5, "longueur d'onde de bannière plausible")
	var grass: Dictionary = MapFireWind.data().get("grass_measured", {})
	check(str(grass.get("source", "")) != "" and (grass.get("sway_hz", []) as Array).size() > 0, "herbe mesurée avec source")
