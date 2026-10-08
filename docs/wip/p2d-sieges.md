# P2d — sièges et résultat naval (WIP)

Chantier PO phase 2 (`docs/wip/po.md`). Branche `feat/p2d-sieges` depuis `main` (phase 1 fusionnée :
`UiLayout`/`UiZones`, `UiType`, `UiMotion` existent déjà). Modèle : P2c/P2g (`docs/wip/p2c-codex.md`,
`docs/archive/chantiers.md`).

## Périmètre trouvé (`grep -rln -i 'siege\|naval' game/scripts/ui game/scripts/map game/scenes`)

Écrans retenus (campagne + décisions liées à un siège/une interception navale, hors 3D interactif) :

- `game/scripts/map/siege_controller.gd` — boîte inline (statut + « Donner l'assaut ») dans
  `army_actions_box`. Pas de `add_theme_font_size_override` : rien à migrer, juste vérifié.
- `game/scripts/map/capture_controller.gd` + `game/scripts/ui/chronicle_window.gd` — **choix du
  sort de la ville prise** (TW2-T1 : occuper/rançonner/piller/raser). Était dans
  `UiZones.Zone.SIDE_PANEL` (384 px de large à 1280×720) : la largeur minimale de `ChronicleWindow`
  (620 px) y débordait déjà à l'écran (mesuré avec `p2d_ui_test.gd`) — passé en `Zone.MODAL` dans
  `capture_controller.gd` (même famille que la rencontre/déclaration de guerre, PO1) ; ne change
  pas `chronicle_controller.gd` (fenêtre séparée, hors liste de fichiers, toujours en `SIDE_PANEL`,
  pas mesuré ici). Tailles migrées vers `UiType`.
- `game/scripts/battle/pre_battle_dialog.gd` (+ `game/scripts/naval/naval_pre_battle_dialog.gd`) —
  fenêtre d'avant-bataille commune aux batailles rangées, **assauts de siège** (`siege: true`) et
  **interceptions navales** (résolution automatique : `PLAYABLE_3D = false` depuis le 25/09, seul
  écran vu par le joueur pour le naval). Pas de zone `UiLayout` (plein écran auto-centré,
  `_layout()`) : je garde ce fonctionnement (modale déjà plein écran avec voile), je migre
  seulement les tailles.
- `game/scripts/naval/naval_campaign.gd` — pas d'UI propre (toasts/évènements génériques déjà en
  place) : rien à faire.

## Hors périmètre (domaine « bataille » 3D interactif, pas propre au siège, ou interdit)

- `battle_siege.gd`, `battle_alerts_column.gd`, `siege_capture_points.gd`, infobulles : interdits
  (autres sessions).
- `battle_result_screen.gd`, `naval_hud.gd` (HUD 3D), `landmark_siege_town.gd` (FX 3D) : écran de
  fin joué en 3D, partagé à l'identique par bataille rangée et siège (pas de règle propre au
  siège), système de typographie `BattleUiKit` distinct déjà cohérent (mêmes polices que
  `UiType` : IM Fell English / EB Garamond) — laissés tels quels, hors du périmètre « campagne
  côté siège/naval » de ce lot.

## Choix de migration

- `chronicle_window.gd` (théorique, piloté par le thème `parchment_theme.tres`) : tailles →
  `UiType.apply` (Title 26 / Caption 14 / Body 17 exactement déjà en place, juste pas via
  `UiType`). `RichTextLabel` : `UiType.apply` + `italics_font_size` posé à la même taille (l'aide
  ne couvre que `normal_font_size`).
- `pre_battle_dialog.gd` / `naval_pre_battle_dialog.gd` (police propre `BattleUiKit`, polices IM
  Fell English / EB Garamond déjà identiques à celles du thème) : remplace les tailles littérales
  par `UiType.size(UiType.XXX)` dans les appels `BattleUiKit.label(...)` /
  `BattleUiKit.button_font(...)` existants, sans toucher au choix de police ni de couleur (déjà
  posé explicitement, `UiType.apply` ne les aurait pas changés non plus mais named calls suffisent
  ici et restent moins intrusifs sur un fichier partagé avec la bataille rangée).
  Correspondance (valeur d'origine → `UiType`) : 38/26/24/22→Title(26) ou Heading(20) selon le
  rôle (bannière = Title, CTA/nom de camp = Heading), 19/17/16→Body(17), 15/14/13→Caption(14).

## Défaut préexistant trouvé, non corrigé (signalé, pas tranché)

`PreBattleDialog._layout()` (siège **et** bataille rangée **et**, via `NavalPreBattleDialog`,
naval) tente de brider `panel.size` à `view - 32` : `height := minf(panel.get_combined_
minimum_size().y, minf(PANEL_MAX.y, view.y - 32.0)) ; panel.size = Vector2(width, height)`. Ça ne
marche pas quand le contenu réel dépasse ce budget : un `Control` ne peut pas être réduit sous la
taille minimale combinée de ses enfants — Godot ramène silencieusement `panel.size` à
`panel.get_combined_minimum_size()`. Vérifié par instrumentation ponctuelle de `_layout()` (non
commise) sur un siège avec les armées principales du début de partie (`debug_stage_siege`) : la
hauteur minimale réelle des colonnes de régiments dépasse le budget visé à 1280×640 (et parfois à
1280×720). Pas propre au siège (même classe pour les batailles rangées), pas causé par la
migration de tailles de ce lot (écarts de quelques px seulement, jamais des centaines). Une vraie
correction (rendre les colonnes défilantes, sur le modèle du corps de `chronicle_window.gd` ajouté
ici pour la fenêtre de sort de ville) dépasse le périmètre d'un lot de migration de tailles et
touche un fichier partagé avec la bataille rangée et le chantier CB : **pas tranché ici**. Rendu
non bloquant dans `p2d_ui_test.gd` (diagnostic affiché, pas d'échec) ; `dialog.panel` (pas le
voile plein écran) est bien la mesure qui compte pour C2.

## État

- [x] squelette (cette note, `game/tests/p2d_ui_test.gd`)
- [x] `chronicle_window.gd` → `UiType` + corps défilant borné (`BODY_MAX_RATIO`) pour C2
- [x] `pre_battle_dialog.gd` → `UiType.size`
- [x] `naval_pre_battle_dialog.gd` → `UiType.size`
- [x] `capture_controller.gd` → zone `MODAL` (au lieu de `SIDE_PANEL`, débordait)
- [x] `p2d_ui_test.gd` (C1-C3 : fenêtre de sort de ville, dialogue de siège, dialogue naval)
- [x] tests verts : smoke, po_ui_test, p2c_ui_test, p2g_ui_test, p2d_ui_test, tw2_t1_capture_test,
      ub1_ui_test, nv1_naval_test
- [x] rapport final

## Prochaine étape

Lot terminé côté P2d. Reste ouvert pour l'orchestrateur : le défaut `PreBattleDialog._layout()`
ci-dessus (colonnes non défilantes) — à trancher en dehors de ce lot (fichier partagé bataille
rangée + CB).
