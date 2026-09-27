# PO2 — Typographie et finition UI — fichier de reprise

Branche `feat/po2-typo` (depuis `feat/po-polish`). Plan : `docs/superpowers/plans/2026-09-27-po-polish.md`
§ « PO2 ». Référence normative : bible DA § 12 (`docs/design/2026-09-25-bible-da.md`), ADR `docs/decisions/0097-gabarit-interface-et-etalonnage.md`.

Tailles de base (hauteur de référence 900 px) : `Title` 26, `Heading` 20, `Body` 17, `Caption` 14.

## État

- [x] Squelette `game/scripts/ui/ui_type.gd` (`class_name UiType`, constantes `TITLE/HEADING/BODY/CAPTION`,
      `size(variation)`, `apply(control, variation)`).
- [x] Sons `ui_click` / `ui_open` : ajoutés à `data/audio/sound_bank.json`, clips générés par
      `tools/cent_ans_tools/ui_sounds.py` (étendu avec 2 recettes) à partir de `sfx/ui_click.ogg`
      et `sfx/page_turn.ogg` (déjà dans le dépôt, synthèse procédurale du projet — pas de tiers,
      donc pas de `CREDITS.md` ; `game/assets/audio/ui/SOURCE.md` régénéré). Import Godot fait.
  - **Écart noté** : `AudioDirector` (`game/scripts/audio/audio_director.gd`, hors lot) connecte
    déjà un clic générique à *tous* les `BaseButton` de la partie (`_on_node_added` /
    `_on_button_pressed` → `play_sfx("ui_click")`, fichier `assets/audio/sfx/ui_click.ogg`) et un
    « page_turn » à l'ouverture des panneaux `*Panel` de la carte. Je n'ai donc PAS rebranché
    `UiSounds.play("ui_click")` sur les boutons du thème (double son garanti sur chaque clic) :
    seul `UiMotion.fade_in` appelle `UiSounds.play("ui_open")`. À trancher en PO1/PO6 : soit
    migrer les panneaux vers `UiMotion` seul (et couper le `page_turn` d'`AudioDirector` pour ces
    panneaux), soit l'inverse.
- [ ] Thème `parchment_theme.tres` : variations de type Title/Heading/Body/Caption + constantes
      d'espacement `space_xs/s/m/l`. États de bouton (survol/pressé/désactivé/focus) déjà présents
      via les StyleBoxTexture (`SBT_button_hover/pressed/disabled`) + `StyleBoxFlat_focus` — hérités
      d'UI1, à vérifier seulement.
- [ ] Remplacement des `add_theme_font_size_override` des 12 scripts de la tranche (voir liste ci-dessous).
- [ ] `game/scripts/ui/ui_motion.gd` (`class_name UiMotion`, `fade_in`/`fade_out`, rien en headless).
- [ ] Activer C3 dans `game/tests/po_ui_test.gd`.
- [ ] `smoke.gd` + `po_ui_test.gd` verts.

## Fichiers de la tranche (remplacement de taille, occurrences relevées avant travail)

| Fichier | Occurrences avant |
|---|---|
| `game/scripts/map/map_ui.gd` | 5 |
| `game/scripts/map/hud_controller.gd` | 0 |
| `game/scripts/map/province_panel.gd` | 2 |
| `game/scripts/map/settlement_panel.gd` | 3 |
| `game/scripts/map/outcome_notice.gd` | 0 |
| `game/scripts/map/encounter_controller.gd` | 0 |
| `game/scripts/ui/start_menu.gd` | 0 (tailles passées à `FrontEndStyle.label/style_menu_button`) |
| `game/scripts/ui/faction_select.gd` | 1 direct + tailles passées à `FrontEndStyle.label/style_action_button` |
| `game/scripts/ui/loading_screen.gd` | 0 |
| `game/scripts/ui/army_strip.gd` | 2 |
| `game/scripts/ui/end_turn_cluster.gd` | 0 |
| `game/scripts/ui/season_report.gd` | 7 |

Total avant : 180 dans tout `game/scripts` (dont 20 dans la tranche). `front_end_style.gd`
(helper partagé, hors liste du lot) pose la taille par un paramètre `size: int` explicite passé
par l'appelant : je ne touche pas ce fichier, je remplace les entiers littéraux passés depuis
`start_menu.gd`/`faction_select.gd` par `UiType.size(UiType.<VARIATION>)`.

## Prochaine étape

Écrire les variations de type + espacements dans `parchment_theme.tres`, puis remplacer les
tailles fichier par fichier (commits `wip:` groupés par fichier ou paire de fichiers), puis
`UiMotion`, puis activer C3.
