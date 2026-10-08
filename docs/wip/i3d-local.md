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
- **Stable Fast 3D** (licence Stability communautaire, gratuite < 1 M$) : marche (venv `.venv`, MPS).
  - Accès HF accepté + `hf auth login` (08/10). Poids ≈ 4 Go dans le cache HF.
  - Piège : texture_baker/uv_unwrapper liés au libomp de brew + celui de torch → OMP #15 puis
    segfault. Corrigé par `install_name_tool -change` vers `<torch>/lib/libomp.dylib` (chemin absolu)
    + `codesign -f -s -`. À refaire après toute réinstallation.
  - ≈ 70 s/objet (chargement compris), 26-72 k triangles, vrai glb texturé, Y vers le haut.
  - Planche `out/sheets/compare_3.jpg` + `cart_close.jpg` (TRELLIS / TripoSR / SF3D) :
    maison : bonne silhouette mais pans de bois perdus, texture paille uniforme, pas de soubassement ;
    chariot : **le meilleur des gratuits** (2 roues, brancards, ridelles), un peu bruité ;
    chêne : masse vert clair délavée, tronc invisible. Toujours sous TRELLIS en détail et en couleur.

## Verdict
TRELLIS fal reste le meilleur rapport qualité/prix (0,02 $). SF3D local = repli gratuit honnête pour
les objets simples vus de loin (chariots, caisses, rochers) ; inutilisable pour arbres et bâtiments
détaillés. TripoSR : à abandonner.
