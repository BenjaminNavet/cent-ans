# TS-T6 — plaques de ville et fil d'événements

Branche `ts/t6` (worktree `../gp-ts-t6`). Plan : `ts-style-total-war.md`, ADR 0710.

## Choix
- Plaque : le libellé existant (`markers/mod.rs`, colonne écu + nom) devient un cartouche en rangée
  (écu | nom | pastilles). Statut (propre / allié / neutre / ennemi) lu dans `OverlayState.view`
  (`ProvinceView::stance_of`, donc le cœur) ; couleurs dans `data/map/stance_cues.json` § `town.plate`.
  Pastilles (siège, garnison, bâtiments) lues sur `SettlementState` du cœur, infobulle `Tooltip`.
- Fil : nouveau panneau compact repliable en haut à droite (`hud/feed.rs`), moins intrusif que de
  refondre la colonne d'alertes (pastilles de fin de tour, gardées telles quelles). Source : lignes de
  tour du `TurnJournal` (même source que `ChronicleLog`), catégories dans `data/ui/event_feed.json`.

## État
- [ ] squelette
- [ ] plaque (statut, pastilles, dump, tests)
- [ ] fil (catégories, repli mémorisé dans `Settings`, clic, dump, tests)
- [ ] vérifs finales (fmt, clippy, tests, smoke)

## Prochaine étape
Squelette.
