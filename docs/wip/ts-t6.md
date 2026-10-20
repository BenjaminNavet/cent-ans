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
- [x] squelette
- [x] plaque : `markers/plate.rs` (statut, pastilles, dump `campaign_markers.plates`, 6 tests) ; à vérifier au dump en jeu
- [x] fil : `hud/feed.rs`, `data/ui/event_feed.json`, `Settings.event_feed_{open,folded}`, dump `event_feed`, 8 tests, preuve `hud/proof_feed.txt` (OK : plaques par statut 32/36/297/35, 15 entrées après 2 tours, repli persisté)
- [ ] vérifs finales (fmt, clippy, tests, smoke)

## Restes
- Fil non restauré depuis une sauvegarde (vide jusqu'au prochain tour) ; entrées d'autres factions sans province ni faction dans le cœur : clic sans effet.
- Rendu non jugé à l'œil (pas de capture) ; pastilles : taille/espacement à juger. Infobulles des pastilles sous les plaques (GlobalZIndex -1) à vérifier à la souris.
- Pas d'icône dédiée à la pastille (texte seul).

## Prochaine étape
Vérifs finales (fmt, clippy, tests, smoke) puis rapport.
