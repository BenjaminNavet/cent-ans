# ADR 0118 — Copies GPU compressées des rasters monde de la carte de campagne

Date : 2026-09-29 (lot OMR-R2). Statut : accepté.

## Contexte
Avec l'emprise Oural–Méditerranée (ADR 0115), le relief fin `relief_shade` (LA8, 14336 × 12288,
4 bandes PNG) était décodé à chaque lancement, empilé, doté de mipmaps puis envoyé en LA8 :
470 Mo de texture et près de 1 Go de tampons transitoires, pour 2,9 Go de RSS au chargement
(contre 1,6 Go avant OM). `wetlands.png` (RGB8, 7168 × 6144, 98 % de blocs noirs) pesait 126 Mo
(176 Mo une fois complété en RGBA8 sur le GPU).

## Décision
- Les rasters monde lus seulement par le GPU ont une **copie GPU précompressée** dans `data/map/`,
  écrite par les outils (`tools/cent_ans_tools/geo/block_compress.py`, commande
  `cent-ans geo gpu-textures`, et en fin de `geo relief-shade` / `geo landcover`) :
  - `relief_shade` en **BC5** (RGTC RG : R = L, G = A) avec toute la chaîne de mipmaps dans la
    disposition de Godot (235 Mo, 1 octet par texel) ;
  - `wetlands` en **BC1** (DXT1, sans mipmaps, 22 Mo).
- Stockage : flux d'octets découpé en parts égales compressées zlib (`<nom>_<i>.bin`, ≤ 32 Mo
  bruts, ≈ 16 Mo sur disque), décrites dans `map.json` (`relief_shade.bc5`, `wetlands_gpu` :
  format, taille, mipmaps, octets par part). Aucun fichier versionné ne dépasse 50 Mo.
- Le jeu (`ReliefLandcover.load_gpu_copy`) lit les parts une à une et crée l'image par
  `Image.create_from_data` : ni décodage PNG ni calcul de mipmaps ; les PNG restent la source
  maîtresse et le repli (cartes d'essai, copie absente ou incohérente).
- Encodeurs maison (numpy) plutôt que ceux de Godot : `Image.compress` (etcpak) n'existe que dans
  l'éditeur et son BC4 donnait jusqu'à 29 niveaux d'erreur ; ici BC4 à bornes min/max (erreur
  ≤ étendue du bloc / 14, moyenne 0,8 niveau), BC1 à bornes de la boîte englobante.
- Le shader lit l'occlusion en `.g` quand la texture est en RG (`relief_shade_rg`), en `.a` pour
  le repli LA8.

## Conséquences
- Texture du relief 447 → 224 Mo, zones humides 126 → 22 Mo ; plus de pic transitoire du relief ;
  chargement du relief ≈ 0,6 s (décompression zlib) au lieu de ≈ 2,3 s de décodage.
- Perte : quantification BC4 du détail d'altitude dans les blocs à fort relief (≤ 5 niveaux de
  1,5 m dans 1 % des blocs) ; invisible à l'échelle de la carte (gradient lissé par le filtrage).
- Toute régénération des PNG doit régénérer les copies (fait par les commandes `geo` ; sinon
  `cent-ans geo gpu-textures`). Le dépôt porte ≈ 131 Mo de parts en plus des PNG.
- Les GPU sans BC (rares sur macOS Apple Silicon et Windows) : Godot décompresse à l'envoi.
