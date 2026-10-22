# Critique ART 3D / DA : nuit du 22/10/2026

Méthode : listings, `manifest.json`, `SOURCE.md`, greps ciblés. Aucun client lancé, aucune capture, rien généré. Les constats sur le rendu réel sont donc des inférences à confirmer à l'oeil.

## 1. Les bâtiments `buildings/`, `town_kit/` et `outbuildings/` sont des assemblages procéduraux très bas en polygones
- **Problème** : ces bâtiments sortent de scripts Blender (`building_kit.py`, recettes `low`), pas de la chaîne image vers 3D. La bible § 14.4 fixe 8 000 triangles au LOD0 pour une maison et 15 000 pour un bâtiment majeur. Les manifestes donnent en moyenne 1 000 triangles pour `buildings` (max 4 528), 304 pour `town_kit` (max 905) et 467 pour `outbuildings` (max 1 555). Une grange fait 434 à 636 triangles. C'est le registre « low poly » que la mémoire du projet exclut, et que la bible n'admet qu'en lointain (§ 3, ligne 59). Si ces modèles s'affichent au zoom proche de la carte, ils jurent avec les figurines fines.
- **Preuve** : `game/assets/models/buildings/manifest.json`, `game/assets/models/town_kit/manifest.json`, `game/assets/models/outbuildings/SOURCE.md` (« recettes `low` »), `docs/design/2026-09-25-bible-da.md` lignes 59 et 344-352.
- **Gravité** : majeur
- **Correctif** : établir par un relevé de rendu (lot de revue, pas de génération) quelles familles apparaissent à une distance où le manque de détail se voit. Les remplacer en priorité par des modèles `dn/` ou `props_ga`, et laisser le procédural aux LOD lointains.
- **Effort** : M

## 2. Plus d'1,7 Go d'assets `dn/` hors dépôt, dont dépend le client
- **Problème** : `game/assets/models/dn/` pèse 1,7 Go (buildings 1,0 Go, vegetation 263 Mo, props 200 Mo) et aucun fichier n'est suivi par git (`git ls-files` renvoie 0, le chemin est ignoré). Pourtant 24 fichiers de `client/src` y font référence (`campaign/markers/dn.rs`, `countryside/config.rs`, etc.). Un clone propre n'a donc aucun de ces assets. Il n'y a ni `SOURCE.md`, ni manifeste, ni procédure de récupération au même endroit.
- **Preuve** : `git check-ignore` positif sur `game/assets/models/dn/...`, `game/assets/models/dn/package.json`, `grep -rl "models/dn" client/src`.
- **Gravité** : majeur (reproductibilité et distribution)
- **Correctif** : ajouter `dn/SOURCE.md` (origine, licence, commande de régénération ou de téléchargement). Prévoir un repli de chargement (modèle de substitution plus un log unique) pour un clone sans `dn/`. Décider d'un hébergement (release d'assets, comme pour le paquet relief) avant toute distribution.
- **Effort** : M

## 3. Un dossier d'assets sans consommateur ou à l'usage incertain : `models/*.glb` à la racine
- **Problème** : `army.glb`, `army_foot.glb`, `army_archer.glb`, `castle.glb` (2,3 Mo), `town.glb`, `village.glb`, `cathedral.glb`, `city_cathedral.glb`, `ship.glb` et `siege_camp.glb` sont des vestiges de l'ère Godot ou de premiers essais. Un grep dans `client/src` ne trouve que 2 fichiers qui les citent. `battle_skinned/` (7,5 Mo, Quaternius) subsiste en plus de `battle_fine/` (49 Mo) et `battle_ga3/` (39 Mo) : trois jeux de figurines coexistent, dont le plus grossier est gardé « le temps de la transition » (`battle_fine/SOURCE.md`, option `--coarse-figures`).
- **Preuve** : `ls game/assets/models`, `game/assets/models/battle_fine/SOURCE.md`.
- **Gravité** : mineur (poids et confusion, risque d'incohérence de style si un repli s'active)
- **Correctif** : lister les références réelles de chaque fichier racine, supprimer les orphelins, retirer `--coarse-figures` et `battle_skinned/` si la transition est finie (un ADR de clôture suffit).
- **Effort** : S

## 4. 4 398 fichiers `.import` Godot résiduels dans `game/assets`
- **Problème** : le client Godot est retiré (ADR 0366), mais `game/assets` contient 4 398 `.import`, ignorés par git et donc invisibles dans le suivi. Le git status du jour liste en plus 22 `.jpg.import` non suivis dans `models/council/`. Ils polluent les listings, gonflent les synchronisations et masquent les vrais fichiers.
- **Preuve** : `find game/assets -name "*.import" | wc -l` renvoie 4 398 ; `git status` (council).
- **Gravité** : mineur
- **Correctif** : purge locale (`find game/assets -name '*.import' -delete`, après accord car suppression de masse), ajout de `*.import` au `.gitignore` pour que le status redevienne propre.
- **Effort** : S

## 5. Salle du conseil : personnages en cartes plates, mobilier en doublon
- **Problème** : le style par défaut `painted` est une illustration découpée sur un quad orienté vers la caméra (`figures.rs`, 1,3 m), alors que `council_*.glb` (huit personnages et trois meubles) existent, avec un dossier `handmade/` et un `multiview/` parallèles. Il y a donc trois sources pour la même table, le même trône et le même tabouret (`council/`, `council/handmade/`, `dn`). Un quad plat dans un lieu de 3D semi-réaliste est la rupture de style la plus visible de la campagne. Le repli `capsule` (primitive) existe aussi, dans `figures.rs` ligne 324.
- **Preuve** : `client/src/campaign/council/figures.rs` (lignes 2-6, 27, 324), `game/assets/models/council/`.
- **Gravité** : majeur
- **Correctif** : trancher par ADR entre quad peint et glb. Si les glb sont validés (même pipeline et même échelle de 1,75 m que les figurines), passer le défaut à `glb` et retirer `handmade/` et `multiview/` du dépôt. Sinon supprimer les glb inutilisés.
- **Effort** : M

## 6. Armes et boucliers au sol construits avec quelques boîtes
- **Problème** : le module de la bataille « Meshes are a few boxes » pour les armes et boucliers laissés au sol (épées, écus, armes d'hast, arcs, arbalètes). Ces objets sont posés par milliers après une déroute et restent visibles de près en fin de bataille. Même logique pour `battle/units/volleys.rs` et `banners.rs`, qui créent des maillages primitifs.
- **Preuve** : `client/src/battle/units/arms.rs` (en-tête), recensement des `Cuboid`/`Capsule3d`/`Cylinder` dans `client/src/battle/units/*`, `battle/decor/gore.rs`, `battle/siege/works.rs`.
- **Gravité** : mineur à majeur selon la caméra
- **Correctif** : réutiliser les armes de `battle_fine_weapons.py` (déjà modélisées pour les figurines) comme maillage instancié, au lieu de boîtes. Le coût de rendu reste un draw instancié par pièce.
- **Effort** : M

## 7. Remparts, tours et portes de siège faits de volumes en code
- **Problème** : `battle/siege/works.rs` construit chaque pièce de rempart comme « wall volume, crenels, door of the gate » à partir de primitives. Le beau décor de siège généré existe (`landmarks/*_siege.glb`, `dn/siege`), mais le mur sur lequel se joue l'assaut est un prisme crénelé sans texture de pierre ni appareillage. C'est l'élément central d'une bataille de siège et le plus regardé.
- **Preuve** : `client/src/battle/siege/works.rs` (en-tête et `Cuboid`), `game/assets/models/landmarks/`.
- **Gravité** : majeur
- **Correctif** : kit modulaire de rempart (courtine, tour, porte, brèche) texturé, généré une fois via la chaîne image vers 3D ou la bibliothèque `dn/buildings`, avec variantes « endommagé » pour la brèche qui existe déjà en logique. À défaut, matériau de pierre PBR triplanaire sur les volumes actuels, pour gagner l'essentiel du réalisme à faible coût.
- **Effort** : L (kit) ou S (matériau triplanaire)

## 8. Poids des modèles : deux familles hors budget et des textures lourdes
- **Problème** : `landmarks/` pèse 39 Mo pour 14 fichiers, avec `paris.glb` (5,7 Mo), `paris_siege.glb` (5,6 Mo) et `bruges_siege.glb` (5,7 Mo). `third_party/vegetation/fir_tree_01.glb` pèse 10 Mo (le plus gros glb du dépôt) pour un sapin que la bible plafonne à 6 000 triangles, avec 3 textures de tronc plus une d'écorce, jusque dans la chaîne de la bataille. Les LOD lointains des landmarks ne sont pas visibles dans l'arborescence (un seul glb par ville, sans `_lod1`).
- **Preuve** : `find game/assets -name '*.glb' -size +5M`, `ls game/assets/models/landmarks`.
- **Gravité** : mineur à majeur (temps de chargement, mémoire GPU)
- **Correctif** : passer `fir_tree_01` au pipeline de décimation `ga3_cleanup.py` et retextureter à 1024². Produire des LOD1/LOD2 pour les landmarks (ratios 0,3 et 0,08 de la bible). Compresser les textures en KTX2 au build si le client le permet.
- **Effort** : M

## 9. Cohérence de style : quatre provenances mélangées
- **Problème** : un même écran peut mêler du Quaternius CC0 (`battle_skinned`, `third_party/characters`, `kenney_castle_kit`, `quaternius_medieval_village`), du MakeHuman via script, du Blender procédural (folk, outbuildings, buildings) et du TRELLIS (`dn`, `props_ga`, `rocks`). La bible § 3 n'admet le style « cartoon, low-poly à aplats » qu'au lointain, or les kits Kenney et Quaternius sont exactement de ce registre. Les `folk/*.glb` (charrettes, bûcher, échafaud) sont du « matériau PBR uni », donc sans salissure ni variation de teinte, ce qui les détache des props texturés de `props_ga`.
- **Preuve** : `game/assets/third_party/buildings/`, `.../characters/`, `game/assets/models/folk/SOURCE.md`, bible ligne 59.
- **Gravité** : majeur
- **Correctif** : chercher si `kenney_castle_kit` et `quaternius_medieval_village` sont référencés par le client (grep sur le client). Sinon les retirer. Sinon les basculer au lointain seulement. Pour `folk`, appliquer un atlas de salissure (rugosité et teinte variables) plutôt que des aplats.
- **Effort** : M

## 10. Anachronismes visuels potentiels dans `dn/`
- **Problème** : `dn/` couvre tout le monde médiéval (74 types de bateaux, dhow, felucca, chameaux, morse, flamant rose, palmier d'oasis). Le jeu se déroule en France, Angleterre et Bourgogne en 1337-1453. Plusieurs entrées sont hors cadre géographique ou temporel : `ship_caravel_iberian` et `ship_carrack_genoese` sont plutôt de la fin du XVe, `siege_gun_bronze_wheeled_1450` et `siege_bombard_great_1450` sont de 1450 alors qu'un lancement en 1337 ne devrait voir que `siege_cannon_early_1340` et au plus `siege_bombard_hooped_1380`, `env_boat_longship_viking` est antérieur de trois siècles, `ship_dugout_canoe_nordic` et `env_dwarf_palm` sont hors zone. Sans filtre par date dans les données, la carte peut afficher un canon de 1450 en 1340.
- **Preuve** : `game/assets/models/dn/siege/`, `dn/ships/`, `dn/fauna/`. Le filtrage par date et par région n'a pas été vérifié dans `data/`.
- **Gravité** : mineur à majeur
- **Correctif** : ajouter `year_min`, `year_max` et `regions` aux entrées du catalogue qui pilote `dn` (le champ n'existe peut-être pas ; vérifier `client/src/campaign/countryside/config.rs`), et un test de données qui refuse un asset hors fenêtre.
- **Effort** : S à M

## 11. Absence de LOD pour les familles du village et de la carte vivante
- **Problème** : seules les familles TRELLIS (`dn`, `props_ga`, `rocks/hb`) et `battle_*` ont des variantes `lod0/1/2`. `buildings/`, `town_kit/`, `outbuildings/`, `folk/` et `settlements/` n'en ont pas, alors que la bible § 14.6 prévoit `<id>[_lod1|_lod2].glb`. Avec des centaines d'instances sur la carte, le coût vertex se règle sans LOD par la seule densité. C'est lié au point 1 : ils sont bas en polygones parce que seuls leurs LOD lointains servaient.
- **Preuve** : `ls game/assets/models/buildings game/assets/models/settlements` (aucun `_lod`).
- **Gravité** : mineur
- **Correctif** : si le point 1 est traité par un remplacement progressif, ajouter les LOD au passage ; sinon documenter dans l'ADR que ces familles sont volontairement mono-LOD.
- **Effort** : S

## 12. Échelle et proportions à contrôler : bâtiments contre figurines
- **Problème** : une grange mesure 10 à 11 m de haut pour 8 m de profondeur (`barn_0`, 10,72 m de haut), et une meule ou une ruine fait 1,6 m. La bible exige des modèles à taille réelle, figurine à 1,75 m. Une grange de 10 m de haut est atypique pour une ferme de 1337 (plutôt 7 à 9 m au faîte pour une grange de ce type, hors grange dîmière monumentale). Aucun test automatique ne contrôle l'échelle des glb ajoutés par rapport à `size_m` déclaré dans les manifestes.
- **Preuve** : `game/assets/models/buildings/manifest.json` (`height`, `length`, `depth`), bible § 14.5.
- **Gravité** : mineur
- **Correctif** : script de contrôle (Python, `tools/`) qui lit les `.glb` et compare la boîte englobante au manifeste et à une plage par type ; bloquant en CI pour tout nouvel asset.
- **Effort** : S

## 13. Les `placeholder` du code sont des marqueurs techniques, pas d'assets manquants
- **Problème (constat négatif utile)** : le grep `placeholder` ne trouve que des `Entity::PLACEHOLDER`, des champs de texte et un `placeholder` d'image blanche pour les arbres (`battle/decor/tree_kit.rs`, remplacée au chargement). Il n'y a pas de fichier ou de modèle marqué « à remplacer ». En revanche la couverture reste inégale : une seule `fir_tree_01` en `third_party`, `vegetation/` ne contient que `campaign_trees.glb` et `ga3/`, et aucune liste des assets manquants n'existe (la bible § 14 n'a pas de matrice assets prévus contre assets livrés).
- **Preuve** : `grep -rn placeholder client/src`, `ls game/assets/models/vegetation`.
- **Gravité** : mineur
- **Correctif** : produire `docs/asset-coverage.md` générée par script (catalogue de `data/` contre fichiers présents) afin que le prochain relevé soit mécanique.
- **Effort** : S

## Top 3
1. **Remparts de siège en volumes de code (7)** : l'élément central de la bataille de siège est le moins abouti ; un matériau de pierre triplanaire est un gain rapide, un kit modulaire le vrai correctif.
2. **Bâtiments procéduraux en quelques centaines de triangles (1, 11)** : le plus gros écart avec la bible (1 000 contre 8 000 triangles) et le décor le plus vu de la carte.
3. **1,7 Go de `dn/` hors dépôt et doublons de provenance (2, 3, 9)** : un clone propre ne peut pas afficher le décor ; documenter la source, prévoir un repli et nettoyer les vestiges Godot et Quaternius.
