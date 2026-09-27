# PO2 — Typographie et finition UI — fichier de reprise

Branche `feat/po2-typo` (depuis `feat/po-polish`). Plan : `docs/superpowers/plans/2026-09-27-po-polish.md`
§ « PO2 ». Référence normative : bible DA § 12 (`docs/design/2026-09-25-bible-da.md`), ADR `docs/decisions/0097-gabarit-interface-et-etalonnage.md`.

Tailles de base (hauteur de référence 900 px) : `Title` 26, `Heading` 20, `Body` 17, `Caption` 14.

## État — TERMINÉ (toutes les étapes du plan faites)

- [x] Squelette `game/scripts/ui/ui_type.gd` (`class_name UiType`, constantes `TITLE/HEADING/BODY/CAPTION`,
      `size(variation)`, `apply(control, variation)`).
- [x] Thème `parchment_theme.tres` : variations de type `Title`/`Heading`/`Body`/`Caption`
      (police IM Fell English pour Title/Heading, EB Garamond pour Body/Caption, couleurs
      d'encre), constantes d'espacement `Spacing/constants/space_xs|s|m|l` (4/8/16/24). États de
      bouton (survol/pressé/désactivé/focus) déjà présents depuis UI1
      (`SBT_button_hover/pressed/disabled` + `StyleBoxFlat_focus`) — vérifiés, pas retouchés.
- [x] Remplacement de tous les `add_theme_font_size_override` des 12 scripts de la tranche par
      `UiType.apply(control, VARIATION)`, et des tailles littérales passées aux aides de style
      partagées (`FrontEndStyle.label/style_menu_button/style_action_button`,
      `HudStyle.label`) par `UiType.size(VARIATION)` (ces deux fichiers sont hors liste, non
      modifiés — seuls les arguments côté appelant, dans mes fichiers, changent).
  - `army_strip.gd` : `fit_name` réduisait le nom d'unité jusqu'à 10 px pour tenir dans la carte ;
    ramené à une seule taille (`Caption`, 14 px) et à l'abréviation seule au-delà (plus de
    réduction sous le plancher bible § 12.2).
  - **Exception assumée** : le grand guillemet ouvrant « « » de `loading_screen.gd` (64 px) reste
    tel quel — fleuron décoratif d'écran de chargement, pas un texte catalogué.
  - **Exception assumée** : `end_turn_cluster.gd` dessine la cloche et les alertes en
    `draw_string()` immédiat (12-15 px) sur un cadran à taille fixe (rayon ~38 px) — texte
    d'instrument graphique, pas un `Control` du système de thème ; non retouché (retouche = refonte
    du cadran, hors mécanique de ce lot).
- [x] `game/scripts/ui/ui_motion.gd` (`class_name UiMotion`) : `fade_in`/`fade_out` (fondu
      0,12 s + glissement 8 px par `Tween`), respecte `Settings.access/reduce_motion` (glissement
      supprimé, fondu gardé), rien en headless (état final appliqué tout de suite). `fade_in` joue
      `UiSounds.play("ui_open")`.
- [x] Sons `ui_click` / `ui_open` ajoutés à `data/audio/sound_bank.json` ; clips générés par
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
- [x] C3 activé dans `game/tests/po_ui_test.gd` (C1/C2 restent désactivés, à PO1) : boot réel du
      menu-titre + choix de faction (`start_menu.tscn`) et de la carte de campagne (vraie
      simulation, comme `smoke.gd`/`holdings_test.gd`) — barre du haut, panneau de province,
      panneau de colonie (première colonie de la première province), bandeau d'ost (armée
      sélectionnée), cloche de fin de tour, rapport de saison. Parcourt les `Label`/
      `RichTextLabel`/`Button`/… visibles et vérifie : aucune taille sous 14 px, 4 tailles au
      plus. **Résultat : OK, exactement `[14, 17, 20, 26]`.**
  - Deux exclusions documentées dans le test lui-même (`_OUT_OF_LOT_SCENE_LABELS`,
    filtre `*List`) :
    1. Conteneurs remplis par `PanelWidgets` (garnison, constructions, recrutement, colonies —
       nommés `*List`), aide partagée hors liste de fichiers du lot : nettoyage de phase 2.
    2. Étiquettes posées directement dans une `.tscn` avec une taille figée dans la scène, et non
       par le script du lot : `campaign_map.tscn` (barre du haut — `FactionLabel`,
       `TreasuryLabel` 15 px, `IncomeLabel` 15 px, `ResearchLabel` 12 px) et
       `province_panel.tscn` (`NameLabel` 24 px, `GarrisonHeader` 18 px). Ces deux `.tscn` ne sont
       pas dans la liste de fichiers du lot PO2 (qui ne cite que les scripts `.gd`) : je ne les ai
       pas touchées. À corriger par PO1 (`TOP_BAR`/`SIDE_PANEL`, dont les fichiers de PO1
       recouvrent `province_panel.gd` et la barre du haut) ou en phase 2.
- [x] `smoke.gd` vert (tourné après chaque lot de remplacements).

## Occurrences de `add_theme_font_size_override` restantes hors tranche

162 (sur 180 au départ ; 180-162 = 18 remplacements directs comptabilisés par ce grep — les
2 restants du delta de 20 étaient des tailles littérales passées à `FrontEndStyle`/`HudStyle`,
pas des `add_theme_font_size_override`). Nettoyage complet en phase 2 (P2a-P2f, ADR 0097).

## Fichiers touchés

`game/scenes/ui/parchment_theme.tres` ; `game/scripts/ui/ui_type.gd` (+`.uid`) ;
`game/scripts/ui/ui_motion.gd` (+`.uid`) ; `game/scripts/map/map_ui.gd`,
`province_panel.gd`, `settlement_panel.gd` ; `game/scripts/ui/{start_menu,faction_select,
loading_screen,army_strip,end_turn_cluster,season_report}.gd` ; `data/audio/sound_bank.json` ;
`tools/cent_ans_tools/ui_sounds.py` ; `game/assets/audio/ui/{click,open}.wav(.import)` +
`SOURCE.md` ; `game/tests/po_ui_test.gd`.
Non touchés (hors liste, cf. écarts ci-dessus) : `game/scripts/ui/front_end_style.gd`,
`game/scripts/ui/hud_style.gd`, `game/scripts/map/panel_widgets.gd`,
`game/scripts/audio/audio_director.gd`, `game/scenes/campaign_map.tscn`,
`game/scenes/ui/province_panel.tscn`.

## Prochaine étape (pour PO1/PO6/phase 2, pas pour ce lot)

- PO1 : en migrant `TOP_BAR` et `SIDE_PANEL`, profiter du passage dans `campaign_map.tscn` /
  `province_panel.tscn` pour aussi corriger `FactionLabel`/`TreasuryLabel`/`IncomeLabel`/
  `ResearchLabel`/`NameLabel`/`GarrisonHeader` (voir liste ci-dessus).
- Trancher le doublon de son de clic (`AudioDirector` vs `UiSounds`/`UiMotion`), voir écart noté.
- Phase 2 (P2a-P2f) : `panel_widgets.gd`, `front_end_style.gd`, `hud_style.gd` et les 162
  occurrences restantes de `add_theme_font_size_override` hors tranche.
