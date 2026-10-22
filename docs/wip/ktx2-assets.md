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

## Essai encodeur (2026-10-22)
- `brew` n'a pas de formule KTX. KTX-Software 4.4.2 (release GitHub, `.pkg` dézippé avec `pkgutil --expand-full`, sans sudo) : `ktx` + `libktx*.dylib` copiés côte à côte.
- `ktx create --encode bc7` n'existe pas en 4.4 : passer par **UASTC + zstd** (`--format R8G8B8A8_SRGB|UNORM --encode uastc --uastc-quality 2 --zstd 19 --generate-mipmap`), que Bevy transcode en BC7 (feature `basis-universal` + `zstd`).
- Mesure sur `castle_wall_varriation_{diff,nor}.jpg` : diff 2,4 Mo JPG → 5,1 Mo KTX2 (8,2 s), nor 0,97 → 1,3 Mo (1,4 s). Le disque grossit ; le gain est GPU (BC7 = 1 o/texel contre 4 o en RGBA8, mips inclus) et chargement (pas de décodage JPEG).
- Conséquence : ne pas committer ~630 Mo de KTX2. Sources JPG/PNG restent dans git ; KTX2 = cache généré (gitignoré) → convient au préprocesseur Bevy (`AssetMode::Processed`).

## État 2026-10-22 (fin de session)
- Fait : feature `processed-assets`, plugin `asset_processing` (compresseur enregistré, aucun défaut), ADR 0724 amendé. `cargo check` OK (11 min, ctt).
- Constat : seulement ~10 Mo de textures sûres (voir ADR). Rien n'est lancé en mode `Processed`, non testé à l'exécution.
- Suite possible : (a) abandonner KTX2 ; (b) adapter les consommateurs CPU (cuire mips/atlas hors ligne, ordre : ground.rs,
  water/detail.rs, vegetation/mips.rs, towns/atlas) ; (c) ne compresser que `models/` (glb) si `fx/model_mips.rs` est adapté.

## Mesure du gain réel (2026-10-22, option 2 examinée puis suspendue)
- Le chiffre « ~5,9 Go de VRAM » pour `textures/battle/tx` additionnait les 14 biomes ; une bataille ne charge qu'un pack
  (`load_pack`) : albedo + normales, bandes de 13 couches 1024² (1024×13312), soit ~18 Mo RGBA8+mips chacune.
  Gain BC7 par bataille ≈ 30-40 Mo, sur une machine à mémoire unifiée : marginal.
- Même raisonnement pour la campagne (terrain/hi, vegetation) : un pack à la fois. Seule la mesure de la mémoire GPU réelle
  (outil Metal, pas de dump actuel) pourrait justifier le chantier.
- Piste moins coûteuse si on y revient : bake hors ligne des tableaux (`ktx create --layers`, UASTC + feature `basis-universal`)
  chargés à la place des bandes brutes quand le fichier existe, sans passer par `AssetMode::Processed`.
- Décision : suspendu faute de gain mesuré ; aucun consommateur CPU modifié.
