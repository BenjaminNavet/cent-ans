# M10 (partie 2) — Assets, sons et musique : spécification

Date : 2026-09-23. Objectif : remplacer les placeholders par des assets cohérents avec le style « manuscrit
enluminé / 3D stylisée low-poly » (design § 2), sans dépasser le budget cloud (50 $ v1, suivi dans
`docs/budget.md` ; enveloppe de cette tâche : **10 $ maximum**, estimation avant chaque lot, consignation
après chaque lot via `tools/cent_ans_tools/budget.py`).

## 1. Portraits (OpenRouter, `tools/`)
- Commande `cent-ans assets portraits [--limit N] [--dry-run]` : pour chaque personnage historique de
  `data/characters` (vivant en 1337 ou naissant avant 1400), prompt construit depuis les données (nom, âge
  en 1337, rang, maison, blason, traits, description) dans un style unique : « enluminure gothique française
  du XIVe siècle, buste de trois quarts, fond or et azur, cadre de parchemin » ; modèle le moins cher
  capable d'images (`cent-ans models list`), une image par personnage, redimensionnée en 256×256 PNG dans
  `game/assets/portraits/<id>.png`. Idempotent (ne régénère pas un fichier existant), budget vérifié avant
  chaque appel (`budget check`), `--dry-run` imprime prompts et coût estimé.
- Godot : panneau Cour et fiche personnage affichent le portrait s'il existe, sinon le placeholder actuel
  (couleur + initiales). Personnages générés : placeholder.

## 2. Héraldique procédurale (gratuit, `tools/`)
- Commande `cent-ans assets heraldry` : un écu PNG 128×128 par faction dans `game/assets/heraldry/<id>.png`
  à partir de `heraldry.primary_color`, `secondary_color` et d'une interprétation simple du blasonnement
  (`semé de fleurs de lis`, `écartelé`, `lions`, `bandes`, `pal`…) : formes vectorielles tracées en Python
  (Pillow), contour et ombrage léger. Utilisé dans la barre supérieure, le panneau Diplomatie, le menu de
  départ et comme portrait des personnages générés.

## 3. Modèles 3D (Blender headless, `tools/blender_scripts/`)
- Scripts procéduraux exportant en glTF (`game/assets/models/*.glb`) : château (capitale), ville fortifiée,
  village, cathédrale (bâtiment religieux), porte-étendard d'armée (chevalier à cheval stylisé avec bannière
  colorée par matériau paramétrable), camp de siège, navire (cogue) pour le transport maritime.
  Low-poly (< 3 000 triangles chacun), couleurs par vertex/matériaux simples.
- Godot : marqueurs de villes et d'armées de la carte de campagne utilisent ces modèles quand ils existent
  (repli sur les marqueurs actuels), bannière teintée à la couleur de la faction.

## 4. Sons et musique (procéduraux, gratuits, `tools/`)
- Commande `cent-ans assets audio` (Python, numpy + wave) : effets en OGG/WAV courts dans
  `game/assets/audio/sfx/` : clic d'interface, page tournée (ouverture de panneau), cloche de fin de tour,
  fanfare (victoire), tambour de marche, choc d'armes, volée de flèches, galop, cor de guerre, chœur bref
  (événement religieux) ; musique dans `game/assets/audio/music/` : 3 pièces modales de 60-90 s (mode
  dorien / mixolydien, bourdon + mélodie, timbres synthétisés type vièle, luth en Karplus-Strong, orgue
  portatif), une ambiance « campagne » calme, une « guerre » et une « cour ».
- Godot : autoload `AudioDirector` (bus Musique / Effets, volumes réglables dans le menu, persistés dans
  `user://settings.cfg`), musique en boucle selon le contexte (guerre si le joueur est en guerre), effets
  sur les actions (fin de tour, ouverture de panneaux, bataille dans le journal, naissance, mort).

## 5. Tests et critères de fin
- Tests Python (`uv run --project tools pytest`) : génération héraldique et audio déterministes (hash des
  sorties), construction des prompts de portraits, garde-fou budget (refus au-delà de l'enveloppe),
  `--dry-run` sans appel réseau.
- Smoke Godot vert (le jeu fonctionne sans assets comme avec).
- Captures `docs/img/godot-portraits.png` (cour avec portraits), `docs/img/godot-campaign-models.png`.
- `docs/budget.md` à jour ligne par ligne ; `docs/status.md`, `docs/tools.md` à jour.
