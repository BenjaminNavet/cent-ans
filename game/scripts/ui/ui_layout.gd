extends Node

## Chantier PO (ADR 0097, bible DA § 12.1) : autoload `UiLayout`, zones d'écran fixes. Chaque
## panneau réclame une zone au lieu de se placer en coordonnées absolues ; les rectangles sont
## fixés par des ancres en proportion de l'écran et ne dépendent jamais de la taille minimale des
## enfants (boucles de mise en page vues en UI1).
## - `SIDE_PANEL` n'a qu'un occupant : en réclamer un ferme le précédent (`side_panel_changed`).
## - `MODAL` assombrit le fond et bloque les entrées derrière.
## - `TOASTS` empile 3 avis au plus (conseiller, annonces, avis de résultat), effacés après `seconds`.
## PO0 : squelette (API publique, corps vides) ; implémentation au lot PO1.

signal side_panel_changed(control: Control)

enum Zone { TOP_BAR, BOTTOM_SELECTION, MINIMAP, SIDE_PANEL, TOASTS, MODAL }

## Rectangles des zones en part de l'écran (x, y, largeur, hauteur), bible DA § 12.1.
const ZONE_RECTS := {
	Zone.TOP_BAR: Rect2(0.0, 0.0, 1.0, 0.05),
	Zone.BOTTOM_SELECTION: Rect2(0.01, 0.80, 0.80, 0.20),
	Zone.MINIMAP: Rect2(0.82, 0.72, 0.18, 0.28),
	Zone.SIDE_PANEL: Rect2(0.70, 0.06, 0.30, 0.64),
	Zone.TOASTS: Rect2(0.01, 0.07, 0.26, 0.50),
	Zone.MODAL: Rect2(0.2, 0.12, 0.6, 0.76),
}
const TOAST_SECONDS := 6.0
const MAX_TOASTS := 3


## Place `control` dans `zone` (reparenté sous le conteneur de la zone).
func claim(_zone: Zone, _control: Control) -> void:
	pass


## Retire `control` de sa zone (sans le libérer).
func release(_control: Control) -> void:
	pass


## Rectangle de `zone` en pixels de la fenêtre courante.
func zone_rect(_zone: Zone) -> Rect2:
	return Rect2()


## Avis éphémère dans la zone `TOASTS`.
func toast(_text: String, _icon: String = "", _seconds: float = TOAST_SECONDS) -> void:
	pass
