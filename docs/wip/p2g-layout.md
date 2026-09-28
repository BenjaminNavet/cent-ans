# P2g — fenêtres centrales et `SaveLoadDialog` dans `UiLayout`

Chantier PO phase 2 (`docs/wip/po.md`, `docs/wip/restes.md`). Branche `feat/p2g-layout`.
Les fenêtres Techniques, Diplomatie, Cour, Fiche de personnage et `SaveLoadDialog` (depuis
`start_menu` et `map_ui`) étaient restées hors de `UiLayout` : les reparenter dans la zone `MODAL`
cassait `map_ui._keep_on_screen` (condition `panel.get_parent() == self`, décalage en coordonnées
locales) et leur géométrie (ancres pensées pour un parent plein écran).

## Approche
- `map_ui` réclame ces panneaux dans `UiZones.Zone.MODAL` (voile, étage `MODAL`, `modal_open()`)
  puis **convertit leurs ancres** du repère écran au repère de la zone
  (`a' = (a - zone.x) / zone.w`, décalages en pixels inchangés) : la géométrie reste identique à
  toute résolution, sans dépendre de la taille de la zone.
- `_keep_on_screen` travaille en coordonnées globales et accepte les panneaux de la zone `MODAL`
  (plus seulement les enfants directs de `map_ui`).
- `SaveLoadDialog` : même motif que `pause_menu.gd` (P2e) — `UiZones.put(MODAL, …)`, centré à sa
  taille. Dans `start_menu`, l'hôte par défaut de `UiLayout` vit hors de la scène : le dialogue est
  libéré avec le menu.

## État
- [ ] squelette (cette note, `game/tests/p2g_ui_test.gd` désactivé)
- [ ] `map_ui` : zone `MODAL` + conversion d'ancres + `_keep_on_screen` global
- [ ] `diplomacy_panel._fit_to_viewport` en coordonnées globales
- [ ] `start_menu` : `SaveLoadDialog` dans `MODAL`
- [ ] `p2g_ui_test.gd` (C1, C2 à 1280×720 / 1280×640 / 1920×1080, C3)
- [ ] tests : smoke, po_ui, p2a, p2b, p2c, p2e, p2g

## Prochaine étape
Implémenter `map_ui._claim_modal_panel`.
