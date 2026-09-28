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

## Écarts et corrections annexes
- Le voile `MODAL` couvre aussi la barre du haut : tant que Cour, Techniques ou Diplomatie sont
  ouvertes, leurs boutons ne servent plus à basculer d'une fenêtre à l'autre ; fermeture par « × »
  ou Échap (règle de la bible § 12.1 : la modale bloque les entrées derrière).
- Tailles minimales qui débordaient déjà avant P2g (mesuré sur `main`) : liste de la Cour
  (480 → 240 px), corps de la fiche (640 → 320 px), carte de la diplomatie (taille minimale à
  cliquet, remise au plancher puis réajustée une image plus tard ; panneau calé sous la barre du
  haut). Tout tient désormais à 1280×720, 1280×640 et 1920×1080.
- `CodexHub` (P2c) était désinscrit de la pile par son reparentage différé : inscription reportée
  après lui dans `map_ui`.
- Godot : `set_anchor(…, push_opposite = false)` ramène l'ancre gauche/haut à l'opposée si elle la
  dépasse ; la conversion pousse donc l'opposée pour gauche/haut.

## État
- [x] squelette (cette note, `game/tests/p2g_ui_test.gd` désactivé)
- [x] `map_ui` : zone `MODAL` + conversion d'ancres + `_keep_on_screen` global
- [x] `diplomacy_panel._fit_to_viewport` en coordonnées globales
- [x] `start_menu` : `SaveLoadDialog` dans `MODAL`
- [x] `p2g_ui_test.gd` (C1, C2 à 1280×720 / 1280×640 / 1920×1080, C3)
- [ ] tests : smoke, po_ui, p2a, p2b, p2c, p2e, p2g

## Prochaine étape
Faire passer `p2g_ui_test.gd` puis les tests existants (smoke, po_ui, p2a/b/c/e).
