# P2c — codex, encyclopédie, infobulles riches (WIP)

Chantier PO phase 2 (orchestré par RS, `docs/wip/restes.md`). Spec/plan : ADR 0097, bible DA
§ 12, `docs/superpowers/plans/2026-09-27-po-polish.md` (section « Phase 2 »). Branche
`feat/p2c-codex` depuis `main` (phase 1 est fusionnée : `UiLayout`/`UiZones`, `UiType`,
`UiMotion` existent déjà dans `main`, vus dans `game/scripts/ui/ui_layout.gd`, `ui_type.gd`,
`ui_motion.gd`).

Fichiers du lot : `game/scripts/codex/codex_window.gd`, `codex_bubbles.gd`,
`game/scripts/ui/codex_hub.gd`, `encyclopedia.gd`, `rich_tooltip.gd` et leurs scènes.

## Choix de migration (mécanique, pas de nouvelle règle)

- **Zones** : `CodexHub` (fenêtre commune Histoire/Règles sur la carte) et la fenêtre `CodexWindow`
  autonome (`CodexBubbles.window()`, utilisée hors de la carte, ex. en bataille) rejoignent
  `UiZones.Zone.MODAL` — même famille que la rencontre et la déclaration de guerre (PO1) : grande
  fenêtre à onglets qui bloque la carte, pas un panneau docké. `CodexHub` reste aussi enregistrée
  dans `PanelStack.Kind.CENTRAL` par `map_ui.gd` (hors de mon lot, je n'y touche pas) : même
  double inscription que `EncounterController` (`UiZones.put` puis `register_panel`), le panneau
  central n'étant simplement plus retrouvé par `restack()` une fois reparenté — limite déjà
  acceptée pour les fenêtres modales existantes.
- Le reparentage de `CodexHub` se fait en différé (`call_deferred`) car `_ready()` tourne encore
  dans la pile de `map_ui.add_child(codex_hub)` : reparenter tout de suite lèverait l'erreur Godot
  « parent busy setting up children ».
- Les bulles du Codex (`CodexBubbles`) et les infobulles riches (`RichTooltip`) restent des
  popups flottants positionnés à la souris (pas de zone `UiLayout` : ni modale ni panneau latéral,
  cohérent avec leur nature d'infobulle) ; seules leurs tailles de police migrent vers `UiType`.
- Tailles : `Title` (en-têtes de fenêtre et de fiche), `Body` (listes, corps de fiche, encyclopédie),
  `Caption` (méta, sources, bulles et infobulles compactes — 14 px, identique à l'existant).

## Point en attente (`docs/wip/h8-codex-content.md`)

Déjà traité avant ce lot : `codex_window.gd` a ses libellés courts (3e élément de
`CodexStore.FAMILIES`) et `clip_tabs = false` depuis H8b. Rien à faire ici.

## État

- [x] Branche créée depuis `main`.
- [ ] `codex_window.gd` : tailles → `UiType` (fait), zone MODAL laissée à l'appelant (`CodexBubbles`).
- [ ] `codex_bubbles.gd` : bulle → `UiType.CAPTION` ; fenêtre autonome → `UiZones.Zone.MODAL` ;
      suppression de `_window_layer` (mort après le passage par `UiZones`).
- [ ] `codex_hub.gd` : titre + `style_tabs` → `UiType` ; fenêtre → `UiZones.Zone.MODAL` (déféré).
- [ ] `encyclopedia.gd` : tailles → `UiType` (contrôles et BBCode `[font_size=…]`).
- [ ] `rich_tooltip.gd` : tailles → `UiType.CAPTION`.
- [~] `game/tests/p2c_ui_test.gd` (C1-C3 sur mes écrans, aides de `po_ui_test.gd` réutilisées sans
      le modifier — instance de son script chargé pour les fonctions `_collect_font_sizes` /
      `_collect_tool_texts`). C1 et C3 verts (30 textes lus, tailles [14, 17, 26]). C2 en cours :
      la fenêtre `CodexWindow` autonome déborde de l'écran à 1280×720 et 1920×1080 quand elle est
      ouverte juste après avoir libéré une carte de campagne dans le même test — position figée à
      (540, 187) quelle que soit la résolution suivante, alors qu'une fenêtre isolée (sans carte
      créée avant) se centre bien. Un ajout de 6 `await process_frame` après `map.queue_free()`
      n'a pas changé le résultat (mêmes chiffres) : sonde de débogage en cours (impression de
      `UiLayout.host()` / `zone_rect(MODAL)` / parent du contrôle) pour trouver la vraie cause
      avant de conclure si c'est un artefact de mon test ou un vrai défaut de `UiZones` (hors de
      mes fichiers si c'est le cas).
- [ ] `game/tests/p2c_shot.gd` (captures 1280×720 dans `docs/img/po/p2c/`).
- [ ] `smoke.gd`, tests existants des bulles et du Codex (`tools/tests/test_codex*.py`,
      `game/tests/codex_screenshot.gd`, `encyclopedia_screenshot.gd`).
- [ ] Merge `main` (déjà à jour, branché dessus), réimport, rapport final.

## Prochaine étape

Migrer `codex_hub.gd`, `encyclopedia.gd`, `rich_tooltip.gd`, puis écrire les tests.
