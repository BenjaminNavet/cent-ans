# I3D — image→3D gratuit en local (essai 08/10)

But : trouver une alternative gratuite à TRELLIS fal (0,02 $/objet). Tout est hors dépôt dans
`~/dev/cent-ans-raw/sf3d/` (dépôts clonés, venvs, sorties, planches `out/sheets/`).

## État
- **TripoSR** (MIT, non gated) : marche sur Mac M4 Pro (MPS), venv `.venv-tsr`.
  - `torchmcubes` ne compile pas sur Mac → `tsr/models/isosurface.py` patché vers
    `skimage.measure.marching_cubes` (axes inversés pour imiter torchmcubes).
  - `--model-save-format glb --bake-texture` écrit en fait un **OBJ** nommé `.glb` + `texture.png` :
    conversion trimesh → `tsr.glb`. Sortie en Z vers le haut : rotation −90° autour de X.
  - ≈ 4-5 min/objet (dont cuisson de texture), 120-390 k triangles bruts (à décimer).
  - Planche `out/sheets/d_all.jpg` (TRELLIS à gauche, TripoSR à droite, `ga3_sheet.py`) :
    maison correcte de face mais floue/molle ; chariot : une roue manque, brancards OK ;
    chêne : boules molles, tronc à peine visible. Nettement en dessous de TRELLIS.
- **Stable Fast 3D** (licence Stability communautaire, gratuite < 1 M$) : installé (venv `.venv`,
  libomp brew, extensions texture_baker/uv_unwrapper compilées, MPS OK) mais **modèle gated** :
  attend `hf auth login` + licence acceptée par le joueur sur huggingface.co/stabilityai/stable-fast-3d.

## Prochaine étape
Une fois le jeton HF présent (`~/.cache/huggingface/token`) : lancer SF3D sur les 3 mêmes images
(`ga3/s4/cut.png`, `ga3/l1/cart/cut.png`, `ga3/s5/oak_tree_cut.png`) avec
`PYTORCH_ENABLE_MPS_FALLBACK=1 python run.py IMG --device mps --output-dir ../out/sf3d/<n>`,
normaliser (même script trimesh que TripoSR), ajouter une colonne à la planche.
