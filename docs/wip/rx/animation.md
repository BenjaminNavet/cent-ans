# RX — critique animation (soldats, chevaux, figurines, interface)

Limites de la revue : lecture du code et des données (`battle_skinned.gd`, `battle_soldiers.gd`, `battle_soldier_skinned.gdshader`, `battle_gore.json`, `battle_animation.json`, `manifest_merged.json`), `an1b_clips_test` vert, 1 capture fixe (`--closeup`, `/private/tmp/claude-501/rx-shots/animation/closeup.png`). Une image fixe ne juge pas le mouvement : les constats de fluidité sont des risques lus dans le code, marqués « à vérifier à l'œil ». Aucun fichier du dépôt modifié.

## 1. Verdict
- Forces : chaîne unique cuite (texture d'os + MultiMesh), désynchronisation par soldat (phase et cadence ±7 %, `shader:284`), fondus entre clips (`cycle_blend_s` 0,2 ; `role_blend_s` 0,25), cadence de marche asservie à la vitesse réelle (`_cadence`, `battle_soldiers.gd:1281`), allures de cavalerie pas/trot/galop/virages avec hystérésis, jeux de clips par style riches (70 clips humains, 39 chevaux), morts par cause (`battle_gore.json: deaths`).
- Faiblesses : la mêlée est une loterie de clips sans lien avec les coups du cœur ; le pas du cheval, le cheval sans cavalier et le porteur de pierres restent keyframés/absents ; table de cadence unique par clip (pas par unité) ; peu de gestes propres à certaines unités (arbalète/arc en mêlée = clips d'épée) ; aucun jugement en jeu (AS7) consigné.
- La capture montre une cavalerie lisible et sans défaut flagrant de pose (lances portées à des angles variés, étendard déployé) ; rien d'« asset à refaire ».

## 2. Constats

### [majeur] conception — Mêlée : clips tirés au hasard, non synchronisés avec les coups résolus par le cœur
**Constat** : en mêlée, chaque soldat enchaîne un clip tiré par hachage (`slash/thrust/overhead/parry/hit…`, mode CYCLE) ; un « hit » (recul) peut jouer sur un soldat qui ne reçoit rien et un coup porté n'a aucun lien avec une perte. Le joueur ne voit pas qui frappe qui ni pourquoi une ligne cède.
**Preuve** : `battle_skinned.gd:752-760` (`melee` mode M_CYCLE), shader `choose()` lignes 288-292 (`hash1(h.x*91.7 + cyc*13.1)`) ; aucun paramètre d'événement de coup dans `apply_config`. Seule la volée de tir est liée à la sim (`volley_time` sur baisse de munitions, `battle_soldiers.gd:~610`).
**Correction proposée** : déclencher un `hit` (déjà présent : `hit`, `hit_b`, `hit_c`) à l'instant d'une perte du régiment (événement d'impact déjà utilisé pour les morts) par un `hit_time` régiment + hachage de sélection des soldats touchés ; réduire le poids des `hit*` dans le jeu de base. Fichiers : `battle_soldiers.gd`, shader, `battle_skinned.gd` STYLES.
**Coût** : M

### [majeur] finition — Pas du cheval et cheval sans cavalier encore keyframés
**Constat** : le pas (`c_walk`, 29 images) et `c_fall` n'ont pas la qualité mesurée du trot/galop Muybridge ; or le pas est l'allure de tout déploiement et de toute approche lente de cavalerie.
**Preuve** : `docs/animation.md` § chevaux « Reste keyframé : pas (`c_walk`), cheval sans cavalier (`c_fall`) » ; cadence `c_walk` 1,8 m/s (`battle_gore.json:~107`).
**Correction proposée** : source Muybridge « walk » (domaine public, planches existantes) via `track_quadruped.py fit … walk`, nouvelle entrée `FREE_WALK` dans `battle_skinned_gaits.py`, recuit puis `bake_skinned_manifest.py` ; mesurer avec `as8b_measure.py c_walk`. Ne promouvoir que si le glissement baisse (règle AS).
**Coût** : M

### [majeur] équilibrage — Seuils d'allure et cadence : le galop minimal est déjà accéléré de 24 %
**Constat** : le galop est choisi dès 5,6 m/s alors que la vitesse nominale du clip est 4,5 m/s : à l'entrée en galop le clip est lu à ×1,24 (jusqu'à ×1,8 plafonné) ; le trot de 3,2 à 5,6 m/s pour un nominal 3,4 va de ×0,94 à ×1,65. Foulées nerveuses en haut de plage, risque de pieds qui « patinent » ou de trot de souris au passage de seuil.
**Preuve** : `data/fx/battle_animation.json: cavalry_gaits` (3,2 / 5,6) contre `battle_gore.json: cadence` (`c_trot` 3,4 ; `c_gallop` 4,5) ; clamp `0.35–1.8` `battle_soldiers.gd:1288`. À vérifier à l'œil.
**Correction proposée** : relever `trot_min_speed`→ ~3,4 et caler `c_trot` nominal sur le milieu de plage (~4,4), `c_gallop` nominal ≈ 5,6-6,0 ou abaisser `gallop_min_speed`; valider avec `as3_test`/`as8b_test`. Données seulement.
**Coût** : S

### [majeur] finition — Jitter de cadence ±7 % par soldat contredit l'asservissement des pieds
**Constat** : la cadence du régiment est calée sur la vitesse réelle, mais chaque soldat joue ensuite à `mix(0.93,1.07,h.y)` : jusqu'à 7 % de glissement de pied résiduel sur toute la formation, visible en gros plan sur marche lente.
**Preuve** : shader lignes 284, 298 (`anim_time * speed * mix(0.93,1.07,h.y)`) ; `_cadence` ne le compense pas.
**Correction proposée** : réduire à ±3 % en mode LOOP de locomotion (garder la désynchronisation par la phase `h.z*11`), jitter plein conservé pour les idles. Un uniform `loco_rate_jitter`.
**Coût** : S

### [majeur] conception — Une cadence nominale par clip, pas par unité
**Constat** : tous les fantassins en `walk` ont 1,35 m/s nominal ; les chevaliers lourds, milices ou piquiers ne marchent pas au même pas ; pique `pike_walk` 1,3. Quand la vitesse de l'unité diffère, la cadence monte/descend (0,35–1,8) au lieu d'avoir une foulée adaptée.
**Preuve** : `battle_gore.json: cadence` (clés = noms de clip uniquement) ; `_cadence` lit `table[names[0]]`.
**Correction proposée** : à l'évidence, si les vitesses d'unité du cœur sont groupées (marche 1,0-1,6), cela fonctionne ; sinon ajouter un facteur par style/poids. Mesurer d'abord la distribution des `_speed` en bataille (sonde headless) avant de changer.
**Coût** : M

### [majeur] finition — Archers/arbalétriers en mêlée jouent des clips d'épée, sans arme tirée
**Constat** : les styles `bow`/`crossbow` utilisent `slash/thrust/parry` : un archer attaque à l'épée à mains nues et l'arc reste en main (ou disparaît) selon le rig ; pas de coup d'arc, pas de dague.
**Preuve** : `battle_skinned.gd` styles `bow`/`crossbow` clé `melee` ; `battle_gore.json: weapons` associe `unit_longbowmen: axe`, `unit_crossbowmen: sword`.
**Correction proposée** : vérifier à l'œil (capture archers au contact) ; si l'arme est un arc, ajouter un clip `bow_bash` (coup de crosse) réutilisant `bow_shoot` inversé, ou cacher l'arc sur état `melee` (uniform). Cohérent avec l'économie d'assets (pas de nouveau modèle).
**Coût** : M

### [mineur] finition — Cycle de mêlée : durée du cycle ≠ durée du clip
**Constat** : `cycle` 1,3 s (CYCLE) pour des clips de 25-32 images à 24 i/s (1,04-1,33 s) : le dernier instant du cycle tient la pose finale (ou coupe) puis enchaîne ; arrêts de ~0,25 s visibles en groupe, atténués par le jitter ±10 %.
**Preuve** : `STYLES.sword.melee cycle 1.3` ; `thrust` 32 images = 1,33 s > 1,3 (coupé de 0,03 s), `overhead` 25 = 1,04 s.
**Correction proposée** : cycle par clip (durée = `clip_seconds`) ou fondu de sortie ; ou garder si l'effet est invisible (à vérifier à l'œil).
**Coût** : S

### [mineur] équilibrage — Cavalerie en mêlée : seulement `c_thrust` + `c_idle`
**Constat** : tout cavalier au contact répète la même pointe de lance (2 clips sur 3) ; pas de coup d'épée/masse, pas de réaction de monture hors cabrage vs piques.
**Preuve** : `STYLES.lance.melee {"c_thrust","c_thrust","c_idle"}`, `horse_bow.melee`.
**Correction proposée** : ajouter `c_slash`/`c_swing` (cavalier + cheval au pas) cuit via `battle_skinned_gaits.py`, ou cabrage occasionnel (`c_rear` existe) à faible poids dans `melee`.
**Coût** : M

### [mineur] finition — Mort de cavalier : seulement 3 clips, sans chute de cheval cohérente
**Constat** : `c_death`, `c_death_m`, `c_fall` (30/30/108 images) pour tous les cas ; les causes `ball`/`stone` utilisent `c_death_m` seul.
**Preuve** : `battle_gore.json: deaths` (`mounted` listes), manifeste.
**Correction proposée** : un clip `c_death_back` (chute arrière) pour `charge`/`ball`. À juger à l'œil d'abord.
**Coût** : M

### [mineur] finition — Servants et porteurs d'engins : pas d'animation propre pour bombarde/mangonneau
**Constat** : « Bombarde, mangonneau, porteurs de pierres, cheval attaché : aucune vidéo libre exploitable ; procédural ou tournage du joueur (AS6) ».
**Preuve** : `docs/animation.md` § Ce qui manque encore.
**Correction proposée** : réutiliser `load`/`load_heavy`/`haul`/`swab` (existent dans `fine_human`) en séquences de servant, déclarées dans `data/fx/siege_engines.json` (`crew.clips`) ; aucun nouvel asset.
**Coût** : S

### [mineur] conception — Transitions à état visible : porte-étendards et musiciens
**Constat** : `role_clips` ne définit pas d'état `melee`/`routing` pour `standard`, `drum`, `horn` : en mêlée ils gardent l'attente et en déroute ils courent comme les autres via repli ; le porte-étendard mort/tombé est géré par le rendu (EP5).
**Preuve** : `data/fx/battle_animation.json: role_clips` (seuls `charging`, `victory`, `idle_alt`).
**Correction proposée** : ajouter `routing` → `std_run`/`drum_run` et `melee` → `std_plant`/`drum_beat` (clips déjà cuits). Données seulement.
**Coût** : S

### [mineur] finition — Interface/campagne : animations hors bataille non vérifiées ici
**Constat** : figurines de carte (`army_figures.gd`, `campaign_army_walk.json`) et bêtes (shader `animal_motion`) sont mesurées sur vidéos (AS8c) mais l'AS7 « jugement en jeu » reste ouvert.
**Preuve** : `docs/wip/as8.md` « Prochaine étape… AS7 étendu aux clips AS8 ».
**Correction proposée** : une capture de contrôle campagne (budget 5) dans une session visuelle dédiée, cadences ajustées dans `data/fx/animal_motion.json`.
**Coût** : S

## 3. À ne surtout pas changer
- Chaîne unique texture d'os + MultiMesh, pas d'AnimationPlayer ni de Skeleton3D (perf de masse).
- Manifeste fusionné cuit (ADR 0239) et `bt6_manifest_test`.
- Trot et galop Muybridge (AS8b), hystérésis des allures, virages hérités du trot.
- Cadence asservie à la vitesse et horloge de régiment (`_lag`) : continue, sans saut de phase.
- Désynchronisation par phase de soldat, fondus (`cycle_blend_s`, `role_blend_s`), volée liée aux munitions.
- Règle « un clip ne devient défaut que s'il bat l'actuel » : le rejet des clips vidéo de reconstitution (pieds qui glissent, AS8a) est correct.
