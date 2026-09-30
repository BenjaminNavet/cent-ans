class_name SeasonBanner
extends PanelContainer

## Chantier PO5 (ADR 0097) : transition de fin de tour — cartouche enluminé de la nouvelle
## saison (« Printemps 1338 ») qui entre et sort en `TOTAL_SECONDS` (1,2 s) au centre de l'écran,
## un peu au-dessus du milieu, sans intercepter la souris. Posé par `TurnLight` quand la date
## change. « Réduire les animations » : pas de glissement, fondus plus courts. Rien en headless
## (le texte est posé, `last_text` retenu pour les tests, le cartouche reste caché).

const TOTAL_SECONDS := 1.2
const FADE_SECONDS := 0.3
const SLIDE_PX := 12.0
## Position verticale du centre du cartouche (part de la hauteur de l'écran).
const CENTER_Y := 0.32

## Dernière saison annoncée (tests).
var last_text: String = ""
var _label: Label
var _tween: Tween


func _init() -> void:
	name = "SeasonBanner"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	z_index = 100  # VN : le cartouche passe au-dessus des panneaux (panneau de faction, latéral)
	add_theme_stylebox_override("panel", HudStyle.illuminated_box(14))
	_label = Label.new()
	_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	UiType.apply(_label, UiType.TITLE)
	_label.add_theme_color_override("font_color", HudStyle.RUBRIC)
	add_child(_label)
	modulate.a = 0.0
	visible = false


## Annonce `text` (entrée, pause, sortie : `TOTAL_SECONDS` en tout).
func announce(text: String) -> void:
	last_text = text
	_label.text = text
	if _tween != null and _tween.is_valid():
		_tween.kill()
	if DisplayServer.get_name() == "headless":
		visible = false
		return
	reset_size()
	var view := get_viewport_rect().size
	var rest := Vector2(round((view.x - size.x) * 0.5), round(view.y * CENTER_Y - size.y * 0.5))
	var reduce := Accessibility.reduce_motion()
	var slide := Vector2.ZERO if reduce else Vector2(0.0, SLIDE_PX)
	var fade := FADE_SECONDS * (0.5 if reduce else 1.0)
	position = rest + slide
	modulate.a = 0.0
	visible = true
	_tween = create_tween()
	_tween.set_parallel(true)
	_tween.tween_property(self, "modulate:a", 1.0, fade)
	_tween.tween_property(self, "position", rest, fade).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_OUT)
	_tween.chain().tween_interval(TOTAL_SECONDS - 2.0 * fade)
	_tween.chain().tween_property(self, "modulate:a", 0.0, fade)
	_tween.parallel().tween_property(self, "position", rest - slide, fade).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN)
	_tween.chain().tween_callback(hide)
