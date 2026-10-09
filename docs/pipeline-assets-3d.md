# Procédé de création des assets 3D (image → 3D)

Adopté par le joueur le 08/10 (ADR 0210, essais dans `docs/wip/i3d-local.md`). À suivre pour tout
nouvel asset 3D généré : figurine, décor, accessoire, végétation. Gratuit d'abord ; le payant n'est
qu'un dernier recours, avec une ligne dans `docs/budget.md`.

## 1. Vue d'ensemble

| Asset | Image (2D) | 3D |
|---|---|---|
| **Figurines** (soldats, cavaliers, personnages) | **Qwen-Image-Edit-2511** local, avec une figurine validée en image de référence | **TRELLIS** gratuit (Space HF), puis **SF3D** local en repli |
| **Décor, accessoires, végétation** | **Z-Image Turbo** local (texte seul) | idem |
| **Cas difficiles** (vues impossibles à obtenir en local) | Nano Banana 2 via OpenRouter, un seul appel payant | idem |

**Meilleur-de-N, modèles gratuits seulement** : on génère plusieurs images (graines différentes)
et on garde la meilleure, puis plusieurs modèles 3D par image retenue et on garde le meilleur.
Jamais de meilleur-de-N sur un modèle payant (fal, NB2) : un seul appel.

## 2. Respecter la charte à l'étape image

Aucun modèle ne « connaît » la charte : elle passe par le **prompt** et, pour Qwen et NB2, par des
**images de référence**. Z-Image ne lit que le texte. On vérifie l'image **avant** la 3D ; une
image hors charte ne part pas en 3D.

Références (liens rassemblés hors dépôt dans `~/dev/cent-ans-raw/charte/`) :
- Bible DA `docs/design/2026-09-25-bible-da.md` : § 3.1 teintes héraldiques, § 3.3 palette (S ≤ 0,40
  hors livrées, contrôlée à l'étape image), § 6 réalisme peint, **§ 14 assets générés** (acceptation
  image et glb, budgets triangles, repère, nommage, prompt figurines, ADR 0211).
- Matières : `data/art/materials.yaml` (étoffes, mailles, plates, cuir, bois des figurines),
  `data/art/building_materials.json` (bâti, fer), `ground_materials.yaml`, `water_materials.yaml`.
- Décor : `style_prefix` / `style_suffix` de `data/art/ga3_decor.json` (révisés, ADR 0211) ; prompt figurines : bible § 14.8.
- Image de référence d'une figurine : un rendu réaliste validé (planche SR3, figurines GA3
  `docs/img/ga3/`). **Jamais l'image d'ancrage** `data/art/style/anchor.jpg` : elle ne sert qu'au
  registre 2D (bible § 13).

Règles propres aux figurines :
- **Bras levés presque à l'horizontale, comme le Christ de Rio**, paumes ouvertes et **vides**, rien
  en main ni devant le corps ; cavaliers compris (mains hors des rênes). Les armes s'attachent à
  part en 3D.
- Livrée en **vert pur saturé**, chausses en **bleu pur saturé** : canaux de recoloration (§ 6).
- Fond blanc uni (figurines) ou gris moyen uni (décor), objet entier, centré, plein cadre.

## 3. Étape image

### Z-Image Turbo (décor, accessoires, végétation)
Local et gratuit (mflux, ADR 0190), environ 1-2 min par image 1024², 6 min pour une planche de
3 vues. Commande type :

```sh
mflux-generate-z-image-turbo --model ~/models/mflux/z-image-turbo-q8 --base-model z-image-turbo \
  --prompt-file PROMPT.txt --steps 9 --seed 1337 --width 1024 --height 1024 --output out.png
```

Voie outillée : `tools/experiments/ga3_fal_decor.py --catalog data/art/ga3_decor.json
--image-backend local --cut-backend local` (prompt = style_prefix + objet + style_suffix).
Limite connue : Z-Image tient mal plusieurs angles de vue d'un objet non humain (cheval, ville).

### Qwen-Image-Edit-2511 (figurines, retouches, vues manquantes)
Local et gratuit (Apache 2.0, 20 B), quantifié q6 dans `~/models/mflux/qwen-image-edit-2511-q6`
(≈ 30 Go). Très fidèle en retouche : change la pose sans toucher au reste (essai archer du 08/10).
**Lent** : 25-35 min par image 1024² en 25 étapes quand la RAM est partagée (machine 48 Go),
à lancer par lots, de préférence la nuit, machine libre (pas de compilation Rust, Docker arrêté).

```sh
mflux-generate-qwen-edit --model ~/models/mflux/qwen-image-edit-2511-q6 --base-model qwen-image-edit \
  --image-paths SOURCE.png [REFERENCE.png] --prompt-file PROMPT.txt \
  --steps 25 --guidance 4 --seed 1337 --width 1024 --height 1024 --output out.png
```

Vues manquantes (profil, dos) : LoRA `fal/Qwen-Image-Edit-2511-Multiple-Angles-LoRA` dans
`~/models/mflux/loras/`, ajouter `--lora-paths <fichier> --lora-scales 1.0` et le prompt
`<sks> [azimut] [élévation] [distance]`, par exemple `<sks> back view eye-level shot wide shot`
(azimuts : front view, front-right quarter view, right side view, back view, left side view… ;
élévations : eye-level shot, elevated shot, high-angle shot ; distances : close-up, medium shot,
wide shot). Non encore éprouvé dans le projet.

## 4. Détourage et cadrage
`rembg` (isnet) en local, PNG transparent, objet ≈ 90 % d'un carré **1024 × 1024**. C'est l'entrée
idéale des deux modèles 3D (vérifié dans leur code) : TRELLIS recadre la boîte alpha × 1,2 puis
518²; SF3D garde 85 % du cadre puis 512². Voie outillée : `ga3_local.cut_local`
(`tools/experiments/ga3_local.py`).

## 5. Étape 3D

### TRELLIS gratuit (choix 1)
Space Hugging Face `trellis-community/TRELLIS`, après `hf auth login` :

```sh
uv run --with gradio_client python tools/experiments/trellis_hf.py OUT_DIR NAME CUT.png \
  [VUE2.png VUE3.png] --seeds 1337,1338,1339
```

- Une glb par graine (`NAME__s<graine>.glb`, ≈ 25-35 s chacune), on garde la meilleure.
- Plusieurs images = mode multivue : seulement si les vues sont **vraiment différentes**
  (face/profil/dos d'une figurine) ; des vues presque identiques font moins bien qu'une vue seule.
- **Quota ZeroGPU** : ≈ 3,5 min de GPU par jour en compte gratuit, et chaque appel en réserve 120 s.
  Concrètement 1 à 3 générations par jour : le meilleur-de-N y est très limité. HF PRO (9 $/mois,
  25 min/jour) le lèverait ; à décider par le joueur et à consigner dans `docs/budget.md`.

### SF3D local (choix 2, repli)
Stable Fast 3D dans `~/dev/cent-ans-raw/sf3d/stable-fast-3d` (venv `.venv`, MPS, ≈ 1 min/vue,
sans quota). **Toujours depuis une vue de face ou ¾ avant** : bon de face, mauvais depuis un profil
ou un dos (texture noire, déformations). Sortie tournée de 180° en lacet.

```sh
cd ~/dev/cent-ans-raw/sf3d/stable-fast-3d && ../.venv/bin/python run.py CUT.png --output-dir OUT
```

Meilleur-de-N : relancer avec d'autres découpes ou images sources (le modèle est déterministe pour
une même entrée). Piège d'installation (libomp) : voir `docs/wip/i3d-local.md`.

### Payant (dernier recours)
TRELLIS sur fal (0,02 $, `fal-ai/trellis`, `fal-ai/trellis/multi`) : un seul appel, ligne dans
`docs/budget.md`. Pas de TRELLIS 2 ni de Meshy (trop chers).

### Objets orientés : au moins deux vues (règle du joueur, nuit DN 08/10)
Depuis une seule vue de face, TRELLIS (et SF3D) **invente le dos** : sur `city_west` il l'a rendu noir.
Tout objet qui a un devant et un derrière passe donc en multivue : face + dos ¾ (+ profil pour les
objets longs).

- **Classes concernées** (`MULTI_VIEW_CLASSES` de `dn_batch.py`) : `house`, `major_building`, `bridge`,
  `ship`, `cart`, `siege_engine`, `figure`, `figure_mounted` ; profil en plus pour `cart`,
  `siege_engine`, `bridge`. Arbres, rochers, plantes, animaux : vue unique suffisante.
- **Pourquoi un modèle d'édition** : Z-Image Turbo part d'un texte ; lui demander « le même objet vu de
  dos » donne un *autre* objet. Il faut un modèle qui reçoit l'image de face et redessine le même objet
  sous un autre angle (prompts `VIEW_PROMPTS` : mêmes matières, proportions, lumière, fond gris).
- **Voie gratuite (choix 1)** : Qwen-Image-Edit-2511 local + LoRA Multiple-Angles (§ 3), ≈ plusieurs
  minutes par vue sur le M4 Pro ; puis TRELLIS HF multivue (quota) ou, à défaut, SF3D depuis la face.
- **Voie payante (nuit DN)** : `fal-ai/flux-2/edit` (0,024 $ par vue) → rembg → `fal-ai/trellis/multi`
  (0,02 $, `multiimage_algo: stochastic`). Bilan de la nuit : 389 vues = **9,34 $, premier poste**
  (sur 28,58 $), plus cher que l'image de face et la 3D réunies (0,025 $). Un bâtiment multivue coûte
  ≈ 0,05-0,07 $ contre 0,025 $ en vue unique.
- **La classe d'ingest décide seule** : vérifier la classe avant un lot. Les 52 vues `env_*` de la nuit
  étaient justifiées (barges, bateaux, chapelles, hospices, chariots : classes `house`, `major_building`,
  `ship`, `cart`) ; seuls quelques objets symétriques ou plaqués (pigeonnier rond, front de carrière)
  auraient pu rester en vue unique — les classer `prop` ou `rock` pour éviter la dépense.
- Les vues générées sont gardées dans `dn/<id>/views/` et listées dans `generation.json` (galerie).

## 6. Contrôle et rangement
- Planche de comparaison : `tools/blender_scripts/ga3_compare_render.py` (`--ref`, `--yaw`,
  `--fit-width` pour cheval et décor).
- Sorties brutes hors dépôt dans `~/dev/cent-ans-raw/` ; galerie
  `~/dev/cent-ans-raw/galerie-3d/` (`<objet>__<modèle image>__<modèle 3D>.glb`, régénérer avec
  `python3 -I build.py`).
- Dans le jeu : budgets de triangles et LOD de la bible § 6, procédure d'asset § 11.

### Ingestion d'un glb brut : `cent-ans dn-ingest` (lot DN)
Un seul outil transforme un glb brut TRELLIS/SF3D en asset de jeu (bible § 14.4-14.6) :
```
uv run --project tools cent-ans dn-ingest <brut.glb> --id <snake_id> --class <classe> \
    (--length|--width|--height <m>) [--yaw 180] [--lods 3] [--tex 1024] [--gamma auto] [--no-grade] \
    [--source-image ...] [--model-3d trellis-fal] [--cost-usd 0.02]
```
- Classes et budgets lus dans `data/art/dn_ingest_classes.json` (prop, house, major_building,
  siege_engine, cart, ship, tree, rock : triangles par LOD, texture, axe d'échelle, dossier). Le
  plafond de saturation (S 0,40) et la fenêtre d'albédo moyen (0,18-0,35) y sont aussi.
- Étapes (`tools/blender_scripts/dn_ingest.py`, Blender headless) : fusion des maillages, lacet
  `--yaw` (SF3D sort tourné de 180°), échelle uniforme à la taille cible, pivot au sol au centre,
  +Y haut, texture réduite à la taille de la classe, étalonnage de l'albédo
  (`dn_grade.py` : plafond de S, gamma automatique vers la fenêtre de luminance, `--gamma` pour forcer), nettoyage du brut (îlots < 1 % des sommets calculés par position, donc sans confondre une coupe d'UV avec un débris ; socle parasite supprimé quand la base est > 8 % plus large que le corps, `strip_base` de la classe), rugosité constante par classe et métallique 0, décimation quadrique LOD0/1/2.
- Sorties : `game/assets/models/dn/<categorie>/<id>_lod{0,1,2}.glb` (un glb par LOD, texture JPEG
  embarquée) et entrée dans `data/art/dn_manifest.json` (classe, chemins, dimensions, triangles par
  LOD, texture, brut, image source, modèle 3D, coût, statistiques d'étalonnage). Code de sortie 2 si
  un LOD dépasse son budget ou si S p95 > plafond. Aucune génération : l'outil ne touche pas au GPU.
- **Les glb ne sont plus commités** (ADR 0212) : ils voyagent en paquet de release « Cent Ans modèles »,
  comme le relief. Après une ingestion, commiter seulement `data/art/dn_manifest.json`, puis
  `uv run --project tools cent-ans art models-update` (empaquette ; publie la Release `models-v<N>` avec
  l'accord du joueur, `--no-publish` pour s'arrêter avant) et commiter `data/art/dn_models_hosting.json`.
  Installer le paquet sur un autre poste : `cent-ans art models-fetch` (fait par `tools/launch.sh`,
  `--no-models` pour passer). Sans paquet le jeu tourne avec les maquettes de repli.
- Le contour des LOD dérive d'environ 1 % de la hauteur (mesuré sur une maison TRELLIS), sous la
  limite de 3 % de la bible. Ne pas souder les sommets avant la décimation (les LOD bloquent).
- Lacet SF3D : `--yaw 180` tourne bien le maillage (vérifié sur les sommets) ; le sens « avant »
  des glb `sf3d/out/norm/` (déjà normalisés) semble déjà être −Z : vérifier à l'œil avant d'appliquer 180.

### Arbre glb -> imposteur de campagne (lot DN nature)
La chaîne glb -> atlas d'imposteurs avait disparu avec le nettoyage SC (`ga3_vegetation_l2.py`,
a600b88f2) ; restaurée, et complétée d'une entrée glb. Pour une essence `<id>` :
```
uv run --project tools cent-ans dn-ingest <brut.glb> --id tree_<id> --class tree --height <m>
uv run --project tools python tools/blender_scripts/ga3_vegetation_l2.py sheet <id> game/assets/models/dn/vegetation/tree_<id>_lod0.glb
# ajouter l'essence à data/art/tree_species.yaml (une rangée d'atlas par entrée, dans l'ordre)
uv run --project tools python tools/blender_scripts/ga3_vegetation_l2.py atlas
```
`sheet` rend 8 vues (Blender, albédo plat, 25 deg, `dn_tree_views.py`) en une planche
`~/dev/cent-ans-raw/ga3/l2/<id>_sheet_cut.png`, que `atlas` lit : réécrit `ga3_impostors_{albedo,
normal}.png` et compile `tree_species.json`. Le semis (MultiMesh d'imposteurs, rôles, biomes) est
piloté par ce json (`ga3_vegetation.gd`) ; `fal` (ancien chemin image -> planche) reste disponible.
Eau : `uv run --project tools python -m cent_ans_tools.water_procedural` régénère les textures de
`game/assets/textures/water/`.

### Figurines de bataille : glb → jeu

Script : `tools/blender_scripts/ga3_figures.py` (restauré après sa suppression par SC, `a600b88f2` ;
dépendances `ga3_figure_probe.py` et les `battle_fine_*`/`battle_skinned_*` du même dossier ;
prompts de référence dans `tools/experiments/ga3_fal_figure.py`). Chaîne : glb TRELLIS →
nettoyage et mise à l'échelle → épaississement 12 mm, remaillage voxel 8 mm, décimation au budget LOD0 →
UV Smart, albédo cuit Cycles (livrée vert pur et chausses bleu pur passées en niveaux de gris, alpha =
masque de teinte) → auto-pondération sur le rig `human` ou `cavalry` → LOD1/LOD2 décimés →
`CAM1` (`<figure>_lod{0,1,2}.mesh.bin`) + `<figure>_albedo.png` + entrée de `manifest.json`.

1. **Déclarer** la figurine dans `UNITS` (en tête du script) : nom de recette, `figure` remplacée
   (`infantry_N`, `archer_N`, `cavalry_N`, `standard_N`), `glb` et `reference` (chemins sous
   `~/dev/cent-ans-raw/ga3/`), `top` (hauteur du casque, m), `equipment` (armes procédurales),
   `metal_z`, `clips`, `faces` (têtes greffées, 2 variantes). Cavalier : `"mounted": True`.
2. **Cuire** (depuis la racine du dépôt, Blender 5.x) :
   `blender -b --factory-startup --python tools/blender_scripts/ga3_figures.py -- <recette>
   [--raw ~/dev/cent-ans-raw/ga3] [--glb autre.glb] [--renders]`.
   Écrit dans `game/assets/models/battle_ga3/` ; `GA3_OUT_DIR=<dossier>` redirige la sortie
   (essai à blanc, comparaison). Planches : sous-commandes `sheet` et `board` (voir l'en-tête du script).
3. **Vérifier** : la ligne `MESH` du journal (sommets, triangles) respecte les budgets de la bible § 6
   (fantassin ≤ 12 000 / 1 350 / 260, monté ≤ 17 500 / 2 100 / 550) ; `uv run --project tools pytest
   tools/tests/test_ga3_figures_manifest.py`.
4. **Ligne manifeste** : le script ajoute `figures.<figure>` (`lods`, `tris`, `variants`, `head_y`,
   `ga3_albedo`, `ga3_lum`, `unit`, `source`, `faces`) ; le nom de clé est celui de la figurine fine
   remplacée (`BattleSkinned._merge_ga3`).
5. **Lien avec l'unité** : champ `figure` de `data/unit_types/unit_<id>.json`
   (`^(infantry|archer|cavalry)_N$`) ; sans champ, repli sur la variante 0 de la famille.
6. `godot --headless --path game --import`, puis `godot --headless --path game --script res://tests/smoke.gd`.

Reproductibilité vérifiée le 2026-10-08 : `man_at_arms` (infantry_0) recuit dans un dossier
temporaire, 6 s sur Blender 5.2.2 : `.mesh.bin` des trois LOD, albédo et entrée de manifeste
identiques octet pour octet à la version commitée (12 083 sommets, 11 616 triangles de maillage LOD0).

## 7. Lot de nuit (`tools/experiments/dn_batch.py`)

Outil de lot non interactif et reprenable (banc DN du 08/10). Entrée : un catalogue JSON, liste de
`{"id", "kind": "decor"|"figure", "prompt", "seeds": N, "backend3d": "fal"|"sf3d"|"hf"|"both", "region"?}`
(`data/art/dn_catalog_*.json`). Sorties hors dépôt : `~/dev/cent-ans-raw/dn/<id>/` (`img/`, `cut_raw/`,
`cut/`, `contact.png`, `scores.json`, `chosen.json`, `3d/<fal|sf3d|hf>__s<graine>.glb`, `sheet.png`).

```sh
source <(grep '^export FAL_KEY' ~/.zshrc)
uv run --with rembg --with onnxruntime --with fal-client --with pillow --with numpy \
  python tools/experiments/dn_batch.py CATALOGUE.json --select-best [--until select] \
  [--only id,id] [--charter strict|warn|off] [--fal-workers 4]
```

- Étapes `image` (Z-Image Turbo) → `cut` (rembg, cadrage 90 % de 1024²) → `select` → `3d` → `sheet`
  (Blender headless) → `gallery` (`build.py` relie les glb `dn/` dans `galerie-3d/2-decor|1-figurines`).
  Une étape dont la sortie existe est sautée : relancer la même commande reprend.
- `--until select` s'arrête à la planche contact `dn/<id>/contact.png` (et `dn/_contact.png`, toutes les
  entrées) : l'orchestrateur choisit à l'œil en écrivant `dn/<id>/chosen.json` = `{"seed": N}`, puis relance
  sans `--until`. Sans `chosen.json` : `--select-best` (heuristique : alpha non coupé au bord, centré,
  bien rempli, plein) sinon la première graine.
- Prompt : décor = `style_prefix` + objet + `style_suffix` de `ga3_decor.json`; le champ `region` remplace le
  segment initial « Rural medieval France circa 1340, humble, » (cathédrales, navires, familles non
  françaises). Figure = prompt du catalogue tel quel (bible § 14.8), ici en texte seul, sans Qwen.
- Contrôle de charte D5 (bible § 14.2) à l'étape `select` : S moyenne ≤ 0,35, percentile 95 de S ≤ 0,40,
  V moyenne 0,25-0,60 sur le détouré hors vert/bleu purs. `strict` (défaut) : l'image refusée ne part pas en 3D
  (`dn/charter.jsonl`, mention HORS-CHARTE sur la planche); `warn` : signalée seulement.
  **Piège** : le p95 de S en HSV vaut 0,83-0,86 sur le bois/pierre chauds réalistes (moulin, chariot, 6 images
  pourtant correctes à l'œil), donc `strict` rejette tout objet en bois. Seuil à revoir (bible) ou lancer en
  `warn` et juger sur la planche; `--charter-s-p95 0.9` convient aux essais du banc.
- Verrou GPU global `~/dev/cent-ans-raw/dn/gpu.lock` (flock, attente bloquante) autour de mflux et de SF3D.
  Envelopper toute autre commande locale : `tools/gpu_lock.sh <commande...>`
  (ex. `tools/gpu_lock.sh uv run --project tools cent-ans assets ... --local`). fal n'est pas verrouillé.
- Dépenses fal : `dn/fal_spend.jsonl` (id, endpoint, graine, 0,02 $) à reporter dans `docs/budget.md`;
  durées : `dn/timings.jsonl`; erreurs (solde fal, quota HF, Blender) : `dn/failures.jsonl`, jamais fatales.
- Temps mesurés (M4 Pro 48 Go) : Z-Image 1024² 103-119 s/image; rembg 0,5 s (30 s au premier appel, modèle);
  TRELLIS fal 52-88 s par appel (envoi + génération + export, en parallèle); SF3D 28-29 s/vue (hors
  chargement ≈ +10 s); planche Blender 3,5 s. Pour 3 graines + fal + SF3D : ≈ 6 min par objet, dont 5,5 de GPU local.
- Pièges : attendre le verrou peut durer plusieurs minutes si un autre agent génère (observé : 4 et 15 min);
  TRELLIS HF : quota ZeroGPU épuisé le 08/10 (`exceeded your free ZeroGPU quota`, retour dans ~19 h), l'étape
  `hf` est optionnelle et son échec est journalisé; sorties SF3D tournées de 180° (la planche corrige);
  la planche coupe un objet plus large que 1,8 m (roue du moulin fal).
- Qualité sur les 2 objets (planches `dn/watermill/sheet.png`, `dn/ox_cart/sheet.png`) : TRELLIS fal nettement
  meilleur (tour de pierre et ardoise nettes, roue à aubes lisible, chariot propre aux 4 roues, couleurs
  sombres et sobres); SF3D : formes correctes mais texture floue, trop claire et saturée, roue du moulin
  déchiquetée, chariot bruité. SF3D reste un repli.
