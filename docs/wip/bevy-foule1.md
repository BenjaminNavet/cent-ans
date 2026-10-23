# foule1 : rendu de masse des soldats (L1 habillage allégé, L2 culling par soldat)

Réglages : `data/fx/battle_crowd.json` (schéma `data/schemas/fx_battle_crowd.schema.json`), code `client/src/battle/units/crowd.rs`.

## Fait
- **L2 culling par soldat** (`cull.enabled`, défaut oui) : `update_regiments` classe chaque soldat (frustum de la caméra de la frame, calculé depuis son `Transform` après `apply_camera`, + marge ; sinon « ombre seule » si le segment pied -> bout d'ombre (direction du soleil `BattleSun`) touche le frustum élargi ; sinon écarté). Le tampon d'instances est compacté : visibles d'abord, puis « ombre seule ». `count` = tous les enregistrements, composant `SkinnedColourCount(n)` = préfixe dessiné en couleur ; les cascades dessinent `count`. Imposteurs : préfixe visible seulement. Les `id` de soldat sont dans l'enregistrement : variantes/hash inchangés.
- **L1 habillage allégé des ombres** (`shadow_skin.enabled`, défaut oui) : le pipeline d'ombres reçoit des shader defs (`SHADOW_SINGLE_FRAME` image la plus proche au lieu de 2, `SHADOW_BONE_CUT` os de poids < 0,15 ignorés, `SHADOW_NO_FADE` pas de fondu clip précédent / échange, `SHADOW_NO_PROP` pas de lance procédurale). Chaque partie a son interrupteur dans le JSON. Le pipeline d'ombres était déjà distinct (fragment None, même vertex) : pas de nouveau shader, seulement des defs.
- **LOD le plus bas en couleur** (`far_skin.enabled`, défaut NON) : `LITE_FAR`, mêmes allègements pour `far_blend > 0`. Non vérifié visuellement : à décider par la session principale.
- Dump `battle_units` : `soldiers_drawn` (dans le frustum) / `soldiers_total` (+ `soldiers_drawn_over_total`, `soldiers_shadow_only`, `soldiers_uploaded`, `crowd.{cull,shadow_skin,far_skin,sun_travel}`). NB : `soldiers_drawn` valait avant le total envoyé au GPU.

## Crochets de dev (A/B)
- `CENT_ANS_CROWD=off` ; `CENT_ANS_CROWD_CULL|_SHADOW_SKIN|_FAR_SKIN=0|1` ; `CENT_ANS_CROWD_SHADOW_PARTS=single_frame,bone_cut,no_fade,no_prop`.
- `CENT_ANS_FIXED_DT_MS=16.7` : horloge à pas fixe (reproductible).
- `CENT_ANS_CROWD_FLIP_EVERY=100` : toutes les N images, bascule optimisations coupées/actives dans le même processus (dump `crowd.baseline_phase`). C'est la seule comparaison fiable : deux processus distincts diffèrent par du bruit de rendu (terrain/herbe, ~2 à 9 de différence moyenne selon l'état), bien au-dessus de l'effet mesuré.

## Vérifications (voir rapport)
- Tests unitaires de `crowd.rs` (frustum + marge, cas limites, ombres, tri/compte).
- Captures A/B dans le même processus : soldats, ombres, animations identiques à l'oeil ; différence moyenne 1,56 (image entière, 0-255) pour optimisé vs référence, contre 1,91 pour deux images de référence du même état (plancher de bruit).

## Non mesuré
- Gain en ms (banc release `--bench-battle` par la session principale).
- Cadavres (`corpses.rs`), armes jetées, `siege/crew.rs`, campagne : non concernés par le culling (lots séparés).
