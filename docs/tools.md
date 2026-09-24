# Outils Python (`tools/`)

Projet `cent-ans-tools`, géré avec `uv` (Python 3.12+). Toutes les commandes se lancent
depuis la racine du dépôt avec `uv run --project tools ...`.

## Installation et qualité

```sh
cd tools
uv sync                    # crée .venv et installe les dépendances
uvx ruff check --fix .     # lint
uvx ruff format .          # formatage
uv run pytest              # tests
```

## Ligne de commande `cent-ans`

| Commande | Rôle |
|---|---|
| `uv run --project tools cent-ans budget show` | Affiche le tableau de `docs/budget.md` et le cumul |
| `uv run --project tools cent-ans budget check <montant>` | Vérifie qu'une dépense estimée tient sous le plafond (code de sortie 1 sinon) |
| `uv run --project tools cent-ans models list` | Liste les modèles OpenRouter capables de générer des images, avec tarifs (appel gratuit) |
| `uv run --project tools cent-ans assets heraldry` | Écu PNG 128×128 par faction dans `game/assets/heraldry/` (gratuit) |
| `uv run --project tools cent-ans assets audio [--no-music]` | 10 effets (`sfx/`) et 3 musiques modales (`music/`) dans `game/assets/audio/`, OGG si ffmpeg sait encoder Vorbis, sinon WAV (gratuit, ~40 s) |
| `uv run --project tools cent-ans assets models` | Modèles low-poly glTF `game/assets/models/*.glb` via Blender headless, tableau des triangles (< 3 000 exigé) |
| `uv run --project tools cent-ans assets portraits [--limit N] [--dry-run] [--model M] [--envelope 10]` | Portraits 256×256 des personnages historiques manquants (payant, sauf `--dry-run` : prompts + estimation sans réseau) |
| `uv run --project tools cent-ans assets icons [--offline]` | Icônes game-icons.net (CC BY 3.0) teintées encre sépia dans `game/assets/icons/` + `icons.json` (gratuit ; `--offline` : cache local seulement) |
| `uv run --project tools cent-ans blender smoke` | Lance Blender en arrière-plan, crée un cube, l'exporte en glTF temporaire et attend `OK` |

## Modules

### `cent_ans_tools.budget`

Registre au-dessus de `docs/budget.md` (tableau markdown, montants au format `12,34 $`,
arithmétique en `Decimal` arrondie au centime, demi-supérieur).

- `BudgetLedger(path)` : `entries`, `total()`, `check(estimation)`, `add_entry(date, service, objet, estimé, réel)`.
- Raccourcis de module : `budget.total()`, `budget.check(x)`, `budget.add_entry(...)`.
- `add_entry` supprime la ligne de substitution « Aucune dépense pour l'instant », recalcule la
  colonne *Cumul* à partir des coûts réels et réécrit le fichier en conservant l'en-tête.
- `check(x)` renvoie vrai si `cumul + x <= 50,00 $`.
- `BudgetExceeded` : exception levée par les clients payants quand la vérification échoue.

### `cent_ans_tools.openrouter`

Client OpenRouter pour la génération d'images. La clé est lue dans la variable
d'environnement `OPENROUTER_API_KEY` (jamais dans le dépôt).

- `list_image_models()` : `GET /api/v1/models`, filtre `architecture.output_modalities`
  contenant `"image"`, renvoie `id`, `name` et tarifs par token (`prompt`, `completion`,
  `image_output`). Gratuit.
- `estimate_price(model)` : estimation du coût d'une image (voir hypothèses ci-dessous),
  arrondie au centime supérieur.
- `generate_image(model, prompt, out_path, subject=...)` : appel payant
  `POST /chat/completions` avec `modalities: ["image", "text"]`. **Avant** l'appel, la fonction
  appelle `budget.check(estimation)` et lève `BudgetExceeded` si le plafond serait dépassé.
  **Après** l'appel, elle consigne une ligne dans `docs/budget.md` (coût réel renvoyé par
  `usage.cost` si présent, sinon l'estimation) puis écrit l'image base64 dans `out_path`.

Hypothèses d'estimation : 1290 jetons d'image (facturation Google pour une image
1024×1024), 300 jetons de prompt, 100 jetons de texte. Les modèles OpenAI facturent
plutôt ~1056 jetons en qualité moyenne, l'estimation est donc légèrement prudente.

### `cent_ans_tools.blender`

- `run_blender_script(script_path, *args)` : exécute
  `/opt/homebrew/bin/blender --background --python <script> -- <args>` et renvoie la sortie
  standard ; lève `BlenderError` si le code de retour est non nul.
- Les scripts vivent dans `tools/blender_scripts/` ; `smoke.py` sert de test de fumée
  (vérifié avec Blender 5.2.2 LTS).

### `cent_ans_tools.heraldry` (M10)

- `parse_blazon(blason, primaire, secondaire)` → `Blazon(field, charge, text)` : émail du champ lu en
  tête (`D'or`, `De gueules`…, sinon `primary_color`), meubles en `secondary_color` sauf émail nommé
  (`lion de pourpre`). Mots-clés : semé (de lis), écartelé (château/lion), hermine, bandé, pals, croix,
  chaînes, clefs (+ tiare), écussons, guivre, aigle, léopards, lion, bordure (de châteaux), trescheur.
- Rendu Pillow à 4× (512²) puis réduction Lanczos, masque d'écu « heater », ombrage diagonal léger,
  contour brun. `build()` écrit `game/assets/heraldry/<id>.png`. Sortie déterministe (test par hash).

### `cent_ans_tools.audio` (M10)

- Synthèse numpy/scipy à 44,1 kHz mono : Karplus-Strong (luth), dents de scie à bande limitée avec
  vibrato (vièle), harmoniques sinusoïdales (orgue portatif), voix à formants « a » (chœur), partiels
  inharmoniques (cloche), bruit filtré (page, flèches, galop), réverbération à peignes.
- `SFX` : `ui_click`, `page_turn`, `turn_bell`, `fanfare`, `march_drum`, `sword_clash`, `arrow_volley`,
  `gallop`, `war_horn`, `choir`. `PIECES` : `campaign` (ré dorien, 80 s, luth + orgue), `war` (la
  dorien, 72 s, vièle + tambour), `court` (sol mixolydien, 76 s, luth + orgue). Mélodie : marche
  aléatoire sur les degrés du mode (graine fixe), forme A A' B A…, bourdon tonique + quinte.
- `write_clip` : WAV 16 bits puis OGG (`libvorbis` si présent, sinon l'encodeur Vorbis natif de ffmpeg,
  expérimental et stéréo uniquement — c'est le cas sur cette machine) ; repli WAV sans encodeur.

### `cent_ans_tools.blender` — modèles (M10)

- `build_models(out_dir, *noms)` lance `tools/blender_scripts/models.py` (primitives jointes en un
  maillage par modèle, matériaux plats nommés ; `Banner` est teinté par Godot), lit les lignes
  `MODEL <nom> <triangles>` et refuse un modèle ≥ 3 000 triangles.
- Relevé (Blender 5.2.2) : castle 618, town 558, village 186, cathedral 248, army 208, siege_camp 306,
  ship 116 triangles ; 232 Ko au total.

### `cent_ans_tools.portraits` (M10)

- `plan(limit)` : personnages `historical` nés avant 1400 et vivants en 1337, sans PNG existant
  (idempotence). `build_prompt` : nom (et nom local), âge en 1337 (les mineurs sont peints en jeunes
  adultes), rang et titres, maison, blason de la faction, traits, description, puis le style unique
  (enluminure gothique française du XIVe siècle, buste de trois quarts, fond or et azur, cadre de
  parchemin, sans texte).
- `generate(jobs, model, envelope=10)` : avant chaque appel, vérifie `dépensé + estimation ≤ enveloppe`
  et `budget.check` (plafond global), sinon `BudgetExceeded` ; `max_tokens = 4096` limite la réserve de
  crédit ; recadrage carré (haut privilégié) et réduction 256×256 ; **une seule ligne** de
  `docs/budget.md` par lot (même interrompu).
- `openrouter.request_image(model, prompt, client, max_tokens)` : appel payant brut (image, coût
  `usage.cost`) sans écriture de budget ; `generate_image` l'enveloppe avec vérification + consignation.
- Coût réel observé : `openai/gpt-5-image-mini` **0,0455 $ par portrait** (l'estimation au jeton de
  l'API donne 0,0113 $ ; `KNOWN_PRICES` sert de plancher à l'estimation et au `--dry-run`). Qualité
  jugée très bonne (enluminure, écu correct, fond fleurdelisé) : modèle retenu.

### `cent_ans_tools.icons` et `icons_catalog` (F2)

- `icons_catalog.ICONS` : identifiant du jeu → (`<auteur>/<nom>` du dépôt
  [game-icons/icons](https://github.com/game-icons/icons), catégorie). Identifiants : ceux de `data/`
  (`unit_*`, `bld_*`, `res_*`, `tech_*`), familles (`tech_branch_*`, `unit_category_*`,
  `building_category_*`), `class_*`, `gauge_*`, `hud_*`, `branch_*` (compétences),
  `trait_category_*`, et un repli `cat_<catégorie>` par catégorie (`FALLBACKS`). `AUTHORS` : nom crédité
  de chaque auteur. Pour ajouter une icône : une ligne dans `ICONS`, relancer la commande, compléter
  le tableau de `CREDITS.md` (un test vérifie la concordance).
- `build(out_dir, cache_dir, fetch, offline)` : télécharge chaque source une fois
  (`raw.githubusercontent.com`, cache `~/.cache/cent-ans/game-icons`), `normalize_svg` (retire le carré
  noir de fond, peint toutes les formes en `#4a3219`, `width`/`height` = 64 px, refuse DTD/entités),
  écrit `game/assets/icons/<auteur>-<nom>.svg` et son `.import` (mipmaps activés), puis `icons.json`
  (licence, couleur, replis, id → fichier/catégorie/source/auteur). Idempotent : fichiers inchangés non
  réécrits.
- `missing_icons(data_dir)` : ids de `data/` sans icône (unités, bâtiments, ressources, technologies ;
  branches des compétences, catégories des traits) — la commande échoue s'il en manque.
- `credits_rows()` : auteur → noms d'icônes (tableau de `CREDITS.md`).
- Côté Godot : autoload `IconLibrary` (`game/scripts/ui/icon_library.gd`) : `get_icon(id, category)`
  avec repli par catégorie (préfixe de l'id), `make_rect`, `decorate_button`, `bbcode` (`[img]` pour les
  infobulles `RichTooltip`).

## Modèles d'images disponibles sur OpenRouter (relevé du 2026-09-23)

Tarifs en USD par jeton, tels que renvoyés par l'API ; la dernière colonne est l'estimation
par image 1024×1024 calculée par l'outil. Les routeurs `openrouter/auto*` n'ont pas de tarif
fixe et sont à éviter (choix du modèle non maîtrisé).

| Modèle | Prompt / jeton | Sortie image / jeton | Estimation / image |
|---|---|---|---|
| google/gemini-3.1-flash-lite-image | 0,00000025 | 0,00003 | ~0,039 $ |
| google/gemini-2.5-flash-image | 0,0000003 | 0,00003 | ~0,039 $ |
| openai/gpt-5.4-image-2 | 0,000008 | 0,00003 | ~0,043 $ |
| openai/gpt-5-image-mini | 0,0000025 | 0,000008 | ~0,011 $ |
| openai/gpt-5-image | 0,00001 | 0,00004 | ~0,056 $ |
| google/gemini-3.1-flash-image | 0,0000005 | 0,00006 | ~0,078 $ |
| google/gemini-3.1-flash-image-preview | 0,0000005 | 0,00006 | ~0,078 $ |
| google/gemini-3-pro-image | 0,000002 | 0,00012 | ~0,157 $ |
| google/gemini-3-pro-image-preview | 0,000002 | 0,00012 | ~0,157 $ |
| openrouter/auto, openrouter/auto-beta | — | — | non tarifé |

Ordre de grandeur pour la v1 : 50 $ permettent environ 1 250 images avec
`gemini-2.5-flash-image` / `gemini-3.1-flash-lite-image`, ou plus de 4 000 avec
`gpt-5-image-mini` (qualité moyenne).

## Recommandation

- **Portraits et icônes (usage courant)** : `google/gemini-3.1-flash-lite-image`
  (~0,04 $/image). Même prix que `gemini-2.5-flash-image` mais génération plus récente,
  bon respect des consignes de style (enluminure, parchemin, héraldique) et cohérence
  suffisante pour des séries de portraits. `openai/gpt-5-image-mini` est trois fois moins
  cher mais plus faible sur les détails historiques et le texte héraldique ; à réserver aux
  icônes simples ou aux essais de prompt.
- **Images de référence importantes** (écran titre, concepts clés) : `google/gemini-3-pro-image`
  (~0,16 $/image), à utiliser avec parcimonie.
- Toujours passer par `generate_image` afin que chaque appel soit vérifié et consigné dans
  `docs/budget.md`.

## Export macOS (`tools/export_macos.sh`)

Produit `export/Cent Ans.app` (Apple Silicon, ≈ 190 Mo) : build release de la GDExtension
(`core/build.sh --release`), export Godot avec le préréglage « macOS » (`game/export_presets.cfg`,
modèle universel, bibliothèque Rust arm64 dans `Contents/Frameworks`), copie de `data/` dans
`Contents/Resources/data` (lu par `MapPaths` quand `OS.has_feature("template")`), signature ad hoc.

Prérequis : modèles d'export Godot 4.7.2 installés dans
`~/Library/Application Support/Godot/export_templates/4.7.2.stable/` (seul `macos.zip` est nécessaire).
Le jeu exporté ne peut pas recevoir de scène en argument : `-- --autostart[=fac_x]` lance directement une
campagne (tests), suivi des options habituelles de la carte (`--screenshot=…`, `--stage=…`).

```sh
tools/export_macos.sh
open "export/Cent Ans.app"
"export/Cent Ans.app/Contents/MacOS/Cent Ans" -- --autostart --screenshot=/tmp/exported.png
```
