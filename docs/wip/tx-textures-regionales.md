# TX — Textures régionales générées en local (plan et suivi)

Spécification : `docs/superpowers/specs/2026-10-09-textures-regionales-design.md` (approuvée
2026-10-09). Mandat : enchaîner les tranches sans redemander, 0 $, génération locale seulement
(Z-Image mflux, ADR 0190), aucune fenêtre au premier plan.

## État

- 2026-10-09 : spec commitée (43354418e) ; plan écrit ; squelette T1a en cours.

## Prochaine étape

T1b et T1c (agents Sonnet en parallèle), puis T1d.

## Réservations

- ADR 0236 : fabrique de textures locale (T1f).
- ADR 0237 : 14 biomes (T2a).

## Tranches et lots

Chaque lot : fichiers, tests, critère de fin. « mech » = agent `cent-ans-mech` (Sonnet),
« dev » = `cent-ans-dev`, « moi » = session principale. Chaque tranche fusionnée seule dans `main`.

### Tranche 1 — Fabrique (`tools/cent_ans_tools/texture_factory/`)

| Lot | Qui | Contenu | Critère de fin |
|---|---|---|---|
| T1a | moi | squelette : paquet et modules vides avec API documentée, sous-commande `cent-ans textures`, schéma `data/schemas/texture_catalog.schema.json`, `tools/tests/test_texture_factory.py` désactivé | `uv run --project tools pytest` vert, commit |
| T1b | mech | porter `seamless`, `pbr` (`derive_normal_rough`, aplatissement de l'éclairage, égalisation), `checks` (`seam_ratio`, `blotch_score` + plages luminance/saturation), `pack`, `board` depuis `ground_materials.py` ; `ground_materials.py` réexporte depuis la fabrique (HB reste fonctionnel) | tests unitaires par module ; `test_ground_materials.py` inchangé et vert |
| T1c | mech | `catalog` (lecture YAML + schéma), `generate` (`local_art.render_image`, taille 1024 ou 1536 par entrée, cache `~/dev/cent-ans-raw/textures/<famille>/<id>_<n>.png`, reprise, manifeste avec ratés et `flagged`, graine + 1 deux fois au plus) | tests avec un `runner` factice, aucun appel mflux |
| T1d | dev | `upscale` : Real-ESRGAN ncnn-vulkan macOS installé hors dépôt (`~/models/realesrgan/`, chemin modifiable par variable), wrapper, erreur claire si absent ; mesure du temps sur 3 images | sonde de bout en bout sur 3 matières (sol, écorce, mur) ; planche relue par moi |
| T1e | mech | `alpha` (rembg local, comme GA3) ; micro-détail (hauteur fine passe-haut, tuile 2k par famille) | tests de forme et de plage |
| T1f | moi | ADR 0236 ; `docs/pipeline-assets-3d.md` renvoie à la fabrique pour les textures | relu, commit |

### Tranche 2a — 14 biomes (dev, délicat)

- `data/map/biomes.yaml` : classes 8-14, champ `parent`, règles (Köppen BW → désert ; ET au
  nord d'une latitude → toundra ; boîtes et lisières pour continental est, Atlantique sud,
  Pannonie, hémiboréal, maquis égéen) ; `data/schemas/biomes.schema.json`.
- Recuisson `cent-ans geo biomes` ; planche de la carte recolorée relue par moi.
- Consommateurs (repli sur `parent`) : `core/crates/vegetation/src/{species,stands,lib}.rs`,
  `core/crates/godot-bridge/src/{vegetation_scatter,battle_terrain_field,battle_terrain_kernel}.rs`,
  `game/scripts/battle/battle_terrain.gd`, `game/scripts/map/{countryside_layer,field_layer,
  rock_outcrops,hb_ground,map_bird_flocks,tree_species,vegetation,vegetation_mask}.gd`.
- Données : `ground_biome_mix.json`, `tree_species`, palette de la carte de couleur (entrées 8-14).
- ADR 0237. Tests : `cargo test` (repli), pytest (règles), `smoke.gd`.

### Tranche 2b — Sols de campagne

| Lot | Qui | Contenu |
|---|---|---|
| T2b1 | mech | `data/art/textures/ground_campaign.yaml` : ~40 couches de fond (3 rôles × biome + partagées) et 27 + ~15 matières de parcellaire, prompts dérivés de `ground_materials.yaml` |
| T2b2 | moi | génération en arrière-plan (~80 images), planche, refaire les ratés flagrants |
| T2b3 | dev | `terrain.gdshader` + `campaign_textures.gd` + `ground_materials.gd` : table biome × rôle → couche, fondu ~10 km aux frontières, micro-détail ; `--legacy-textures` |
| T2b4 | mech | réglage « Qualité des textures : haute / moyenne » (Réglages, saute le niveau 2k) |
| T2b5 | moi | captures de contrôle par biome (`*_shot.gd`, ≤ 10) |

### Tranche 2c — Sols de bataille

- `data/art/textures/ground_battle.yaml` : 13 rôles × 14 biomes avec héritage (mech).
- `data/fx/battle_ground_layers.json` + schéma : « rôle → matière par biome », suppression de
  `terrain_tints` ; `battle_terrain_splat.gd`, `battle_terrain.gd` : chargement du seul biome du
  lieu, 2k/2k ; `battle_ground.gdshader` : micro-détail < 30 m (dev).
- Génération (moi), captures de contrôle par biome.

### Tranche 3 — Végétation

- `data/art/textures/vegetation_cards.yaml` (~70 cartes 1024 + alpha) et `foliage_bark.yaml`
  (12 essences : feuilles et écorce 2k), herbe de bataille par groupe (vert, sec, steppique,
  alpin, arctique) (mech).
- Branchement : `tree_species` → textures, `battle_vegetation.gd`, `battle_trees.gd`,
  cartes de campagne (dev).
- Eau : textures de `game/assets/textures/water/` régénérées en 2k par le générateur procédural
  existant (mech).

### Tranche 4 — Bâtiments

- `data/art/textures/building_materials.yaml` (~60 matières régionales, colombages composites)
  (mech).
- `building_regions.json` + schéma : champ `materials: {rôle: id}` ; `building_regions.gd`,
  `building_materials.gd` : choix par région, repli sur la matière par défaut (dev).
- Génération, captures de contrôle.

### Fin

Partie pilote du joueur ; supprimer les textures Poly Haven (ou les garder pour la bataille).

## Points ouverts

- Temps réel de Real-ESRGAN sur M4 Pro (T1d) : s'il dépasse ~30 s par image, réduire à 1536 →
  2048 seulement pour les sols de bataille.

## État T1c (catalogue + génération)

- Fait : schéma complet, `catalog.py` (`load_catalog`, `select`, `build_prompt`, `CatalogError`),
  `generate.py` (`generate`, `retry_flagged`, `load_manifest`, `image_path`), CLI
  `cent-ans textures generate <famille> [--only id]`, `render_image(size=...)`.
- `retry_flagged` fait UN pas par appel (essai n -> n+1, graine + n) ; un id déjà à l'essai 3 passe
  `flagged`. L'étape checks rappelle avec les ids encore rejetés.
