# PO1 — Disposition fixe (`feat/po1-layout`)

Plan : `docs/superpowers/plans/2026-09-27-po-polish.md` § PO1. Bible DA § 12.1, ADR 0097.

## État
- [x] 1. `UiLayout` implémenté + tests « UiLayout » dans `po_ui_test.gd`
- [x] 2. Migration carte de campagne (TOP_BAR, BOTTOM_SELECTION, MINIMAP, SIDE_PANEL, TOASTS, MODAL)
- [x] 3. Textes d'outil en mode dev seulement (avis de cache du relief)
- [x] 4. Bataille : cartes d'unités → BOTTOM_SELECTION, fin de bataille → MODAL, journal → TOASTS
- [x] 5. C1/C2 actifs ; menu titre à 720 px ; tailles figées des `.tscn` → `UiType`

## Conception
- `ui_layout.gd` porte `class_name UiZones` : les scripts à `class_name` sont compilés avant les
  autoloads (un test qui les charge échoue sur « Identifier not found: UiLayout »). Ils passent
  par `UiZones.put(zone, c)`, `UiZones.rect(zone)`, `UiZones.layout()`, `UiZones.Zone`.
  **Après fusion : `godot --headless --path game --import`** (cache des classes globales).
- Un hôte (`attach_host`, ici `MapUI`) porte six `Control` nus ancrés (taille minimale nulle) :
  les zones ne grandissent jamais. `SIDE_PANEL` : chaque occupant dans une enveloppe
  `ScrollContainer` (le panneau de province, 736 px, défile dans une zone de 488 px à 720p).
  `MODAL` : voile noir 45 % + occupant centré. `TOASTS` : pile verticale (avis éphémères en haut).
- Réclamer **avant** d'inscrire dans `PanelStack` : un reparentage émet `tree_exiting`, qui
  désinscrit le panneau de la pile.

## Écarts
- `TOP_BAR` 0,05 → 0,08 (barre mesurée 64 px, hauteur logique 800 px à 1280×720, échelle bornée
  à 0,9) ; `SIDE_PANEL` et `TOASTS` commencent à 0,09. À reporter dans la bible § 12.1.
- Cloche de fin de tour : hors zone (enfant direct de `MapUI`), calée en bas contre le bord gauche
  de la minicarte. La zone `MINIMAP` (0,18 × 0,28) ne peut porter minicarte + cloche (194×136) ;
  ses pastilles d'alerte débordent vers le haut et ne doivent pas être coupées.
- Zones `TOP_BAR` et `BOTTOM_SELECTION` sans `clip_contents` : le bandeau d'ost (170 px) dépasse
  de 10 px la zone de 160 px à 720p ; couper son en-tête serait pire.
- Fin de partie (`CampaignEnding`, `victory_controller.gd`, lot PO5) non migrée : déjà plein écran.
- Hors liste du lot, touchés a minima : `chronicle_window.gd` (centrage dans le parent),
  `army_movement_controller.gd` (déclaration de guerre → `MODAL`), `zg7b_cache_test.gd`, `smoke.gd`.

- Écran de fin de bataille (`BattleResultScreen`) non migré en `MODAL` : écran plein avec son
  voile et son enluminure AR1 (fond plein écran) ; le forcer dans la boîte 0,6 × 0,76 couperait
  l'enluminure. À refaire en phase 2 si voulu.
- Battle : `UiZones.attach_host(root)` ; les hôtes s'empilent (la bataille se pose sur la carte,
  la carte redevient l'hôte à sa sortie).

## Vérifications (fin de lot)
`po_ui_test` (UiLayout, C1, C2, C3), `smoke`, `unit_roster_test`, `cv3_4_ui_test`,
`ui_panel_stack_test`, `hud_components_test`, `ux2_test`, `ub1_ui_test`, `zg7b_cache_test` : OK.
`ux1_test` : erreur de compilation préexistante (`map_mode_controller.gd`, `SimFacade`).
Captures `docs/img/po/po1/` (vues 1, 3-8).

## Prochaine étape
Lot terminé ; relecture visuelle des captures par l'orchestrateur.
