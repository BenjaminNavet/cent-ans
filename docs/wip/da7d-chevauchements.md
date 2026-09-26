# DA7d — Chevauchements des marqueurs de ville (régions denses)

Lot DA7d de `docs/wip/da-direction-artistique.md` (vague DA7). ADR 0066, section « DA7d ».
Branche `feat/da7d-marker-overlap` (worktree agent). 0 $.

## État : terminé, non fusionné

- `game/scripts/map/marker_declutter.gd` : placement glouton par priorité, grille spatiale.
- `SettlementLayer.declutter()` : marqueurs + noms dans l'ordre (épinglés, puis rang, poids,
  données) ; marqueur cédé → fondu shader (`COLOR.b`/`COLOR.a`, horloge `declutter_now` pendant
  le fondu seulement) et nom masqué ; recalcul seulement sur mouvement net de caméra
  (`_camera_moved`), changement de palier ou d'épinglés. Épinglés : sélection (visible à toute
  distance), survol, capitale du joueur. `screen_occupancy()` pour les mesures.
- `settlement_icon.gdshader` : uniformes `declutter_now`, `declutter_fade`, `hidden_alpha`.
- Données : bloc `declutter` de `data/map/settlement_markers.json` + schéma.
- Noms relevés (`LABEL_LIFT` 0,55) pour ne plus toucher leur pictogramme.
- Test `game/tests/da7d_overlap_test.gd` (`-- --measure` : tableau sans échec).

## Mesures (1600×900, headless, 3 régions × 1100 / 600 / 330 / 200)
| | Avant | Après |
|---|---|---|
| Paires qui se recouvrent (marqueurs + noms) | 664 (hors paire marqueur / son nom) | 0 (toutes paires) |
| Pire vue | Île-de-France 330 : 138 | 0 |
| Coût d'un recalcul | ~3,3 ms (noms seuls, toutes les 0,15 s) | ~1,2-1,5 ms (sur mouvement seulement) |

Captures `docs/img/da7d/{flandre_1100,flandre_600,flandre_330,ile_de_france_600,
ile_de_france_330,ile_de_france_200,normandie_330,normandie_200}_{avant,apres}.jpg`
(`campaign_map.tscn -- --stage=map --focus=x,y,d --hide-armies --no-pyramid`).

## Tests passés
smoke, da3_markers_test, ux1_test, c5_settlements_ui_test, da7d_overlap_test, pytest tools
(762 passés). Pas de Rust touché.

## Suites possibles
- Noms de provinces (`CityMarkers`) dans le même placement.
- Soustraire la barre supérieure de l'écran utile.
- Regroupement (« +N ») au lieu de masquer, si le joueur le demande.
