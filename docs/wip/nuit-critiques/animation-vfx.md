# Critique animateur + VFX (22/10/2026)

Lecture seule de `client/src`, `data/fx`, `game/assets`, `docs/`. Aucun client lancé, aucune capture : tout constat est de code ou de données, à confirmer à l'œil.

Acquis (à ne pas refaire) : mélange entre clips et réactions aux coups existent dans `units/skinned.wesl` (`prev`/`blend_time`, `hit time/length/share`) ; gore, taches, membres coupés (`decor/gore.rs`, `blood.rs`) ; pluie/neige GPU ; fumées, poussière, flammes de bombarde, traînées de flèches, flèches plantées.

## Constats

### 1. Doc d'animation périmée (chaîne Godot)
- Problème : `docs/animation.md` décrit encore `AnimationPlayer`, `.gd`, `.gdshader`, `game/scripts/visual/animal_motion.gd`, tests `as1_test.gd`. Le Godot a été retiré (ADR 0366). Un agent de reprise cherchera des fichiers inexistants. De même, `game/shaders/` ne contient plus qu'un `.uid` ; les shaders vivent en `.wesl` dans `client/src`.
- Preuve : `docs/animation.md` (§ Bêtes, Engins, Vérifier), `ls game/shaders`.
- Gravité : majeur (aiguille les agents).
- Correctif : réécrire § « Où ça vit dans Bevy » : table clip → `units/anim.rs`, shaders → `units/skinned.wesl`, `decor/camp_horse*.wesl`, `siege/animation.rs` ; marquer les recettes Blender comme toujours valides.
- Effort : S

### 2. Commentaires de module contradictoires (corpses / gore / effects)
- Problème : `units/corpses.rs` dit « Not ported: blood stains and sprays, severed limbs », or `decor/gore.rs` et `decor/blood.rs` les portent. `units/mod.rs` dit « Not ported: corpses and wounded ». `siege/mod.rs` dit « Not ported: crew figures » alors que `siege/crew.rs` existe. Impossible de savoir ce qui manque vraiment.
- Preuve : `client/src/battle/units/corpses.rs:7`, `units/mod.rs:9`, `siege/mod.rs:25`, `decor/gore.rs:1-20`.
- Gravité : mineur.
- Correctif : une passe qui remplace chaque « Not ported » par l'état réel ; liste unique dans `docs/wip/` des manques VFX.
- Effort : S

### 3. Pas d'éclats de terre aux sabots (Clods non porté)
- Problème : la cavalerie en charge n'a que de la poussière en nuage (flipbook teinté). Pas de mottes ni d'éclaboussures de boue ; en sol détrempé ou neige la poussière est coupée (« no dust under rain or snow ») donc la charge n'a plus aucun retour visuel au sol, précisément quand la lisibilité baisse.
- Preuve : `battle/effects/mod.rs:15` (« Not ported: the clods »), `effects/dust.rs:1-12`.
- Gravité : majeur.
- Correctif : projeter 3-6 petites mottes balistiques par cavalier en galop (réutiliser le moteur de gouttelettes de `gore.rs`), teinte sol ; sous pluie/neige, gerbes de boue/neige à la place de la poussière.
- Effort : M

### 4. Allures du cheval au pas et chute sans cavalier restent keyframées
- Problème : `c_walk` et `c_fall` n'ont pas de source mesurée alors que trot/galop sont issus de Muybridge. Le pas est l'allure la plus vue (marche d'armée en bataille) ; une qualité inférieure au trot rend la différence visible dans une même colonne. Bombarde, mangonneau, porteurs de pierres, cheval attaché n'ont aucune source libre.
- Preuve : `docs/animation.md` (« Reste keyframé : pas (`c_walk`), cheval sans cavalier (`c_fall`) » ; « Ce qui manque encore »), `units/anim.rs:144` (`c_walk`).
- Gravité : mineur à majeur selon le rendu réel (à juger à l'œil).
- Correctif : tirer le pas de la même planche Muybridge (cycle « walk » existe dans la série) via `track_quadruped.py` et l'ajouter à `export_horse_gaits.py` ; réutiliser le mécanisme `FREE_*`.
- Effort : M

### 5. Pas de bruit de pas ni de cadence liée au sol/à l'allure côté VFX (empreintes, éclaboussures de pas hors gué)
- Problème : seuls les gués (`effects/ford.rs`) ont des éclaboussures de pas. Boue, neige et herbe haute ne marquent rien au passage d'un régiment : pas d'empreintes, pas de poussière de pas pour l'infanterie hors charge (la poussière est en nuage régimentaire). Une armée en marche est visuellement « posée sur » le terrain.
- Preuve : `effects/ford.rs` (seul VFX de pas), `effects/dust.rs` (nuage global par régiment), `decor/flatten.rs` (herbe aplatie, taches de sang seulement).
- Gravité : mineur.
- Correctif : réutiliser `flatten.rs` pour une traînée de sol sombre (boue) / blanchie (neige) derrière les régiments, très basse résolution, plafonnée comme `MAX_DECALS`.
- Effort : M

### 6. Mêlée : clips tirés au hasard par cycle, sans lien avec l'adversaire
- Problème : `("sword","melee")` enchaîne `slash/thrust/overhead/parry/guard` en cycle ; rien n'oriente l'homme vers un ennemi précis ni ne synchronise coup et réaction. Le système de réaction (`hit time/share`) est un ajout sur un combat visuellement déconnecté : des soldats frappent dans le vide à 1-2 m de l'ennemi. Cavalerie : `melee` et `melee_pikes` idem.
- Preuve : `units/anim.rs:93-96`, `:147-151`; `units/skinned.wesl:26-27`.
- Gravité : majeur (cœur de la lisibilité d'une bataille).
- Correctif : (a) orienter chaque soldat de mêlée vers le plus proche ennemi du régiment adverse (yaw seul, côté rendu) ; (b) décaler la phase du cycle d'un duel pour que le coup tombe quand l'autre est en `parry` (une phase par paire, déterministe par hash). Aucune règle touchée.
- Effort : M

### 7. Impact des coups : pas d'étincelles/ricochet métal ni de feedback d'armure
- Problème : le feedback d'un coup est le sang, les membres, et un petit mouvement de réaction. Rien pour les impacts sur armure/bouclier (étincelles, « ting »), alors que le jeu met en scène des hommes d'armes plaqués. Flèches : plantées, mais pas de « poof » de poussière ni d'éclat sur bouclier/pavois à l'arrivée d'une volée.
- Preuve : `battle/effects/` (8 modules, aucun d'étincelles) ; `grep -il "spark"` ne trouve que `arms.rs`, `banners.rs`, `anim.rs`, … sans module dédié.
- Gravité : majeur.
- Correctif : module `effects/impact.rs` : 2-4 étincelles additives (flipbook feu existant) sur les `Impact` de type melee contre armure, un petit nuage au point d'arrivée des flèches à distance moyenne ; plafonné par le budget de sprites.
- Effort : M

### 8. Poussière : budget de sprites partagé feux/poussière/fumées, 6 émetteurs seulement
- Problème : `DUST_EMITTERS = 6` (10 dans Godot), « big opaque cloud » par sprite, tous sur un même budget `MAX_PARTICLES` avec les feux de siège et fumées. En grande bataille le budget peut manger la poussière des charges au moment où il y a le plus d'incendies, ou l'inverse (flammes qui disparaissent). Comportement à saturation non documenté ni testé.
- Preuve : `effects/dust.rs` (constantes), `effects/flash.rs:SPRITE_ROOM = 700`, `fx/flipbook.rs`.
- Gravité : majeur (risque silencieux).
- Correctif : priorités explicites (flammes > éclairs > charges > poussière lointaine), compteur de rejets dans le dump `battle_gore`/`fx`, test de saturation.
- Effort : S

### 9. Siège : balancier du trébuchet et servants avec trous de couverture
- Problème : le trébuchet utilise une courbe mesurée (bien), mais les servants ont 6 gestes (`winch, loader, swab, pusher, pusher_still, idle`) ; pas de porteurs de pierres (confirmé « Ce qui manque »), ni de gestes de recharge de bombarde différenciés, ni de réaction d'équipage à un tir ennemi (mort d'un servant → le geste continue ?). Les béliers n'ont pas de fumée/étincelles d'impact sur la porte, seulement des « scars ».
- Preuve : `siege/crew.rs:44`, `siege/animation.rs:1-12`, `docs/animation.md:« Ce qui manque encore »`.
- Gravité : mineur.
- Correctif : vérifier côté `crew.rs` qu'un servant tué est retiré/joue un clip de mort ; ajouter un clip « porter » en réutilisant `walk` + accessoire ; poussière + éclats de bois au coup de bélier (déjà `siege/fx.rs`, vérifier le déclenchement par coup).
- Effort : M

### 10. Navires : aucune bataille navale 3D, mais mer de campagne sans interaction
- Problème : décision actée (résolution automatique, mémoire projet), donc pas un défaut. Mais les `cog/nef` n'ont qu'une voile qui respire ; pas de tangage, sillage, ni fumée de départ visibles dans `campaign/armies/sail.rs`. Une flotte mobile glisse donc sur la carte comme un pion.
- Preuve : `client/src/campaign/armies/sail.rs` (seul VFX naval), `game/assets/models/fleet/{cog,nef,bivouac}.glb`.
- Gravité : mineur.
- Correctif : tangage/roulis par shader de sommet (déjà fait pour les bêtes : `campaign/life/anim.wesl`) + ruban de sillage derrière l'armée en mer ; réglages dans `data/fx/sea_life.json`.
- Effort : M

### 11. Météo : pluie sans impacts au sol, ni éclairs, ni coupe-son de visibilité
- Problème : pluie et neige sont des traits/flocons dans une boîte autour de la caméra (bon pour le coût) mais sans anneaux de pluie au sol/sur l'eau, sans assombrissement local du terrain mouillé hors réglage global, sans éclair. La brume volumétrique existe mais la portée de visibilité ne réduit pas la lisibilité tactique (aucune atténuation des étendards lointains). `grep` : `lightning/thunder` absents.
- Preuve : `battle/decor/precipitation.wesl`, `data/fx/battle_weather_fx.json`, aucune occurrence de `lightning`.
- Gravité : mineur.
- Correctif : (a) petite texture d'anneaux ou pastilles qui apparaissent sur le sol près de la caméra (même box, shader seulement) ; (b) mouiller le sol : assombrir/éclaircir le spéculaire via `decor/ground.wesl` selon `rain` ; (c) éclair optionnel = pic d'exposition 0,1 s dans `grade.wesl`.
- Effort : M

### 12. Pas de preuve de couverture clips/état en test
- Problème : `anim.rs` mappe famille × état → liste de clips avec `.fallback()`. Rien ne garantit qu'un clip nommé existe dans le manifeste cuit : un nom absent retombe silencieusement (ou fige en T-pose/idle). Les nouvelles familles (`horse_bow`, `pike`, `militia`) ont beaucoup d'états ; un manque ne se voit qu'à l'écran.
- Preuve : `units/anim.rs:82-170` (table), `game/assets/models/battle_skinned/manifest_merged.json`.
- Gravité : majeur (régression silencieuse).
- Correctif : test unitaire qui charge `manifest_merged.json` et vérifie, pour chaque (famille, état), qu'au moins un clip de `names` ou de `fallback` existe pour chaque rig ; échec nommé sinon.
- Effort : S

## Top 3

1. **Constat 6 (mêlée désorientée, coups dans le vide)** : plus grand gain de lisibilité et de « vie » ; côté rendu seulement.
2. **Constat 12 (test de couverture clips/état)** : peu coûteux, évite les régressions silencieuses sur toutes les familles.
3. **Constat 7 (impacts : étincelles/éclats)** puis **3 (mottes de sabots)** : retour visuel de la violence et de la charge, absent en sol mouillé. Le constat 1 (doc périmée) est un correctif S à faire en passant.
