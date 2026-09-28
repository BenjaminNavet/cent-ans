# P2e — menus secondaires — fichier de reprise

Chantier PO phase 2 (`docs/wip/po.md`), lot **P2e** : migration mécanique vers `UiLayout` /
`UiType` / `UiMotion` des menus secondaires (démos, rejeux, batailles historiques, réglages,
pause, crédits, sauvegarde, chargement). Sans nouvelle règle de style (bible DA § 12, ADR 0097).

Branche `feat/p2e-menus` (depuis `main`), worktree
`.claude/worktrees/agent-a9c8f8637b20e240c`.

## Fichiers du lot

`game/scripts/ui/battle_demos_menu.gd`, `replays_menu.gd`, `historical_battles_menu.gd`,
`settings_menu.gd`, `pause_menu.gd`, `credits_screen.gd`, `save_load_dialog.gd`, `save_slots.gd`,
`loading_screen.gd`, `battle_loading_card.gd` et leurs scènes.

## Décisions de migration

- Les menus autonomes qui construisaient eux-mêmes un voile + `CenterContainer` (battle demos,
  rejeux, batailles historiques, crédits) deviennent des `PanelContainer` qui rejoignent
  directement la zone `MODAL` de `UiLayout` (`UiZones.put(UiZones.Zone.MODAL, self)`) : la zone
  fournit déjà le voile à 45 % et le blocage des clics, plus besoin de veil ni de
  `CenterContainer` propres. Fondu d'ouverture/fermeture par `UiMotion.fade_in`/`fade_out`
  (fermeture : `fade_out(self, DURATION, true)` avant `queue_free`).
- `SettingsMenu` : même traitement (devient `PanelContainer`, s'auto-réclame en `MODAL` dans
  `_ready`) — un seul point de changement qui couvre ses deux points d'ouverture existants
  (`pause_menu.gd` et `flow_controller.gd`, ce dernier hors lot, non modifié).
- `PauseMenu` : reste un `Control` coordinateur (signaux, F1 aide, confirmation de sortie) mais
  ne construit plus de voile ; `_menu_panel` et `_confirm_panel` (déjà des `PanelContainer`)
  rejoignent `MODAL` individuellement, ainsi que sa propre instance de `SaveLoadDialog` (celle
  instanciée dans `pause_menu.gd` uniquement — voir point ouvert ci-dessous).
- `SaveLoadDialog` (`save_load_dialog.gd` + `.tscn`) : **je n'ai pas touché à son rattachement**
  (`add_child` dans `start_menu.gd` et `map_ui.gd`, tous deux hors lot, l'un avec
  `panels.register(..., PanelStack.Kind.MODAL)`). Le faire rejoindre `UiLayout` universellement
  depuis son propre `_ready()` aurait déplacé ces deux instances partagées hors de mon lot sans
  pouvoir vérifier leurs effets de bord (ordre d'affichage, `PanelStack`). Seule ma migration des
  tailles de police (`UiType`) s'applique partout où il est instancié. **Point ouvert signalé au
  joueur/orchestrateur** : la boîte de dialogue de sauvegarde garde sa disposition et son voile
  d'origine (hérités de son parent), sauf dans le menu pause où elle rejoint `MODAL` explicitement
  depuis `pause_menu.gd` (instance propre à ce fichier, sans effet sur les deux autres).
- `save_slots.gd` : classe de données pure (aucune UI, aucune taille de police) — non modifiée.
- `loading_screen.gd` et `battle_loading_card.gd` : écrans de transition plein écran par
  `CanvasLayer` à étage élevé (100 / 110), **hors zones `UiLayout`** (le motif existant de
  `loading_screen.gd`, déjà migré en PO2, confirme ce choix : transitions, pas des fenêtres de
  choix). Seule migration : tailles de police brutes → `UiType` dans `battle_loading_card.gd`
  (`loading_screen.gd` était déjà migré).

## État (mis à jour à chaque commit)

- [x] Squelette de reprise (ce fichier), branche créée.
- [x] `battle_demos_menu.gd` migré (MODAL, UiType, UiMotion).
- [x] `historical_battles_menu.gd` migré.
- [x] `replays_menu.gd` migré.
- [ ] `credits_screen.gd`
- [ ] `settings_menu.gd`
- [ ] `pause_menu.gd`
- [ ] `save_load_dialog.gd` (+ `.tscn`)
- [ ] `battle_loading_card.gd`
- [ ] `game/tests/p2e_ui_test.gd`
- [ ] `game/tests/p2e_shot.gd` + captures `docs/img/po/p2e/`
- [ ] `smoke.gd` + tests existants relancés
- [ ] merge `main`, réimport, tests finaux

## Prochaine étape

Migrer `credits_screen.gd` (même patron MODAL que les trois menus déjà faits, plus les tailles
bbcode incrustées dans `markdown_to_bbcode`), puis `settings_menu.gd` et `pause_menu.gd`.
