# Crédits — Cent Ans

Ce fichier recense les œuvres de tiers utilisées par le jeu et leurs licences. Les assets
produits par les outils du projet (`tools/`) sont signalés comme tels.

## Icônes — game-icons.net (CC BY 3.0)

Icônes de [game-icons.net](https://game-icons.net) (dépôt
[github.com/game-icons/icons](https://github.com/game-icons/icons)), sous licence
[Creative Commons Attribution 3.0](https://creativecommons.org/licenses/by/3.0/).
**Modifications** : fond noir retiré, recoloration monochrome encre sépia (`#4a3219`),
taille déclarée 64 px (outil `uv run --project tools cent-ans assets icons`). La
correspondance identifiant → fichier → auteur est versionnée dans
`game/assets/icons/icons.json`.

| Auteur | Icônes | Noms (game-icons.net) |
|---|---|---|
| Carl Olsen | 1 | crossbow |
| Caro Asercion | 4 | cloaked-figure-on-horseback, medieval-village-01, spinning-wheel, water-mill |
| Cathelineau | 1 | swordman |
| Delapouite | 49 | abacus, abbot-meeple, barracks-tent, barrel, bread, broom, castle-ruins, chest-armor, church, coins, diploma, double-fish, farmer, graduate-cap, hand-saw, hanging-sign, harbor-dock, healing, herbs-bundle, horse-head, hospital, knight-banner, medicines, medieval-barracks, medieval-pavilion, military-fort, palisade, pikeman, plague-doctor-profile, plow, powder-bag, rolled-cloth, scroll-quill, shaking-hands, shop, siege-tower, stable, stone-pile, stone-wall, sword-brandish, throne-king, trebuchet, two-coins, village, well, windmill, wood-pile, wooden-crate, wool |
| HeavenlyDog | 2 | catapult, defensive-wall |
| Lorc | 55 | anvil, archery-target, armor-vest, arrows-shield, bandage-roll, boot-prints, bowman, breastplate, cannon, cannon-shot, castle, crossed-swords, crown, crown-coin, drama-masks, drop, falling-leaf, fist, flying-flag, galleon, gears, gothic-cross, grapes, gunshot, halberd, hospital-cross, hourglass, laurels, leeching-worm, metal-bar, muscle-up, open-book, papers, potion-ball, powder, prayer, quill-ink, round-bottom-flask, scales, scalpel, scalpel-strike, scroll-unfurled, sleepy, snowflake-2, spears, sprout, stone-block, stone-tower, sun, swap-bag, target-arrows, tied-scroll, visored-helm, wax-seal, wheat |
| Skoll | 4 | mounted-knight, musket, open-treasure-chest, siege-ram |

## Données géographiques

- **Relief (terre et bathymétrie)** : ETOPO 2022 15 Arc-Second Global Relief Model, NOAA
  National Centers for Environmental Information — domaine public (données du gouvernement des
  États-Unis). Citation : *NOAA National Centers for Environmental Information. 2022: ETOPO
  2022 15 Arc-Second Global Relief Model. doi:10.25921/fd45-gt74*.
- **Terres, côtes, rivières, lacs** : [Natural Earth](https://www.naturalearthdata.com/)
  (10 m physical) — domaine public. « Made with Natural Earth. »
- Traitement (reprojection EPSG:3035, découpage des provinces) : outils `tools/cent_ans_tools/geo`
  (voir `docs/geo.md`).

## Données historiques

Personnages, provinces, factions, technologies, unités et bâtiments de `data/` sont rédigés
pour le projet à partir de sources publiques ; chaque fichier liste ses sources dans le champ
`sources` (titres d'articles de Wikipédia en français ou références d'ouvrages). Les textes de
Wikipédia ne sont pas recopiés.

## Assets produits par le projet

- **Écus** (`game/assets/heraldry/`) : dessinés procéduralement (Pillow) à partir des blasons
  de `data/factions/`.
- **Modèles 3D** (`game/assets/models/`) : générés par scripts Blender (`tools/blender_scripts/`).
- **Sons et musiques** (`game/assets/audio/`) : synthèse procédurale (numpy/scipy), sans
  échantillon externe.
- **Portraits** (`game/assets/portraits/`) : images générées par IA via OpenRouter
  (`openai/gpt-5-image-mini`), dépenses consignées dans `docs/budget.md`.

## Polices

- Interface parchemin : polices serif du système (Georgia, Palatino, Times New Roman), non
  redistribuées avec le jeu.
- Étiquettes de carte et textes sans thème : police par défaut intégrée à Godot (Open Sans,
  SIL Open Font License 1.1).

## Logiciels

- [Godot Engine](https://godotengine.org) 4.7 — licence MIT.
- [godot-rust (gdext)](https://github.com/godot-rust/gdext) — licence MPL 2.0.
- Bibliothèques Rust et Python : voir `core/Cargo.lock` et `tools/uv.lock` (licences
  permissives MIT / Apache 2.0 / BSD).
