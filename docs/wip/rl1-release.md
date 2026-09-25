# RL1 — build de release et performance globale

Agent RL1, nuit du 25/09. Machine : Apple M4 Pro (Apple9), 48 Go, 14 cœurs, macOS 26, écran
Retina (échelle 2). **Machine partagée** : charge moyenne 5 à 112 pendant les mesures, et surtout
un autre Godot en rendu 1920×1080 (playtest Q1/Q2) une bonne partie du temps : le GPU est
disputé, les temps d'image varient du simple au double d'une exécution à l'autre. Chaque chiffre
ci-dessous donne la charge ; les comparaisons fiables sont celles faites dans le même processus
(`--map-ab`) ou les compteurs (primitives, dessins).

## Tâches
1. [x] Build de release + export macOS vérifié (menu, partie, carte, fin de tour, sauvegarde, bataille)
2. [x] Performance globale : 3 goulets identifiés, 2 corrigés, 1 documenté
3. [x] Préréglage de qualité par défaut selon le GPU détecté
4. [x] Premier lancement : shaders pré-calculés à l'export, cache Metal préchauffé, écran de démarrage

## Procédure de release (à suivre telle quelle)
```
tools/export_macos.sh            # build release Rust, import, export, data/, signature, préchauffage
open "export/Cent Ans.app"       # ou : "export/Cent Ans.app/Contents/MacOS/Cent Ans"
```
- Prérequis (une fois par machine) : modèles d'export Godot 4.7.2 (déjà installés dans
  `~/Library/Application Support/Godot/export_templates/4.7.2.stable`) et **Metal Toolchain**
  d'Xcode (`xcodebuild -downloadComponent MetalToolchain`, installée cette nuit) : sans elle le
  pré-calcul des shaders s'arrête au SPIR-V (« Metal shader baking limited to SPIR-V »).
- L'export n'est **pas** headless : le pré-calcul des shaders (`shader_baker/enabled=true` dans
  `export_presets.cfg`) a besoin du rendu Forward+ ; une fenêtre d'éditeur s'ouvre ~30 s. L'éditeur
  fenêtré réécrit `project.godot` en sortant : le script le sauvegarde et le restaure (`trap`).
- macOS minimum 14.0 (arm64) : avec 11.0, cible Metal Apple7/MSL 2.3 et 4 shaders moteur
  (`SceneForwardClusteredShaderRD`, brouillard volumétrique, FSR2) restaient à compiler au
  lancement (« requires Apple6 / MSL 3.1 »). Le modèle d'export n'existe qu'en « universal ».
- Fin du script : un parcours automatique du jeu exporté (`-- --journey --frames=30 --turns=1`)
  vérifie le build et remplit le cache Metal de macOS (`$(getconf DARWIN_USER_CACHE_DIR)/fr.navet.centans`) :
  le joueur de cette machine n'a donc pas le premier lancement lent. `CENT_ANS_NO_WARMUP=1` le saute.
- `export/` est dans `.gitignore` (application ≈ 740 Mo, pck ≈ 420 Mo).

### Parcours automatique `--journey` (`game/scripts/dev/release_journey.gd`)
`--script` n'existe pas dans un jeu exporté (option « X » de Godot) : le parcours est lancé par le
menu principal (`ReleaseJourney.maybe_start`) quand `--journey` suit `--`. Étapes : menu 3D →
nouvelle campagne France → carte à 1500 / 1250 (fondu parchemin) / 491 / 150 → 3 fins de tour
(cœur seul) + 1 fin de tour complète de la carte → sauvegarde → bataille France–Angleterre lancée
comme par « Combattre » → sortie propre. Ligne `JOURNEY_JSON {...}`, code 0/1. Réglages, codex et
sauvegardes vont dans `user://rl1_journey_<pid>/` (effacé) : **réglages par défaut**, rien n'est
écrit chez le joueur. Options : `--turns=`, `--frames=`, `--no-battle`, `--uncapped`,
`--map-ab=<distance>` + `--ab-configs=base,medium,no_fine,msaa_off,hide:<Nœud>,…` (coûts isolés,
configurations alternées dans le même processus, médiane GPU sous `--rendering-driver vulkan`).
Banc de bataille T8 dans le jeu exporté : `-- --journey=battle --benchmark --units=20 --bench-at=40`.

### Vérification du jeu exporté
Journal sans erreur ni avertissement de script sur tout le parcours (menu, carte, fins de tour,
sauvegarde, bataille). Deux erreurs de fuite à la sortie (`ParticlesShaderRD`) venaient de la sortie
en pleine bataille du parcours : bataille et carte libérées avant `quit` désormais. Ressources :
rien ne manquait (filtre `all_resources` + `assets/*` inclut .glb, .ogg, textures des shaders
globaux ; `data/` est copié dans `Contents/Resources/data`, lu par `MapPaths`). Les replis
`res://../data` restants (`battle_standards`, `battle_gore`, `sound_bank`…) ne servent qu'après
`MapPaths.data_dir`, sans effet dans l'export.

## Mesures (réglages par défaut : qualité Haute, vsync, 1440×900 Retina)
Metal plafonne à 60 i/s (16,67 ms) même vsync coupée : une médiane de 16,7 ms = cible atteinte ;
Metal ne rend pas de temps GPU (0), mesuré sous Vulkan (`--rendering-driver vulkan`).

### Démarrage et chargements (jeu exporté)
| Mesure | Premier lancement (cache Metal vide) | Lancements suivants |
|---|---|---|
| Démarrage → première image du menu | 10,7-11,3 s (avant : écran Godot ; après : carte « Cent Ans ») | **0,82-0,89 s** |
| Menu → carte jouable | 11,4-13,1 s | **3,3-3,4 s** |
| « Combattre » → bataille affichée | 6,8-7,0 s | **0,70-0,73 s** |
Éditeur (`godot --path game`, lib debug optimisée T1) : menu 1,1-1,3 s, carte 4,0-4,5 s, bataille 0,9-1,0 s.

Le pré-calcul des shaders n'a presque rien changé au premier lancement (11,2 s sans, 10,7 s
SPIR-V seul, 10,6 s metallib complet) : le coût est la compilation finale des pipelines par le
pilote Metal (propre au GPU, mise en cache par macOS par application), que Godot ne peut pas
livrer pré-compilée. D'où le préchauffage en fin d'export et l'écran de démarrage à l'image du jeu
(`application/boot_splash`, `assets/ui/boot_splash.png`) au lieu du logo Godot.

### Fin de tour (France, graine 1337, tours 1-4)
| | release (export) | debug optimisée (éditeur) |
|---|---|---|
| `end_turn` du cœur | 63-103 ms | 64-113 ms |
| fin de tour complète de la carte (cœur + UI + événements) | 280-387 ms | 280-480 ms |
Cible < 1,5 s tenue largement (pic A5 de 18 s en debug non optimisée, réglé par T1).

### Images (médiane / p95 ms)
| Scène | Export (charge 41-113) | Éditeur (charge 5-18) | Éditeur, GPU disputé (charge 18-42) |
|---|---|---|---|
| Menu 3D | 16,7 / 16,8 | 16,7 / 17,1 | 16,7 / 17,0 |
| Carte d=1500 (parchemin) | 16,7 / 17,1 | 16,7 / 17,3 | 16,8 / 18,2 |
| Carte d=1250 (fondu) | 16,7 / 17,2 | 16,7 / 19,1 | 18,1 / 22,7 |
| Carte d=491 (défaut) | 16,7-17,4 / 17-25 | 16,7 / 18,3 | 27,7 / 33,1 |
| Carte d=150 (comté) | 18,3-20,7 / 19-21 | 18,7 / 25,2 | 35,3 / 40,2 |
| Bataille depuis la carte | 16,7 / 17,2 | 20,1 / 23,1 | 20,9 / 27,6 |

Banc T8 (éditeur, charge 6-10, `--bench-at=40`, `--bench-repeat=2`, 1200 images) :
| Bataille | soldats | i/s moy. | médiane | p95 |
|---|---|---|---|---|
| 20 régiments par camp (40) | 4 747 | 59,1 | 16,7 | 18,1 |
| 50 régiments par camp (100) | 11 941 | 57,5 | 16,7 | 18,1 |
| Siège de Guyenne (`--siege`) | 966 | 59,2 | 16,7 | 16,7 |
| Siège de Paris (`--siege-province=prov_ile_de_france`) | 927 | 59,3 | 16,7 | 16,8 |

### Coûts isolés sur la carte (`--map-ab`, Vulkan, même processus, GPU ms médian)
| Config | d=491 (GPU libre) | d=491 (GPU disputé) | d=150 (GPU disputé) |
|---|---|---|---|
| Haute (base) | 12,6 | 27,0 | 19,4 |
| Moyenne | — | 21,0 | 18,4 |
| Basse | — | 13,3 | — |
| MSAA coupé | 9,1 | 23,1 | 15,2 |
| sans relief fin | — | 26,3 | 16,2 |
| rendu 3D à 75 % | 11,2 | — | 14,9 |
| sans SSIL / SSAO / halo | — | −1,2 / −0,2 / −0,6 | −1,4 / −1,2 / — |
| sans ombres / brouillard | −1,5 / −2,7 | — | −0,5 / — |
| sans mer / fleuves / côtes / routes / végétation / villes | −2,4 / −2,8 / −2,2 / −2,7 / −1,9 / −0,9 | — | routes −1,5, végétation −0,8, villes −1,5 |

## Les 3 goulets
1. **Premier lancement** (≈ 11 s d'écran figé avant le menu, 8 s de plus à la carte, 7 s à la
   première bataille) : compilation des pipelines par le pilote Metal. Corrigé autant que possible :
   shaders pré-calculés (metallib, MSL 3.1), cache préchauffé par le script d'export, écran de
   démarrage aux couleurs du jeu. Lancements suivants : menu < 0,9 s.
2. **Couche 2D du parchemin redessinée à chaque image** (≈ 3 ms CPU selon CM2) : redessinée
   seulement quand la vue change (caméra, fenêtre, fondu, positions des armées), sinon à 10 Hz
   pour le tangage des navires ; le jeton sélectionné pulse à chaque image. Caméra immobile :
   **28-30 dessins sur 180 images au lieu de 180** (−84 %). Trace de débogage « CM2 draw us »
   (toutes les 60 images dans le journal) retirée.
3. **Carte au zoom comté (d=150)** : limitée par le GPU (7,2 M primitives, MSAA ×2 sur écran
   Retina) ; 53-55 i/s à GPU libre, sous 40 i/s quand un autre rendu tourne. MSAA ≈ 4 ms, relief
   fin ≈ 3 ms. **Non corrigé** (toucherait l'aspect du niveau Haute) : pistes dans « Points ouverts ».

## Préréglage par défaut (tâche 3)
`video/quality` vaut désormais `"auto"` par défaut : `RenderQuality.detected_level()` lit
`RenderingServer.get_video_adapter_name()` → **Haute** sur Apple M2 et suivants ou M1
Pro/Max/Ultra, **Moyenne** sinon (M1 de base, Intel, AMD, inconnu). Un choix enregistré par le
joueur (`low`…`ultra` dans `settings.cfg`) l'emporte toujours. Menu Réglages : « Automatique
(Haute) » en tête de liste. Test : `godot --headless --path game --script res://tests/rl1_quality_test.gd`.

## Points ouverts
- Zoom comté : passer le MSAA ×2 en FXAA/TAA sur écran Retina (−4 ms), ou relief fin plus
  grossier entre d=100 et d=200 (−3 ms) : choix visuels, à trancher par l'orchestrateur.
- Premier lancement après une nouvelle exportation sur une **autre** machine : ~11 s avant le
  menu (écran de démarrage affiché) ; seul un préchauffage sur place l'évite.
- Le `user://` du jeu exporté (« Cent Ans ») est partagé avec l'éditeur et les tests des agents
  (sauvegardes automatiques mêlées) ; `application/config/use_custom_user_dir` le séparerait mais
  cacherait les sauvegardes existantes au joueur : non fait.
- Espace disque : la machine est tombée à 325 Mo libres pendant la nuit (≈ 286 Go de worktrees,
  12 Go de `core/target` par agent) ; j'ai seulement vidé les `incremental/` de mon worktree.

## État
Terminé ; reste la fusion de main finale, import, smoke, export et parcours.
