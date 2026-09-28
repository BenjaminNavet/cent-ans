# P2d — sièges et résultat naval (WIP)

Chantier PO phase 2 (`docs/wip/po.md`). Branche `feat/p2d-sieges` depuis `main` (phase 1 fusionnée :
`UiLayout`/`UiZones`, `UiType`, `UiMotion` existent déjà). Modèle : P2c/P2g (`docs/wip/p2c-codex.md`,
`docs/wip/p2g-layout.md`).

## Périmètre trouvé (`grep -rln -i 'siege\|naval' game/scripts/ui game/scripts/map game/scenes`)

Écrans retenus (campagne + décisions liées à un siège/une interception navale, hors 3D interactif) :

- `game/scripts/map/siege_controller.gd` — boîte inline (statut + « Donner l'assaut ») dans
  `army_actions_box`. Pas de `add_theme_font_size_override` : rien à migrer, juste vérifié.
- `game/scripts/map/capture_controller.gd` + `game/scripts/ui/chronicle_window.gd` — **choix du
  sort de la ville prise** (TW2-T1 : occuper/rançonner/piller/raser). Déjà dans
  `UiZones.Zone.SIDE_PANEL` ; tailles à migrer vers `UiType`.
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

## État

- [ ] squelette (cette note, `game/tests/p2d_ui_test.gd`)
- [ ] `chronicle_window.gd` → `UiType`
- [ ] `pre_battle_dialog.gd` → `UiType.size`
- [ ] `naval_pre_battle_dialog.gd` → `UiType.size`
- [ ] `p2d_ui_test.gd` (C1-C3 : fenêtre de sort de ville, dialogue de siège, dialogue naval)
- [ ] tests : smoke, po_ui_test, p2c_ui_test, p2g_ui_test, p2d_ui_test, tw2_t1_capture_test,
      ub1_ui_test, nv1_naval_test
- [ ] rapport final

## Prochaine étape

Migrer `chronicle_window.gd`, puis les deux dialogues d'avant-bataille, puis écrire le test.
