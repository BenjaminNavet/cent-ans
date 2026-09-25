# WIP — UI1 : habillage « manuscrit enluminé » de l'interface

Demande (2026-09-25) : l'UI n'est qu'une bande sépia et des boutons plats ; la rendre plus
belle, avec des assets « moines », enluminures.

## Approche
- Générateur procédural Pillow/numpy `tools/cent_ans_tools/ui_illumination.py`
  (`uv run --project tools cent-ans assets ui-illumination`) → PNG 9-slice dans
  `game/assets/ui/illumination/` : vélin (tuile périodique), filets d'or, rinceaux à feuilles
  de lierre, baguettes azur/gueules, boutons-cartouches, bulles, onglets, barres.
- `parchment_theme.tres` : `StyleBoxFlat` → `StyleBoxTexture` (tous les écrans d'un coup),
  variations `IlluminatedPanel` (grandes fenêtres) et `TopBarPanel` (bandeau du haut).
- `HudStyle.panel_box` → texture ; `card_box` reste plat (couleurs sémantiques).
- Captures avant/après : `godot --path game --script res://tests/ui1_capture.gd -- --out=<dir>`.

## État
- [ ] Générateur + textures
- [ ] Thème
- [ ] Bandeau du haut, grandes fenêtres
- [ ] Captures, smoke, docs

## Prochaine étape
Écrire le générateur.
