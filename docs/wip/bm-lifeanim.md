# bm-lifeanim : animation de la vie de la carte (client Bevy)

Dossier : `client/src/campaign/life/`. Branche `bm/lifeanim`. Point de départ : `docs/wip/bm-life.md`, `bm-lifeplus.md`.

## Fait
- `motion.rs` : `MotionSpec` complet de `animal_motion.json` (corps, cou, queue, roues, essieux, tables de pas de 16 échantillons, lignes de cahot mesurées) ; `camp_horse_motion.json` (`campaign_horse`) fusionné dans `horse` comme `AnimalMotion._merge_camp_horse_motion`. Courbes testées (`lut_at`, `wheel_angle`, `prop_pose` = copie CPU de référence du shader, `herd_clock`, étiquettes). `animal_motion_measured.json` n'est lu par aucun jeu à l'exécution (Godot non plus : ses valeurs sont cuites dans `animal_motion.json`) ; un test vérifie que les deux fichiers concordent.
- `anim.rs` + `anim.wesl` : matériau `ExtendedMaterial<StandardMaterial, AnimExtension>` à étage de sommets (port de `animal_motion.gdshaderinc` : pattes sur les tables mesurées, hochement / broutage / mâchonnement de la tête, queue, souffle, déhanchement, roues qui tournent avec la distance, cahot et roulis de la caisse ; et des allures de `fauna.gdshader` : pattes en diagonale, levée, rebond, tête basse à l'arrêt). Données par instance dans `MeshTag` (vitesse + graine, ou phase des pattes + « en marche »). Le modèle est tourné dans le repère de Godot (`yaw_deg` de la table) puis ramené. Les matériaux de la scène glb sont remplacés par `bind_models` (cache par matériau source et modèle).
- `fauna.rs` : le temps déformé de `fauna.gdshader` (arrêts pour brouter, reprise plus vive) conduit la trajectoire et la phase des pattes ; `write_herd_tags` écrit l'étiquette chaque image ; le bascule / tangage de repli du CPU ne sert plus que pour les modèles de remplacement (non DN).
- `folk.rs` / `motion.rs::animate_carts` : charrettes, bêtes attelées et charrues portent `AnimBind` (id DN, vitesse = `WALK_SPEED_M_S` x `folk_speed`) ; le cahot / bob CPU ne s'applique plus aux modèles liés au shader.
- `skin.rs` : villageois, ouvriers, charretiers, marchands, gardes, cavaliers = figurines skinnées de la bataille (lot instancié `battle::units::skinned`, texture d'os), une batch par (figure, rôle, activité) ; clips des activités (`activity_anims` et `folk_styles` ajoutés à `data/rules/folk_props.json` + schéma : table `ACTIVITIES` et styles `folk` / `folk_carry` de Godot), livrée 0,4 du rôle, tresse assombrie de 0,3, tissus `drab`, niveau fin sous `folk_detail_distance`. Le rebond de pas du CPU est supprimé.
- Dump `campaign_life.anim` : `props` (instances, liées au shader, vitesse, plage de graines), `binder`, `herds` (phase / en marche), `figures.animated` (rôle, activité, figure, clips), `pose_probe` (déplacements de sabot, tête, queue, jante, caisse sur 10 s).

## Preuve (CENT_ANS_CAMERA)
- Paris d=30 : 6 props liés (3 charrettes `env_cart_ox_peasant`, 3 boeufs `animal_ox_draught`, 0,6 m/s) ; 4 batchs de figurines (peasant walk 6, monk / pilgrim procession 7 + 7 `walk`, porter `carry` 2).
- Normandie d=60 : 190 bêtes, 187 conduites par le shader (phase 0-1, en marche 0,59 en moyenne), 96 props liés ; peasant walk 8, porter 2.
- `pose_probe` (CPU de référence) : sabot arrière du cheval z -0,22 .. +0,25 m, levée 0,096 m ; tête qui broute -0,56 m ; jante de roue parcourt le cercle (z -0,58..+0,58, y -1,16) ; caisse +-0,027 m.

## Écarts / reste
- Modèles FK2 de remplacement (hors paquet DN) : rigides (chemin CPU de secours).
- Les ombres des bêtes / charrettes ne suivent pas le déplacement des sommets (le prepass utilise le sommet de base de Bevy).
- Chevaux de camp de bataille (`camp_horse.gdshader`, `build_camp_horses`) : hors du dossier `life/` (lot battle) ; `camp_horse_motion.json` n'y est pas encore lu.
- Un batch par (figure, rôle, activité) : jusqu'à ~15 batchs simultanés, un `Transparent3d` chacun ; fondu de portée et éclaircie des figurines passent par l'échelle de l'ancre (Fade), pas par le fondu tramé de Godot.
- À revérifier à l'oeil : scènes FK4 `scythe` / `plough` jouent les clips `scythe`/`plough`/`carry` de `battle_fine` (présents dans `manifest_merged.json`) ; non revérifiés à l'oeil.
