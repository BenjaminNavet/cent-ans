# DA7d — Chevauchements des marqueurs de ville (régions denses)

Lot DA7d de `docs/wip/da-direction-artistique.md` (vague DA7). ADR 0066, section « DA7d ».
Branche `feat/da7d-marker-overlap` (worktree agent). 0 $.

## État : terminé, main (DC3/DC4/DC6c) fusionnée dans la branche, prêt pour ff

### Fusion avec DC4 / DC6c (26/09)
- **Un seul placement** : `SettlementLayer.declutter()` de DA7d (marqueurs + noms, par priorité,
  `MarkerDeclutter`). Le masquage des noms seuls de DC4 (`LabelPlacer.SpatialGrid`) est retiré.
- Repris de DC4 : taille réelle du texte (`_label_size`, police + contour, via
  `_label_text_size` / `_label_screen_rect`, échelle écran `_label_screen_scale`), `_label_kind`,
  `_show_label` (couleurs réécrites seulement si l'opacité change), `_marker_world` (déclaré une
  fois, sert au picking et au placement).
- Gardé tel quel de DC4/DC6c : maquettes (`ABSORB_FACTOR`, `ABSORB_OVERLAP`, `MIN_FIT_SCALE`
  0,4, `SettlementFit`, hameaux hors emprise) — ce n'est pas du dé-encombrement de marqueurs.
- Picking : `marker_visible(i)` (un marqueur cédé n'est pas cliquable) au lieu du seul palier.
- Hauteur des noms : `LABEL_LIFT` remplacé par `_label_lift_px` (haut de l'emprise du
  pictogramme + demi-hauteur du texte mesuré + 2 px, à l'échelle écran), sinon le texte mesuré
  (plus haut que l'estimation) mordait son propre pictogramme.
- Après fusion (1 192 colonies) : da7d 0 paire aux 12 vues, recalcul ~2,2 ms ; sonde DC4
  (`dc4_density_probe --no-shots --no-valley`) : 0 chevauchement de noms et de marqueurs aux
  4 vues, `declutter` 1,0-2,1 ms.


- `game/scripts/map/marker_declutter.gd` : placement glouton par priorité, grille spatiale.
- `SettlementLayer.declutter()` : marqueurs + noms dans l'ordre (épinglés, puis rang, poids,
  données) ; marqueur cédé → fondu shader (`COLOR.b`/`COLOR.a`, horloge `declutter_now` pendant
  le fondu seulement) et nom masqué ; recalcul seulement sur mouvement net de caméra
  (`_camera_moved`), changement de palier ou d'épinglés. Épinglés : sélection (visible à toute
  distance), survol, capitale du joueur. `screen_occupancy()` pour les mesures.
- `settlement_icon.gdshader` : uniformes `declutter_now`, `declutter_fade`, `hidden_alpha`.
- Données : bloc `declutter` de `data/map/settlement_markers.json` + schéma.
- Noms relevés (`_label_lift_px`) pour ne plus toucher leur pictogramme.
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
Après fusion de main : smoke, da3_markers_test, ux1_test, c5_settlements_ui_test,
da7d_overlap_test, dc6c_fit_scale_test, dc4_density_probe (fenêtre, ok: true), pytest tools
(768 passés). Pas de Rust touché (dylib copiée du dépôt principal).

## Suites possibles
- Noms de provinces (`CityMarkers`) dans le même placement.
- Soustraire la barre supérieure de l'écran utile.
- Regroupement (« +N ») au lieu de masquer, si le joueur le demande.
