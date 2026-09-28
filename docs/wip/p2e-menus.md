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
- [x] `credits_screen.gd` migré (`_panel` en `MODAL`, `self` garde le fond `MenuBackground` hors
      zone ; tailles bbcode incrustées converties via `UiType.size(...)`).
- [x] `settings_menu.gd` migré (un seul point de rattachement pour ses deux appelants).
- [x] `pause_menu.gd` migré : `_menu_panel`/`_confirm_panel`/`save_dialog` rejoignent `MODAL`
      individuellement ; `self` reste un coordinateur invisible. Ajout de
      `_free_modal_children()` sur `tree_exiting` car `flow_controller.gd` (hors lot) ne fait
      que `pause_menu.queue_free()`, qui ne libérerait plus ces enfants déplacés.
- [x] `save_load_dialog.gd` + `.tscn` : tailles → `UiType`, `UiMotion` sur `open_save`/`open_load`/
      `close`. Rattachement (zone/parent) **non touché** : voir « Points ouverts ».
- [x] `save_slots.gd` : aucune UI, aucun changement.
- [x] `battle_loading_card.gd` : tailles brutes → `UiType.size(...)`, structure `CanvasLayer`
      inchangée (hors zones `UiLayout`, comme `loading_screen.gd`).
- [x] `game/tests/p2e_ui_test.gd` écrit (C1/C2 adapté/C3) — **aides dupliquées, pas réutilisées**
      depuis `po_ui_test.gd` : voir « Points ouverts ».
- [x] `game/tests/p2e_shot.gd` écrit (7 vues, `docs/img/po/p2e/`).
- [ ] dylib copiée, import fait ; smoke test relancé — en cours (machine très chargée, plusieurs
      builds Rust d'autres lots tournent en parallèle).
- [ ] lancer `p2e_ui_test.gd`, `p2e_shot.gd` (une fois, pas de relecture d'image)
- [ ] merge `main`, réimport, tests finaux, rapport

## Points ouverts (à trancher par le joueur/orchestrateur si besoin)

1. **`SaveLoadDialog` non rattaché à une zone `UiLayout` par lui-même.** Il est instancié dans
   trois fichiers : `pause_menu.gd` (mon lot — rejoint `MODAL` explicitement), `start_menu.gd` et
   `map_ui.gd` (hors lot, l'un avec `panels.register(save_load_dialog, PanelStack.Kind.MODAL)`,
   l'ancien système d'étages). Faire rejoindre `MODAL` depuis le `_ready()` de `save_load_dialog.gd`
   lui-même aurait déplacé ces deux instances partagées hors de mon lot, sans pouvoir vérifier les
   effets de bord (ordre d'affichage `PanelStack`, éventuels appels `move_child` dans ces fichiers).
   Je n'ai donc migré que ses tailles de police, pas son rattachement, en dehors de l'instance de
   `pause_menu.gd`.
2. **`p2e_ui_test.gd` ne réutilise pas littéralement les aides de `po_ui_test.gd`** comme demandé.
   `po_ui_test.gd` `extends SceneTree` et son `_init()` lance tout seul l'ensemble de ses tests puis
   appelle `quit()` : instancier ce script (`preload(...).new()`) aurait exécuté *et terminé*
   `po_ui_test.gd` avant même mon premier test. J'ai donc reproduit ses aides
   (`_collect_font_sizes`, `_collect_tool_texts`, motifs C1, `MIN_SIZE`/`MAX_DISTINCT_SIZES`)
   plutôt que de les appeler sur une instance. Signalé pour arbitrage si une vraie factorisation
   (par ex. extraire ces aides dans une classe `RefCounted` séparée, touchant `po_ui_test.gd`) est
   préférée — je ne l'ai pas fait puisque la consigne était de ne pas modifier ce fichier.

## Prochaine étape

Une fois la machine moins chargée : relancer `smoke.gd`, puis `p2e_ui_test.gd` et `p2e_shot.gd`,
fusionner `main`, réimporter, relancer tous les tests, puis répondre à l'orchestrateur.
