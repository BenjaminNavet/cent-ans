# 0105 — Textures du sol de bataille en 2k et macro-variation (lot GA2)

Date : 2026-09-28. Statut : accepté. Suite de l'ADR 0086 (convention demi-pixel des rasters) et
de la spec `docs/superpowers/specs/2026-09-28-ga-assets-generes-design.md` (§ GA2).

## Contexte

Le sol de bataille (lot V4) mélangeait 9 couches Poly Haven (CC0) en 1k dans un
`Texture2DArray`. La spec GA2 demande : passage en 2k, 3 à 4 couches ajoutées (prairie fleurie,
herbe piétinée, chaume/éteules, labour), une macro-variation procédurale (50–200 m, teinte et
luminance, faible amplitude) et un `--no-ga2` pour comparer.

Mesure mémoire préalable (avant tout téléchargement, arithmétique BC7/mipmaps supposé par la
spec, 1 octet/texel, facteur mipmaps 4/3) :
- 13 couches, albédo **et** normale en 2k (2048²) : 13 × 2048² × 4/3 ≈ 69,3 Mo chacun,
  soit **≈ 138,6 Mo** — au-dessus du plafond de 120 Mo fixé par la spec.
- 13 couches, albédo 2k / normale **1k** (1024²) : 69,3 + 17,3 ≈ **86,6 Mo** — sous le plafond.

Mesure réelle en jeu (`game/tests/ga2_ground_test.gd`, `TextureLayered.get_format()`) : le format
VRAM effectif de ces couches (sans alpha) est **DXT1/BC1** (format Godot 17), pas BC7/BPTC comme
la spec le supposait — réglage `compress/channel_pack=0` des fichiers `.import`, déjà en place
avant GA2 (`ground_albedo_array.jpg.import` du lot V4, inchangé ici). DXT1 tient 0,5 octet/texel
(la moitié de BC7) : mémoire mesurée **≈ 43,3 Mo** (albédo 2k ≈ 34,7 Mo, normale 1k ≈ 8,7 Mo),
très en dessous des 86,6 Mo anticipés. Décision inchangée (albédo 2k / normale 1k) : la marge
sous le plafond permettrait même normale 2k (≈ 138,6 Mo réel ÷ 2 ≈ 69,3 Mo hypothétique en DXT1),
mais la spec fixe explicitement le repli 1k dès 120 Mo dépassés en BC7 ; ne pas rouvrir ce choix
ici (voir « point ouvert » dans le rapport du lot).

## Décision

1. **Albédo 2k, normale 1k** pour les 13 couches du sol (mesure ci-dessus, valeur mesurée en jeu
   dans `game/tests/ga2_ground_test.gd`). La spec anticipait déjà ce repli.
2. **Identité des couches dans les données**, pas dans le shader ni le script Python :
   `data/fx/battle_ground_layers.json` (schéma `data/schemas/fx_battle_ground_layers.schema.json`)
   liste, dans l'ordre d'empilement du `Texture2DArray`, l'identifiant Poly Haven, la taille de
   répétition (m) et un `role` optionnel pour les couches que le shader traite spécifiquement.
   `build_textures.py` et `battle_terrain.gd` (`BattleTerrain.ground_layers()`,
   `ground_role_index()`) lisent ce fichier ; le shader (`battle_ground.gdshader`) reçoit le
   nombre de couches (`layer_count`), leur taille de répétition (`layer_tile_size[]`) et l'index
   des rôles ajoutés (`idx_flowering_meadow`, `idx_trodden_grass`, `idx_stubble`,
   `idx_fresh_plough`, -1 si absents) en uniformes : aucune identité de couche codée en dur dans
   le shader, hormis les 9 rôles historiques (`L_GRASS`…`L_SNOW`) dont l'occupation (splatmap,
   pente, relief) reste un calcul de rendu spécifique à chacun, pas une donnée.
3. **4 couches ajoutées, occupation câblée sur des règles existantes** (pas de nouveau coût de
   calcul par pixel notable, pas de changement de comportement hors sol) :
   - **labour frais** (`fresh_plough`, `farm_furrows`) : parcelles « labour » posées par le
     décor (EP6, `decor_kind == 1`), distinctes du labour ambiant/procédural loin des parcelles
     (qui garde `farm_soil`, layer historique 7) ;
   - **chaume/éteules** (`stubble`, `withered_grass`) : parcelles « chaume » du décor (EP6,
     `decor_kind == 6`), jusqu'ici sans texture dédiée (teinte seulement) ;
   - **herbe piétinée** (`trodden_grass`, `grassy_cobblestone`) : halo élargi déjà cuit autour des
     chemins et routes (`splat_b.g`), sans nouvelle carte ni nouveau coût de simulation ;
   - **prairie fleurie** (`flowering_meadow`, `leafy_grass`) : part des taches de prairie grasse
     existantes (bruit `patch`/`patch2`, lot V4b), sans nouvelle carte.
   Poly Haven n'a pas de texture CC0 dédiée « prairie fleurie » (wildflower meadow) à la date du
   lot ; `leafy_grass` est le substitut le plus proche du catalogue. Point ouvert, cf. rapport.
4. **Macro-variation dédiée 50–200 m** (teinte et luminance, faible amplitude, 4 octaves de bruit
   déjà utilisé ailleurs dans le shader) appliquée à l'albédo final, indépendante des taches de
   prairie (plus grande échelle) et de la macro-variation existante (`macro`/`macro2`, hors bande
   50–200 m). Sous `ga2_on` (uniforme), coupé par `--no-ga2` : les couches et leur câblage restent
   en place (pas de double jeu de textures 1k/2k à maintenir), seule la macro-variation dédiée
   est désactivée pour le A/B.

## Conséquences

- Mémoire du sol : **≈ 43,3 Mo mesurés** (`game/tests/ga2_ground_test.gd`), largement sous le
  plafond de 120 Mo — la spec anticipait 86,6 Mo en supposant BC7 ; le format réel (DXT1,
  préexistant) est deux fois plus compact.
- `data/fx/battle_ground_layers.json` devient la source de vérité pour toute couche de sol future
  (GA4 réutilisera la macro-variation ; GA5 suit le même schéma pour les bâtiments si besoin).
- La substitution « prairie fleurie » → `leafy_grass` est un choix éditorial documenté ici et
  dans `README.md` ; à revoir si Poly Haven publie une texture plus proche.
- `--no-ga2` ne restaure pas les anciennes textures 1k (compromis assumé : un seul jeu de
  textures à maintenir) ; il coupe seulement la macro-variation dédiée pour la comparaison A/B.
