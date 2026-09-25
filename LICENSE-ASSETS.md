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

Les portraits, miniatures de chronique, illustrations de l'encyclopédie et du codex
(`game/assets/portraits/`, `game/assets/events/`, `game/assets/illustrations/`) ont été générés par
un modèle d'images, et les voix (`game/assets/audio/voice/`) par synthèse vocale (voir `CREDITS.md`).
Ils sont distribués aux mêmes conditions (CC BY-SA 4.0) dans la mesure où un droit d'auteur s'y
applique. Les voix sont synthétiques et ne reproduisent aucune personne réelle.
