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
