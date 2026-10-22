# KTX2 + préprocesseur d'assets (client Bevy)

Statut : inventaire fait (2026-10-22), rien d'implémenté.

## Inventaire (`game/assets`)
- 3 700 JPG (1,5 Go), 2 232 PNG (190 Mo), 30 WebP ; 0 KTX2/DDS.
- Gros postes : `models/dn` 457 Mo (2 172 fichiers, surtout glb/images de génération ; usage runtime à confirmer),
  `textures/battle` 457 Mo (102), `textures/terrain` 363 Mo (15, déjà BC1 maison, ADR 0118),
  `textures/vegetation` 88 Mo, `textures/buildings` 60 Mo, `heraldry/banners` 25 Mo (1 032), `textures/water` 24 Mo.
- Aucun encodeur KTX2 installé (`toktx`, `basisu`, `kram` absents).

## Plan
1. Confirmer quelles textures sont chargées au runtime (hors `models/dn` brut, `art/`, `portraits`).
2. Installer un encodeur (KTX-Software `toktx`, ou `basisu`) ; script `tools/cent_ans_tools/` : JPG/PNG → KTX2 (zstd, BC7 couleur, BC5 normales, mips).
3. Activer la feature `ktx2` (+ `zstd`) de Bevy dans `client/Cargo.toml` ; charger via les mêmes chemins.
4. Mesurer avant/après : mémoire GPU, temps de chargement bataille. Exclure les cartes composées (terrain, formats maison).
5. Préprocesseur (`AssetMode::Processed`) ensuite : adapter `run_bg.sh`, `tools/launch.sh`, smoke, CI, paquet Windows.

## Points ouverts
- Sources JPG sRGB vs données linéaires (normales, masques) : choix du format par suffixe (`_diff`, `_nor`, ...).
- Les .tres/JSON qui référencent les chemins doivent suivre le changement d'extension.
