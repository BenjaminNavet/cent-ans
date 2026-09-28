# GA — Qualité visuelle par assets générés et sources CC0 haute résolution

Date : 2026-09-28. Statut : design validé par le joueur (brainstorming), plan à écrire.

## Objectif

Améliorer la qualité visuelle en bataille (le joueur joue **souvent en vue rapprochée**) et en
campagne en retenant les plus gros gains par euro et par risque. Critère de sélection :
part de l'écran × écart de qualité × faisabilité × coût.

## Constats (planche PO `docs/img/po/apres/`, ADR 0014, 0088, 0089, 0093)

- ADR 0093 : le goulot visuel est la qualité des assets, pas le moteur.
- La 2D est déjà générée par IA (portraits, icônes, miniatures, Codex : ~21 $).
- Figurines fines : bonne géométrie (MakeHuman, 9–17 k triangles, rig, variantes, normales et
  occlusion cuites) mais **aucune texture de couleur** : couleur par sommet + matières
  procédurales ; 8 tuiles de détail numpy (ADR 0088). Aspect « plastique » de près.
- Sol de bataille (~70 % de l'écran en vue tactique) et bâtiments : textures **Poly Haven CC0
  photographiques en 1k** (albédo 1024², normales 512²). L'IA ne fera pas mieux qu'un scan :
  le levier est la résolution, des couches manquantes et la macro-variation.
- Campagne : eau grise plate, plaines monotones.

## Principe

IA là où aucune source photo n'existe ; CC0 haute résolution partout ailleurs. Pas de
figurines entièrement générées en 3D (casserait skinning, codes matière, armoiries, blessés,
AN1 pour un gain incertain).

## Chaîne commune

- Module `tools/cent_ans_tools/material_gen.py`, réutilisant `openrouter.py` et `budget.py`
  (ligne de budget par lot dans `docs/budget.md`, section GA).
- Texture IA : prompt → image 1024² (`openai/gpt-5-image-mini`) → **tuilable** (décalage de
  moitié + fondu des coutures, numpy) → dérivées locales : hauteur (luminance passe-haut) →
  normale (OpenGL) → rugosité → planche de contrôle PNG.
- Prompts et paramètres dans `data/art/materials.yaml`, validé par
  `data/schemas/materials.schema.json`.
- `SOURCE.md` par dossier d'assets : provenance, licence (CC0 ou « généré, projet »), prompt.
  Aucun asset sous licence NC ou restreinte dans le dépôt (public).
- Tests pytest : tuilabilité (écart des bords ≤ seuil), dimensions, canaux, schéma.

## Lots (ordre de gain)

### GA1 — Matières des figurines (IA)

- ~12 tuiles IA 512² remplaçant les 8 tuiles numpy de l'ADR 0088 : laine, lin, futaine,
  gambison matelassé, mailles, cuir, plates, bois, peau, cheveux, robe de cheval claire,
  robe de cheval foncée.
- Format conservé (RG normale, B relief, A rugosité) + **nouveau tableau d'albédo de détail**
  centré en luminance (moyenne 0,5) qui **multiplie** la couleur de sommet : armoiries et
  livrée restent pilotées par les données.
- Branché sous `#ifdef FG3_BAKED`, même projection que les tuiles actuelles
  (`FG3_TILE_SIZE`) ; aucune recuisson d'atlas. A/B : `--no-ga1`.
- Hors périmètre : visages peints (≈ 20 px à l'écran, UV MakeHuman non respectables par l'IA).
- Budget : ≤ 4 $. Mémoire : ajout ≤ 4 Mo (BC7 + mipmaps), budget figurines 60 Mo inchangé.

### GA2 — Sol de bataille (CC0, 0 $)

- Les 9 couches Poly Haven passent en 2k (albédo et normale) ; 3 à 4 couches ajoutées :
  prairie fleurie, herbe piétinée, chaume/éteules, labour.
- Macro-texture de variation (procédurale, plusieurs octaves de 50 à 200 m)
  modulant teinte et luminance pour casser la répétition.
- Préalable : mesure mémoire du `Texture2DArray` (13 couches, 2k, BC7) ; si > 120 Mo,
  normales en 1k.
- A/B : `--no-ga2`.

### GA3 — Décor 3D statique (image-vers-3D)

- Pipeline : image de référence gpt-image (vue 3/4, fond neutre) → **TRELLIS** (licence MIT ;
  Hunyuan3D exclu : licence non valable en UE) → Blender scripté
  (`tools/blender_scripts/ga3_cleanup.py` : décimation, LOD0/1/2, dépliage UV, cuisson de
  l'albédo) → `.glb` dans `game/assets/models/props_ga/`.
- **Sonde d'abord** : 1 objet (maison à colombages), jugement qualité, coût, licence, puis
  go/no-go avant le lot complet.
- Lot complet : 8–12 objets (maison paysanne, maison à colombages, église de village, moulin,
  chariot, tente, palissade, puits, trébuchet, bélier).
- Budgets : LOD0 ≤ 8 k triangles (bâtiment), ≤ 3 k (objet) ; albédo 1024².
- Exécution de TRELLIS : service hébergé payant à l'appel si ses conditions cèdent les droits
  sur les sorties ; sinon lot abandonné (pas de GPU CUDA local). Budget : ≤ 8 $, sonde incluse.

### GA4 — Campagne

- Eau : normale et couleur CC0 animées ; plaines en 2k ; macro-variation comme GA2.
- IA ponctuelle seulement pour des vignettes de ville si l'écart reste visible. Budget ≤ 2 $.

### GA5 — Bâtiments (CC0)

- Textures Poly Haven des bâtiments en 2k ; variantes de torchis et de colombages.
- IA en complément seulement si aucune source CC0 ne convient. Budget ≤ 1 $.

## Vérification

- Tests : pytest de la chaîne ; tests Godot de mémoire des textures (modèle
  `fg3_maps_test.gd`) et de chargement des `.glb`/LOD ; `smoke.gd` vert.
- Performance : A/B en passes alternées sur le banc standard et `--closeup` (ADR 0089),
  durée moyenne d'image ; régression tolérée ≤ 5 % par lot.
- Visuel : une planche avant/après par lot, 3 captures au plus ; le jugement du joueur sur la
  planche vaut validation.

## Budget et ordre

- Plafond du chantier : **15 $** (GA1 4 $, GA3 8 $, GA4 2 $, GA5 1 $), pris sur le reste de l'enveloppe DA (≈ 28,7 $).
- Ordre : GA1 ∥ GA2 (fichiers disjoints) → GA5 ∥ GA4 → sonde GA3 → GA3.
- ADR : albédo de détail des figurines (GA1), textures 2k et macro-variation (GA2/GA4/GA5),
  pipeline image-vers-3D (GA3).
- Orchestration : `docs/wip/ga.md`.
