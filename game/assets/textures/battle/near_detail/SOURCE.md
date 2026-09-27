# Détail proche du sol de bataille (lot PO4)

- **Grass Path 2** — Rob Tuytel, **Poly Haven** (https://polyhaven.com/a/grass_path_2), licence
  **CC0** (domaine public, aucune attribution requise).
- Fichiers 2k JPG récupérés tels quels via `https://api.polyhaven.com/files/grass_path_2` :
  `grass_path_2_diff_2k.jpg` (albédo) et `grass_path_2_nor_gl_2k.jpg` (normale OpenGL).
  Échelle réelle : 1 m × 1 m par tuile.
- Usage : `battle_ground.gdshader` (`near_detail_*`), terre et herbe rase relues de près (moins de
  ~30 m de la caméra), fondues avec la distance ; seule la luminance relative module l'albédo du
  sol (la couleur reste celle des couches du sol et de la saison).
