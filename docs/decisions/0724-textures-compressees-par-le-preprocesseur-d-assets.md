# 0724 — Textures compressées par le préprocesseur d'assets de Bevy

Date : 2026-10-22. Statut : acceptée. Suite de `docs/wip/ktx2-assets.md`.

## Contexte
`game/assets` contient ~1,8 Go d'images (3 700 JPG, 2 232 PNG), toutes décodées en RGBA8 au chargement
(4 o/texel en VRAM). Seul le sol de la carte est compressé (BC1 maison, ADR 0118). Le dépôt est public :
y committer ~630 Mo de KTX2 gonflerait l'historique à chaque retouche.

## Décision
- Les sources JPG/PNG/WebP restent dans git. Les KTX2 sont un **cache généré** par l'`AssetProcessor`
  de Bevy 0.20 (`AssetMode::Processed`) dans `imported_assets/`, gitignoré.
- Encodeur : `CompressedImageSaver` de Bevy (feature `compressed_image_saver`, bibliothèque `ctt`, BC7 couleur,
  BC5 normales, mips générés). Pas d'outil externe (KTX-Software abandonné : pas de BC7 en 4.4).
- Feature cargo `processed-assets` du client (`asset_processor`, `compressed_image_saver`, `ktx2`, `zstd_rust`) ;
  plugin `asset_processing`. Le build `dev` garde les fichiers bruts (itération rapide, `file_watcher`) ;
  `tools/launch.sh` (profil release) active `processed-assets`.
- Les normales demandent `is_normal_map` et un chargement linéaire : réglé par des `.meta` générés
  d'après le suffixe (`_nor`, ...), pas à la main.

## Conséquences
- Premier lancement lent (encodage de tout `game/assets`) puis incrémental ; cache à ne pas supprimer sans raison.
- Les formats maison (`CAM1`, `CAB1`, cartes composées) ne passent pas par le processeur.
- CI, smoke et paquet Windows doivent soit générer le cache, soit rester en mode non traité.

## Amendement 2026-10-22 : liste blanche, pas de défaut global
Audit des consommateurs (`client/src`) : presque toutes les grosses textures sont relues ou reconstruites en RGBA8 côté CPU
(`decor/ground.rs` strip→array et atlas, `siege/stone.rs`, `fx/model_mips.rs`, `vegetation/mips.rs`, `water/detail.rs`,
`towns/atlas`, `terrain/ground.rs`, UI : héraldique, portraits, icônes, art). Les compresser casserait ces chemins.
- Le compresseur n'est le processeur par défaut d'aucune extension ; une texture l'active par son `.meta`.
- Textures sûres aujourd'hui (GPU direct) : panoramas d'horizon, `textures/fx`, ornements de parchemin, textures de salle
  du conseil, impostors lointains. Total ≈ 10 Mo sur ~630 Mo : gain négligeable.
- Le vrai gain exige de rendre les consommateurs CPU compatibles (mips et atlas cuits à l'avance, ou lecture depuis les
  sources brutes) : chantier à part, non engagé.
