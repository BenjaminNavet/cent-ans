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

## Essai 2 (08/10) : archer, cavalier, ville
- Sources : archer `ga3/l3/longbowman/front.png` ; cavalier = vue ¾ recadrée de `sr3/knight_mounted.png`
  détourée en local (rembg) ; ville = image locale Z-Image Turbo (prompt `sf3d/in/town/prompt.txt`,
  ville close ronde, église, maquette) détourée en local. Coût 0 $.
- fal.ai toujours vide (403, ADR 0152). **TRELLIS gratuit via le Space HF
  `trellis-community/TRELLIS`** (gradio_client, login HF du joueur, quota ZeroGPU) : ≈ 26 s/objet,
  glb texturé 1024, 7-24 k tri. Script : `~/dev/cent-ans-raw/sf3d/trellis_hf.py` (copie du scratchpad).
- Planches `out/sheets/units.jpg` (archer + cavalier, ¾ face/dos) et `town.jpg`.
  - Archer : TRELLIS le plus fidèle (couleurs sombres) ; SF3D net mais vert fluo, casque clair ;
    TripoSR étonnamment proche de TRELLIS, un peu plus mou.
  - Cavalier : TRELLIS lisible (cheval, housse, cavalier) ; SF3D housse aux armes la plus lisible de
    face mais dos flou/miroité, cavalier fondu ; TripoSR cheval correct, cavalier brouillé.
  - Ville : TRELLIS garde tours, enceinte, église ; TripoSR enceinte + toits, sombre et bruité ;
    SF3D socle penché, tours perdues.
- Orientation : SF3D sort tourné de 180° (yaw 180), TripoSR couché + 90° (rx −90 puis yaw 90).

## Verdict mis à jour
TRELLIS gagne partout et devient **gratuit** par le Space HF (dans la limite du quota ZeroGPU
quotidien). SF3D/TripoSR locaux : replis hors ligne seulement.

## Essai 3 (08/10) : 3 vues Z-Image conformes à la charte, TRELLIS multi contre SF3D par vue
- Images : Z-Image Turbo local (mflux, ADR 0190), une planche de 3 vues par objet, seed 1337,
  prompts `~/dev/cent-ans-raw/sf3d/mv/*.txt`. Charte : figurines selon la convention GA3
  (bible §6 : bras écartés, mains vides, livrée verte et chausses bleues pures = canaux de
  recoloration ; cavalier : jupon et caparaçon unis sans blason, acier non peint) ; ville selon
  `style_prefix/suffix` de `data/art/ga3_decor.json` (tons de terre, fond gris). ≈ 6 min par planche.
  - Archer : vraie face / profil / dos. Cavalier : Z-Image donne ¾ avant, profil, ¾ avant (pas de
    vraie face ni de dos). Ville : trois ¾ avant presque identiques. Z-Image tient mal les angles
    de vue multiples pour un objet non humain.
- Découpe : `sf3d/split_views.py` (rembg isnet, colonnes séparées par l'alpha, carré 1024, objet
  ≈ 90 % du cadre). Le cavalier s'est chevauché : coupe au creux d'alpha, plus grande composante.
- **Résolutions d'entrée vérifiées dans le code** : TRELLIS `preprocess_image` réduit à ≤ 1024,
  recadre la boîte alpha × 1,2 puis 518 × 518 (chaque vue en multi) ; SF3D : rembg,
  `resize_foreground` 0,85 puis 512 × 512 (`cond_image_size`). Un carré 1024 détouré, objet plein
  cadre, convient aux deux.
- **Piège Space HF** : l'API appelle `generate_and_extract_glb` avec `preprocess_image=False` et le
  mode multi dépend de l'onglet. Script `sf3d/trellis_hf_multi.py` : `/start_session`,
  `/lambda_1` (onglet multi), `/preprocess_images`, puis génération (stochastic). 22-36 s, 0 $.
- SF3D : `run.py` sur les 9 vues d'un coup, ≈ 1 min/vue, 11-16 k tri.
- Planches `out/sheets/mv_archer.jpg`, `mv_cav.jpg`, `mv_town.jpg` (`ga3_compare_render.py`,
  nouvelle option `--fit-width` pour cheval et décor).
  - Archer : TRELLIS 3 vues cohérent tout autour (texture sombre, défaut connu) ; SF3D face net,
    couleurs vives, dos inventé correct ; SF3D profil : **texture entièrement noire** (cuisson ratée
    sur une silhouette fine) ; SF3D dos : forme bonne, visage inventé.
  - Cavalier : TRELLIS lisible mais cavalier fin et sombre ; SF3D face très bon de face (meilleur
    rendu des couleurs), flou et miroité ailleurs ; profil et dos déformés.
  - Ville : SF3D face la plus fidèle (enceinte, tours, église) ; TRELLIS multi réorganise la ville
    (vues trop semblables, pas d'information nouvelle) ; SF3D profil et dos plus mous.
- Verdict : SF3D marche bien **depuis une vue de face ou ¾ avant**, mal depuis un profil ou un dos.
  TRELLIS multi n'apporte que si les vues sont vraiment différentes (archer) ; avec des vues
  quasi identiques (ville) il fait moins bien que SF3D en vue unique. Choix inchangé : TRELLIS d'abord,
  SF3D en repli, toujours nourri d'une vue ¾ avant.

## Galerie des modèles 3D (08/10)
`~/dev/cent-ans-raw/galerie-3d/` (hors dépôt) : liens symboliques vers les 98 glb générés (GA3 S1-S5,
L1-L4, I3D essais 1-3), rangés `1-figurines/`, `2-decor/`, `3-vegetation/`, `4-comparatifs/<essai>/`,
nommés `<objet>__<modèle image>__<modèle 3D>[__variante].glb`, avec images sources et rendus.
`README.md` = légende des codes + index complet. Régénérer : `python3 -I build.py`.
Rappel : les figurines GA3 du jeu sortent déjà d'un pipeline **Nano Banana 2 edit (fal) → TRELLIS
multi (fal)** ; les planches SR3 viennent de NB2 via OpenRouter (`google/gemini-3.1-flash-image`).

## Essai 4 (en cours) : Qwen-Image-Edit-2511 pour les vues manquantes
- Modèle `Qwen/Qwen-Image-Edit-2511` (Apache 2.0, 20 B) en local via mflux
  (`mflux-generate-qwen-edit`), sauvé quantifié q6 dans `~/models/mflux/qwen-image-edit-2511-q6`.
- LoRA `fal/Qwen-Image-Edit-2511-Multiple-Angles-LoRA` (Apache 2.0) : prompt
  `<sks> [azimut] [élévation] [distance]`, ex. `<sks> back view eye-level shot medium shot`.
- But : depuis la vue ¾ avant Z-Image du cavalier, produire profil et dos cohérents, puis TRELLIS
  multi et SF3D.
- **Enjeu (joueur, 08/10)** : si Qwen est bon, le procédé standard des assets 3D devient
  Qwen-Image-Edit (image vérifiée selon la charte) → TRELLIS gratuit (Space HF) en premier → SF3D
  local en second. Pour les modèles gratuits seulement, on garde la meilleure de plusieurs images (graines), puis le meilleur de plusieurs modèles 3D par image (meilleur-de-N). Il sera adopté après l'essai et l'avis du joueur.
- **Arrêté le 08/10 (trop long : 33 min par image, RAM partagée)** après la correction de pose de
  l'archer (`~/dev/cent-ans-raw/sf3d/qwen/out/cmp_archer_1337.jpg` : bras écartés, paumes vides,
  reste identique, léger fantôme des anciens bras). Le joueur veut les bras plus haut (« Christ de
  Rio »). Procédé adopté : `docs/pipeline-assets-3d.md`, ADR 0210. Quota ZeroGPU épuisé le 08/10
  (chaque appel réserve 120 s).
