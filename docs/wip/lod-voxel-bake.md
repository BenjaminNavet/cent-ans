# LOD voxel_bake (ADR 0790) — reprise

État (28/10) : code commité (b0eda865b) ; essai à résolution moitié (`voxel_shell`, 200 → 100 → 50) non commité.
Passe 1 (résolution 200) : 209/321 assets refaits, contrôles visuels bons (cabane, falaise, château byz).
Passe 2 (essai grossier) : 102 refaits de plus ; contrôle : `econ_port_warehouse_stone` LOD2 raté (trou au toit,
faces noires = rayons de cuisson manqués) ; `cathedral_west` LOD1 correct.
Les glb (hors git) sont déjà remplacés ; `data/art/dn_manifest.json` modifié non commité.
Rapports : scratchpad `rebake/report.json` et `rebake2.log` (session du 28/10).
Mesure « texels noirs » inutilisable : elle compte le vide entre îlots d'UV.
Ouvert : décider pour les 102 de la passe 2 (garder, ou refaire en `--lod-method collapse`) ; distances de
cuisson à lier à la taille du voxel ; LOD0 TRELLIS non fidèles toujours en `collapse`.
