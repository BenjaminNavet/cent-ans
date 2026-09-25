class_name NavalWaves
extends RefCounted

## Houle des batailles navales (lot NV1) : somme de quatre trains de vagues sinusoïdaux orientés
## selon le vent. La même formule est évaluée par `naval_sea.gdshader` (déplacement des sommets)
## et ici (tangage, roulis et pilonnement des navires), avec le même temps `time` : les coques
## suivent exactement la mer dessinée. Rendu seulement.

const COUNT := 4
## Longueur d'onde (m), amplitude de base (m), décalage angulaire par rapport au vent (rad).
const WAVELENGTH := [62.0, 33.0, 17.0, 8.5]
const AMPLITUDE := [0.55, 0.32, 0.15, 0.06]
const ANGLE := [0.0, 0.55, -0.7, 1.3]
const PHASE := [0.0, 1.7, 4.1, 2.6]
const GRAVITY := 9.81

## Paramètres par train : Vector4(dir.x, dir.z, nombre d'onde k, amplitude) et pulsation.
var waves: Array[Vector4] = []
var omegas: PackedFloat32Array = PackedFloat32Array()
var phases: PackedFloat32Array = PackedFloat32Array()


## `wind_to` : direction vers laquelle souffle le vent (rad, plan x-z) ; `strength` 0-1.
func configure(wind_to: float, strength: float) -> void:
	waves.clear()
	omegas.clear()
	phases.clear()
	var scale := 0.35 + 0.95 * clampf(strength, 0.0, 1.0)
	for i in COUNT:
		var angle := wind_to + float(ANGLE[i])
		var k := TAU / float(WAVELENGTH[i])
		waves.append(Vector4(cos(angle), sin(angle), k, float(AMPLITUDE[i]) * scale))
		omegas.append(sqrt(GRAVITY * k))
		phases.append(float(PHASE[i]))


func height(x: float, z: float, time: float) -> float:
	var h := 0.0
	for i in waves.size():
		var w := waves[i]
		h += w.w * sin(w.z * (w.x * x + w.y * z) - omegas[i] * time + phases[i])
	return h


## Pente (dh/dx, dh/dz) au point.
func slope(x: float, z: float, time: float) -> Vector2:
	var s := Vector2.ZERO
	for i in waves.size():
		var w := waves[i]
		var c := w.w * w.z * cos(w.z * (w.x * x + w.y * z) - omegas[i] * time + phases[i])
		s += Vector2(w.x * c, w.y * c)
	return s


## Uniformes du shader de la mer.
func apply(material: ShaderMaterial) -> void:
	material.set_shader_parameter("waves", waves)
	material.set_shader_parameter("omegas", omegas)
	material.set_shader_parameter("phases", phases)
