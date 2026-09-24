# Lot C7b : rendu des colonies (arbres, routes, chemins, panneaux)

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 6. Suit C5 (`docs/wip/c5-settlements-ui.md`)
et C6 (`docs/wip/c6-zoom-tiers.md`). Branche : `worktree-agent-aa38756fc56f579e1` (partie de `main` `ff21e60`).
Périmètre : rendu Godot (et pipeline géo si besoin) ; C7a touche `core/` en parallèle.

## État : terminé (à fusionner par l'orchestrateur)

- [x] 1. Arbres recalés sur le relief fin (`Vegetation`, signal `chunk_surface_changed`, `VegetationGroundJob`) ; test dans `settlements_render_test` (écart 0,00 contre 0,28 avant)
- [x] 2. Routes principales lisibles au palier moyen (`RoadRenderer`, `shaders/road_line.gdshader`) ; capture `c7b-apres-routes.png`
- [x] 3. Aperçu de chemin d'armée le long des routes réelles (`geo/edge_paths.py` → `data/map/settlement_edge_paths.json`, `SettlementData.edge_path`, `SettlementController._draw_path`) ; tests pytest + `settlements_render_test` ; `docs/geo.md`
- [x] 4. Panneaux de colonie / province sans recouvrir la minicarte (`MapUI.dock_right_panel`, `_dock_panel`) ; captures `c7b-apres-panneau*.png`
- [x] 5. Captures avant / après `docs/img/colonies/c7b-{avant,apres}-{arbres,routes,chemin,panneau}.png` (+ `c7b-apres-panneau-province.png`) ; mesures A/B `--fps-probe` dans `docs/godot-map.md`

## Décisions

- Arbres : semés directement sur la grille du maillage affiché (`TerrainBuilder.surface_grid`) et
  recalés dans une tâche `WorkerThreadPool` à chaque changement de niveau d'une tuile (fin, proche,
  lointain), pas seulement pour le relief fin. Tampons CPU gardés par tuile (64 o par instance).
  Tuile hors champ : recalée quand elle y revient. Coût mesuré ≈ 6 ms de fil par tuile.
- Les candidats d'arbres qui débordaient de leur tuile (dernière colonne de la grille, gigue des
  haies) sont écartés ou ramenés dans la tuile : leur pied était posé sur le maillage d'une autre
  tuile (écart 0,28 mesuré) et la bande de bord était deux fois plus dense.

- Routes au palier moyen : nouveau shader `road_line.gdshader` (largeur écran 4,2 px à 150 → 2,2 px
  à 620, liseré sombre + cœur ocre clair, opacité 70 % au loin). Secondaires et calculées : trait
  fin brun pâle (1,5 → 0,9 px) effacé au-delà de 420, pour que les régions sans Itiner-e aient des
  routes sans surcharger. Ordre : secondaires (priorité 0) < principales (1) < fleuves (2) < icônes.
- Profondeur des routes : test souple contre la texture de profondeur (masquées seulement par un
  objet plus proche de 2,5 unités + 0,6 % de la distance : crêtes, maquettes, figurines d'armée),
  pas par le relief sous la route ni par les arbres.

- Tracé routier précalculé dans le pipeline géo, fichier voisin `settlement_edge_paths.json` (le
  format de `settlement_graph.json` reste celui du chargeur Rust, que C7a modifie en parallèle).
  533 arêtes sur 633 tracées ; les autres gardent le segment droit. Pas de schéma : comme les autres
  sorties dérivées de `data/map/`, le format est vérifié par `tools/tests/test_settlement_graph.py`.

- Panneaux : province et colonie ancrés à gauche de la minicarte (bord droit = minicarte − 8 px),
  de la barre jusqu'au-dessus de la cloche ; la minicarte reste visible avec eux (elle n'est plus
  masquée que par les panneaux de faction et de personnage), les lettres restent masquées. Le
  panneau de colonie est enregistré par `SettlementController` via `MapUI.dock_right_panel`.
- Centrage sur une colonie (onglet Colonies, captures) : le point visé est décalé pour que la
  colonie tombe au milieu de la zone libre à gauche du panneau (`SettlementController._panel_shift`).

## Points ouverts

- `docs/wip/colonies.md` (fichier de l'orchestrateur) n'est pas mis à jour : ligne C7b à cocher.
- Recalage des arbres : tampons CPU gardés (≈ 1 Mo par tuile, 64 tuiles au plus).
- Routes au palier moyen sans test de profondeur matériel (test souple) : une route derrière une
  crête de moins de 2,5 unités + 0,6 % de la distance reste visible.
- 100 arêtes `road` sur 633 gardent un segment droit dans l'aperçu (colonie loin d'une route,
  détour > 1,6 ×).
- Capture « province » : `_stage_screenshot_province` (`campaign_map.gd`, autre session) centre la
  caméra sur la capitale, désormais sous le panneau ; non modifié (fichier partagé).

## Prochaine étape

Rien : fusion par l'orchestrateur.
