# Poly Haven — végétation (arbres et herbe)

- **Auteurs** : Rico Cilliers (modélisation), Rob Tuytel (photographie) — Poly Haven
  (grass_medium_02 : Rico Cilliers seul)
- **Licence** : CC0 1.0 (domaine public) — https://creativecommons.org/publicdomain/zero/1.0/
- **Récupéré le** : 2026-09-24 (agent D0)

| Dossier | Page | Source téléchargée | Contenu du GLB | Triangles |
|---|---|---|---|---|
| `fir_tree_01/` | https://polyhaven.com/a/fir_tree_01 | `.blend` 1k | 3 sapins (variantes a, b, c) | 30 000 chacun (décimés depuis le LOD2 : 70 661 / 54 927 / 31 512) |
| `pine_tree_01/` | https://polyhaven.com/a/pine_tree_01 | `.blend` 1k | 3 pins (variantes a, b, c) | 30 000 chacun (décimés depuis le LOD2 : 416 451 / 267 343 / 347 991) |
| `grass_medium_01/` | https://polyhaven.com/a/grass_medium_01 | `.blend` 2k | 10 touffes (large a-c, mid a-c, small a-b, tall a-b), LOD2 d'origine | 121 à 1 032 |
| `grass_medium_02/` | https://polyhaven.com/a/grass_medium_02 | `.blend` 2k | 5 touffes (a-e) | 714 à 2 489 |

**Modifications** (script `tools/blender_scripts/polyhaven_vegetation.sh`) : seules les
variantes LOD2 sont gardées (le LOD0 d'origine dépasse 2 millions de triangles par arbre) ;
décimation « collapse » des arbres à 30 000 triangles ; matériaux Poly Haven remplacés par des
matériaux glTF simples (couleur + alpha RGBA 8 bits, carte normale 8 bits ; rugosité constante
0,9) ; aiguilles et brins d'herbe en `alphaMode MASK` double face. Textures 1k (arbres) et
1024 px (herbe, réduites depuis 2k), embarquées dans les GLB. Chaque variante est un nœud racine
à l'origine, à instancier séparément (MultiMesh pour l'herbe).
