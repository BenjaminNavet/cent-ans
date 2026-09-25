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
| Delapouite | 60 | abacus, abbot-meeple, bacon, barracks-tent, barrel, bread, broom, castle-ruins, chest-armor, church, coins, diploma, double-fish, farmer, fish-smoking, flag-objective, furnace, graduate-cap, hand-saw, hanging-sign, harbor-dock, healing, herbs-bundle, horse-head, hospital, knight-banner, medicines, medieval-barracks, medieval-pavilion, military-fort, money-stack, palisade, peas, pikeman, plague-doctor-profile, plow, powder-bag, public-speaker, receive-money, rolled-cloth, scroll-quill, shaking-hands, shop, siege-tower, spy, stable, stone-pile, stone-wall, sword-brandish, throne-king, torch, trebuchet, trowel, two-coins, village, well, windmill, wood-pile, wooden-crate, wool |
| Faithtoken | 1 | ore |
| HeavenlyDog | 2 | catapult, defensive-wall |
| Lorc | 84 | angel-wings, anvil, archery-target, armor-vest, arrow-cluster, arrows-shield, bandage-roll, battle-axe, battle-gear, boot-prints, bowman, breastplate, broadsword, cannon, cannon-shot, cash, castle, cauldron, checked-shield, cheese-wedge, cloak-dagger, crested-helmet, crossed-axes, crossed-swords, crown, crown-coin, dove, drama-masks, drop, falling-leaf, fist, flying-flag, galleon, gears, gothic-cross, grapes, gunshot, halberd, hammer-nails, high-shot, holy-symbol, horse-head, hospital-cross, hot-spices, hourglass, laurels, leeching-worm, linked-rings, lyre, metal-bar, muscle-up, open-book, papers, plain-dagger, pocket-bow, potion-ball, powder, prayer, quill-ink, rally-the-troops, round-bottom-flask, scales, scalpel, scalpel-strike, scroll-unfurled, sleepy, snowflake-2, spears, spiked-fence, spiked-mace, sprout, stone-block, stone-spear, stone-tower, sun, swap-bag, target-arrows, thrown-spear, tied-scroll, visored-helm, wax-seal, wheat, wine-glass, wing-cloak |
| Skoll | 4 | mounted-knight, musket, open-treasure-chest, siege-ram |

## Assets tiers (`game/assets/third_party/`)

Chaque dossier contient un `SOURCE.md` (URL, licence, auteur, modifications). Les assets CC0
n'exigent aucune attribution ; ils sont crédités par courtoisie.

### Musique — CC BY 4.0 (attribution obligatoire)

- « Lord of the Land » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/
- « Village Consort » Kevin MacLeod (incompetech.com). Licensed under Creative Commons: By
  Attribution 4.0 License — http://creativecommons.org/licenses/by/4.0/

### Polices — SIL Open Font License 1.1

- **EB Garamond** — Georg Duffner, Octavio Pardo (The EB Garamond Project Authors),
  [fonts.google.com/specimen/EB+Garamond](https://fonts.google.com/specimen/EB+Garamond).
- **IM FELL English** — Igino Marini ([iginomarini.com](http://www.iginomarini.com)), nom
  réservé « IM FELL ».

Le texte de la licence (`OFL.txt`) accompagne chaque police.

### Modèles 3D — CC0 1.0

- **Quaternius** ([quaternius.com](https://quaternius.com)) : personnages riggés de l'Ultimate
  Modular Characters Pack (King, Adventurer, Hooded Adventurer, Farmer), chevaux et âne de
  l'Ultimate Animated Animal Pack, Medieval Village Pack (39 bâtiments et props) — fichiers
  obtenus via [Poly Pizza](https://poly.pizza). Les figurines de bataille skinnées
  (`game/assets/models/battle_skinned/`, lot V2) en dérivent : pièces recolorées, habillées
  d'équipement procédural, décimées, animations rééchantillonnées et complétées.
- **Kenney** ([kenney.nl](https://kenney.nl)) : Castle Kit.
- **Poly Haven** ([polyhaven.com](https://polyhaven.com)) : Fir Tree 01, Pine Tree 01, Grass
  Medium 01 (Rico Cilliers, Rob Tuytel), Grass Medium 02 (Rico Cilliers). Modifiés : LOD2
  seulement, décimation, matériaux simplifiés.

### Ciels HDRI — CC0 1.0

- **Belfast Open Field** — Dimitrios Savva, Jarod Guest (Poly Haven).
- **Autumn Field Pure Sky** — Sergej Majboroda, Jarod Guest (Poly Haven).
- **Kloofendal 48d Partly Cloudy, Kloofendal Misty Morning, Kloofendal Overcast** (Pure Sky) — Greg Zaal,
  Jarod Guest (Poly Haven).
- **Overcast Soil, Snow Field** (Pure Sky) — Sergej Majboroda, Jarod Guest (Poly Haven).
- **Syferfontein 18d Clear, Kloppenheim 06** (Pure Sky) — Greg Zaal, Jarod Guest (Poly Haven).

### Interface — CC0 1.0

- **Fantasy UI Borders** — Kenney ([kenney.nl](https://kenney.nl)).
- **Parchment GUI** — zwonky ([OpenGameArt](https://opengameart.org/content/parchment-gui)).

### Sons de bataille et ambiances — Freesound, CC0 1.0

Banque AU1 (`game/assets/audio/battle/`, `game/assets/audio/ambience/`) : enregistrements
[Freesound](https://freesound.org) sous licence CC0 (vérifiée page par page), découpés, mélangés
et bouclés par `tools/cent_ans_tools/audio_bank.py`. Détail fichier par fichier (numéro, titre,
lien, traitement) dans `game/assets/audio/SOURCE.md`. Merci aux auteurs : 6polnic, adharca,
AlanCat, Ali_6868, Archeos, AyaDrevis, bajko, Blankened, bolkmar, bruno.auzet, Christopherderp,
craigsmith, DeadVDI, Defelozedd94, DeVern, DigestContent, DigPro120, DRFX, ethanchase7744,
FillMat, florianreichelt, foxen10, freefire66, greyfeather, Ittaisha, iwanPlays, jackstraton,
jamesdrake89, JoeDinesSound, JohnBuhr, joseppujol, juryduty, kasparsj, Kinoton, Kubuzz,
loopbasedmusic, Lucas_Schacht, modusmogulus, Mythmazter, nekoninja, omerbhatti34, pborel,
PixelsphereStudios, PorkMuncher, qubodup, Quickmusik, SamuelGremaud, saturdaysoundguy,
shadoWisp, Simonus18, spycrah, Twisted_Euphoria, unfa, waxsocks, WelvynZPorterSamples, xkeril.

Les sons d'interface (`game/assets/audio/ui/`, lot UB1) sont découpés hors ligne dans ces mêmes
fichiers et dans les effets procéduraux du projet par `tools/cent_ans_tools/ui_sounds.py` ;
détail dans `game/assets/audio/ui/SOURCE.md`.

## Données géographiques

- **Relief (terre et bathymétrie)** : ETOPO 2022 15 Arc-Second Global Relief Model, NOAA
  National Centers for Environmental Information — domaine public (données du gouvernement des
  États-Unis). Citation : *NOAA National Centers for Environmental Information. 2022: ETOPO
  2022 15 Arc-Second Global Relief Model. doi:10.25921/fd45-gt74*.
- **Terres, côtes, rivières, lacs** : [Natural Earth](https://www.naturalearthdata.com/)
  (10 m physical) — domaine public. « Made with Natural Earth. »
- **Relief fin (terres de l'emprise jouable)** : Copernicus DEM GLO-90, © DLR e.V. 2010-2014 et
  © Airbus Defence and Space GmbH 2014-2018, fourni dans le cadre de COPERNICUS par l'Union
  européenne et l'ESA — tous droits réservés ; licence gratuite avec attribution. Tuiles lues
  sur le bucket public AWS Open Data `copernicus-dem-90m` (lot R1, ADR 0019).
- **Défrichement vers 1340** : KK10 Anthropogenic Land Cover Change — Kaplan, J. O. et
  Krumhardt, K. M. (2017), PANGAEA, doi:10.1594/PANGAEA.871369, licence CC BY 3.0 ; méthode :
  Kaplan et al. (2011), *The Holocene* 21(5), doi:10.1177/0959683610386983. Moyenne 1330-1349,
  combinée aux grandes forêts et zones humides nommées de `data/map/historical_forests.json` et
  `data/map/wetlands.json` (sources par entrée).
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
- **Sons et musiques** (`game/assets/audio/sfx/`, `game/assets/audio/music/`) : synthèse
  procédurale (numpy/scipy), sans échantillon externe. Les sons de bataille et ambiances
  (`battle/`, `ambience/`) viennent de Freesound (CC0, voir plus haut).
- **Portraits** (`game/assets/portraits/`) : images générées par IA via OpenRouter
  (`openai/gpt-5-image-mini`), dépenses consignées dans `docs/budget.md`.
- **Voix** (`game/assets/audio/voice/` : répliques des unités, discours des généraux,
  conseiller) : voix **générées par synthèse vocale** (OpenAI `gpt-4o-mini-tts`), à partir des
  textes du projet (`data/voice/`, `data/speeches/`) ; aucune voix d'acteur. Outil
  reproductible `tools/cent_ans_tools/voice_tts.py` (liste des fichiers, voix et coût dans
  `game/assets/audio/voice/manifest.json`), dépenses consignées dans `docs/budget.md`.

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
