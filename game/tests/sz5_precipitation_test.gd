extends SceneTree

## Test headless du lot SZ5 (suite de ZG7c, défaut S7) : `PrecipitationProfile` doit rester
## continue en distance (pas de saut ni de « bâtonnet géant » au palier site), croissante et
## raccordée à l'ancien comportement (validé) au-delà de `far_distance`.
## Usage : godot --headless --path game --script res://tests/sz5_precipitation_test.gd

var _failures := 0


func _init() -> void:
	_run()
	print("sz5_precipitation_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("sz5_precipitation_test: " + message)
	return condition


func _run() -> void:
	var profile := PrecipitationProfile.new()
	profile.meters_per_unit = 719.0
	_check_continuity(profile)
	_check_site_scale(profile)
	_check_far_matches_legacy(profile)
	_check_monotonic(profile)


## Pas de saut : deux distances très proches donnent des tailles très proches, sur toute la plage
## (site jusqu'à bien au-delà de `far_distance`).
func _check_continuity(profile: PrecipitationProfile) -> void:
	var d := 0.3
	while d < 500.0:
		var a := profile.rain_size(d).y
		var b := profile.rain_size(d * 1.001).y
		_check(absf(b - a) / maxf(a, 1e-9) < 0.02, "rain length jump at d=%.3f (%.6f -> %.6f)" % [d, a, b])
		d *= 1.3


## Palier site (ZoomTiers.site_threshold = 1.8) : la strie doit rester d'échelle « quelques
## dizaines de cm », pas des mètres à dizaines de mètres (défaut S7).
func _check_site_scale(profile: PrecipitationProfile) -> void:
	var size := profile.rain_size(1.8)
	var length_m := size.y * profile.meters_per_unit
	var width_m := size.x * profile.meters_per_unit
	_check(length_m < 5.0, "rain streak too long at site distance: %.2f m" % length_m)
	_check(width_m < 2.0, "rain streak too wide at site distance: %.2f m" % width_m)
	_check(length_m > 0.05 and width_m > 0.005, "rain streak invisible (sub-mm) at site distance: %s m" % [size * profile.meters_per_unit])
	var snow_m := profile.snow_size(1.8) * profile.meters_per_unit
	_check(snow_m < 2.0, "snowflake too big at site distance: %.2f m" % snow_m)


## Au-delà de `far_distance`, comportement strictement identique à l'ancien (constantes ZG7c/PF1) :
## largeur ~0,035 × (d / 100), longueur ~1,1 × (d / 100), boîte ~80 × (d / 100), vitesse pluie
## 60-75 × (d / 100).
func _check_far_matches_legacy(profile: PrecipitationProfile) -> void:
	for d in [40.0, 100.0, 320.0]:
		var size := profile.rain_size(d)
		_check(is_equal_approx(size.x, 0.00035 * d), "legacy rain width broken at d=%s (%s)" % [d, size.x])
		_check(is_equal_approx(size.y, 0.011 * d), "legacy rain length broken at d=%s (%s)" % [d, size.y])
		var extents := profile.box_extents(d)
		_check(is_equal_approx(extents.x, 0.8 * d), "legacy box horizontal broken at d=%s (%s)" % [d, extents.x])
		var speed := profile.speed(d, false)
		_check(is_equal_approx(speed.x, 0.6 * d) and is_equal_approx(speed.y, 0.75 * d), "legacy rain speed broken at d=%s (%s)" % [d, speed])


## Toutes les grandeurs croissent (au sens large) avec la distance : jamais de goutte qui rétrécit
## en s'éloignant, ni de rebond.
func _check_monotonic(profile: PrecipitationProfile) -> void:
	var distances := [0.3, 0.9, 1.8, 3.0, 6.0, 15.0, 30.0, 60.0, 150.0, 320.0]
	var previous := -1.0
	for d in distances:
		var length: float = profile.rain_size(d).y
		_check(length >= previous - 1e-9, "rain length not monotonic at d=%s (%.6f < %.6f)" % [d, length, previous])
		previous = length
