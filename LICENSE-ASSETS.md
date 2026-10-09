# Licence des assets et données — Cent Ans

Sauf mention contraire, les assets et données **originaux** du projet sont placés sous licence
**Creative Commons Attribution - Partage dans les mêmes conditions 4.0 International (CC BY-SA 4.0)**
— texte complet : <https://creativecommons.org/licenses/by-sa/4.0/legalcode.fr>.

Attribution demandée : « Cent Ans — Benjamin Navet et contributeurs, https://github.com/BenjaminNavet/cent-ans ».

## Ce qui est couvert

- `data/` : données de jeu rédigées pour le projet (factions, personnages, provinces, unités,
  bâtiments, technologies, événements, codex, textes), et les données cartographiques **dérivées**,
  sous réserve des licences de leurs sources (voir ci-dessous).
- `game/assets/` **hors** `game/assets/third_party/` et hors icônes de game-icons.net : modèles 3D
  générés par script, écus, sons et musiques de synthèse, textures produites par le projet.
- `docs/` : documentation, captures d'écran, manuel.

## Ce qui n'est pas couvert

- **Code source** (`core/`, `game/**/*.gd`, `game/**/*.tscn`, `game/**/*.gdshader`, `tools/`) :
  GNU GPL v3.0, voir [`LICENSE`](LICENSE).
- **Assets tiers** (`game/assets/third_party/`, icônes `game/assets/icons/`) : chacun reste sous sa
  licence d'origine (CC0, CC BY 3.0/4.0, SIL OFL 1.1…), voir [`CREDITS.md`](CREDITS.md) et les
  `SOURCE.md` / `License.txt` de chaque dossier.
- **Données géographiques** dérivées de sources tierces (ETOPO, Natural Earth, Copernicus DEM, KK10,
  Itiner-e, GeoNames, OpenStreetMap) : les obligations d'attribution de ces sources s'ajoutent à la
  présente licence, voir [`CREDITS.md`](CREDITS.md).

## Contenus générés par IA

Les assets générés par IA (modèles 3D, images, voix ; liste des modèles et de ce que chacun a
produit dans [`CREDITS.md`](CREDITS.md), § « Contenus générés par IA ») sont **couverts par la
même licence CC BY-SA 4.0**, dans la mesure où un droit d'auteur s'y applique : le droit sur un
contenu purement généré est incertain selon les pays, et le projet ne revendique que sa part
créative (prompts, sélection, retouches, assemblage). Les voix sont synthétiques et ne
reproduisent aucune personne réelle.

Conditions connues des modèles utilisés (état au 09/10/2026 ; ne pas en déduire plus que ce qui
est écrit) :

| Modèle | Condition connue |
|---|---|
| TRELLIS (Microsoft) | MIT (ADR 0140) ; l'offre hébergée fal.ai et le Space HF ont leurs propres conditions d'utilisation : à vérifier |
| Z-Image Turbo (Tongyi-MAI) | Apache 2.0, usage commercial libre (ADR 0190) |
| Qwen-Image-Edit-2511 | Apache 2.0 (`docs/pipeline-assets-3d.md`) ; licence du LoRA Multiple-Angles : à vérifier |
| Stable Fast 3D (Stability AI) | licence communautaire Stability, gratuite sous 1 M$ de revenus (`docs/wip/i3d-local.md`) ; conditions sur les sorties : à vérifier |
| TRELLIS 2 (fal) | quelques essais seulement, modèle écarté ; licence : à vérifier |
| FLUX.2 edit (Black Forest Labs) | licence dépendant de la variante hébergée (dev non commerciale, ou offre fal) : à vérifier ; FLUX.1 dev a été écarté pour cette raison (ADR 0190) |
| Bria background remove (fal) | conditions de l'offre hébergée : à vérifier |
| gpt-5-image-mini, gpt-audio-mini (OpenAI) | conditions d'utilisation d'OpenAI via OpenRouter : sorties utilisables par le client ; à vérifier avant publication commerciale |
| Gemini flash image, Nano Banana 2 (Google) | conditions d'utilisation de Google via OpenRouter / fal.ai ; marquage SynthID possible dans les images : à vérifier |
| ElevenLabs v3 | droits d'usage commercial selon l'offre souscrite, accès via fal.ai : à vérifier |

Les modèles ne sont pas redistribués avec le jeu : seuls leurs résultats le sont. Si une
condition ci-dessus s'avère incompatible avec CC BY-SA 4.0 ou avec une distribution commerciale,
l'asset concerné devra être régénéré ou retiré.
