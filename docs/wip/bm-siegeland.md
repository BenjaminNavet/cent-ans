# bm-siegeland : toile de fond des villes, fondu des servants (client Bevy)

Branche `bm/siegeland` (base 7ec9a21cf, contient towns4). Dossier `client/src/battle/siege/` (fichiers neufs) + un bloc dans `units/skinned.wesl`.

## Fait
- `siege/backdrop.rs` + `backdrop.wesl` (Godot `landmark_backdrop.gd`, `landmark.gdshader`) : toile de fond des 7 villes (paris, london, rouen, calais, bordeaux, bruges, avignon ; cairo n'a pas de bloc `siege`). `models/landmarks/<id>_siege.glb` posé tourné de pi à `bank_z + bank_offset_m` (bank_z = muraille la plus lointaine + 40 m, `center_x_m`), relief de la bataille cuit en 192² `R32Float` (max centre + 4 coins, +0,3 m) lu par le vertex (drapé par ancrage = UV, bâtiments rigides), matériaux glb remplacés par `BackdropMaterial` (atlas `textures/landmarks` 16 couches, triplanaire en mètres, palette x détail, variation par bâtiment, salissures au pied, coulures, mousse, normales plates dérivées, eau `Water`). Hook dev `CENT_ANS_LANDMARK_BACKDROP=<id>` (= `--landmark-backdrop`). Section de vidage `battle_backdrop`.
- `siege/crew_fade.rs` + bloc mode 3 de `units/skinned.wesl` (Godot `siege_crew_fx.gd` fondu NT10) : le geste précédent d'un servant continue depuis son propre début pendant `role_blend_s` (0,25 s, `battle_animation.json`) pendant que le nouveau entre en fondu. Pas de mode CUSTOM côté Bevy : l'information (clip précédent, durées) est empaquetée dans le champ `volley_time` du record (inutilisé en mode Pick). `SiegeStats::crew_fading` dans la section `battle_siege`.
- Roues (`Wheel_<n>`) et balancement de la caisse du bélier / beffroi : déjà portés (`engines.rs::animate_mover`, b7aee2cf5) ; seul l'en-tête « Not ported » était périmé, corrigé.

## Écarts
- Le matériau du fond de scène ne projette pas d'ombre et ne passe pas par le prepass (comme les bannières) ; Godot ne coupait l'ombre que des maisons.
- Pas de `fade` ni de teinte ni de vues de débogage du shader (réservés aux maquettes de campagne).
- `battle_capture_stage.gd` n'est pas une cinématique de prise de point : c'est l'outil de captures `--screenshot` / `--closeup` / `--standard-shot` (arguments de ligne de commande, mise en scène avant capture). Hors périmètre sans captures ; voir le rapport. La prise de point elle-même est dans `overlay.rs` (siege_capture_points.gd).

## Restes
- `battle_capture_stage.gd` : à trancher (équivalent Bevy = `--script` / vidage, ou abandon).
- Ombre de la toile de fond (prepass + vertex drapé), si le joueur la veut.
- Porteurs de munitions (`haul`) des servants (déjà dans `crew.rs` « Not ported »).
