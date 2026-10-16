# bm-lifeanim2 : restes de lifeanim (client Bevy)

Branche `bm/lifeanim2`. Suite de `bm-lifeanim.md`.

## Fait
- Ombres : `life/anim_pose.wesl` (module partagé : `AnimParams`, `pose_local`) importé par `anim.wesl` (passe principale) et `anim_shadow.wesl` (`prepass_vertex_shader` de `AnimExtension`) ; les ombres des bêtes, charrettes et troupeaux suivent les sommets déplacés.
- Convention de lacet : `frame.x` = `yaw_deg` de la table (rotation droite autour de +Y, comme `Basis(UP, a)` de Godot et `Quat::from_rotation_y`). Test `anim::tests::yaw_convention_matches_a_known_matrix` (matrice connue, +90° envoie +X sur -Z et +Z sur +X ; chaîne `prop_model` → `prop_bind` → shader). Pas d'inversion à corriger.
- Chevaux du camp de bataille : `battle/decor/camp_horse.rs` + `camp_horse_pose.wesl`, `camp_horse.wesl`, `camp_horse_shadow.wesl` (port de `camp_horse.gdshader` ; réglages `camp_horse` de `camp_horse_motion.json`, interrupteur `enabled` de `animal_motion.json`, crins = matière `Hair*`). `site.rs` pose `CampHorse` sur les scènes `horse_*` du kit. Section de vidage `battle_decor.camp_horses`.
- Fondu tramé des villageois skinnés : `record_of(.., fade)` écrit `far_blend < 0` (= -(1 - part visible)), `skinned.wesl` rejette les pixels par matrice de Bayer 4x4 (comme `fk_dither`) ; la figure garde sa taille (l'échelle de la racine `Fade` est divisée).
- Battement d'ailes : `AnimKind::Bird` (mode 2 du shader, `bird_pose` : `y += |x| * flap`, repli `wing_fold`), glb tourné par `yaw_deg` ; le roulis du corps est supprimé ; `wing_fold` lu dans `map_birds.json` ; pas d'ombre portée (Godot : `shadows_disabled`).

## Restes
- Les chevaux du camp ne sont pas instanciés (une scène par cheval, Godot : un MultiMesh par modèle) ; portée de visibilité `HORSE_RANGE` non appliquée.
- Oiseaux : triangles d'ailes de repli (espèces sans glb) toujours animés par rotation CPU ; couleur unie / `unshaded` de Godot non reproduite pour les glb (matériau du glb conservé).
- Fondu tramé : seulement les figurines skinnées de la carte ; les props non skinnés gardent le fondu par échelle (ADR 0354). Les ombres des figurines ne sont pas tramées.
- Yeux : non revérifié visuellement (pas de capture dans ce lot).
