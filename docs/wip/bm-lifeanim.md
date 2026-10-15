# bm-lifeanim : animation de la vie de la carte (client Bevy)

Dossier : `client/src/campaign/life/`. Branche `bm/lifeanim`. Point de départ : `docs/wip/bm-life.md`, `bm-lifeplus.md`.

## Plan
1. `motion.rs` : paramètres complets de `animal_motion.json` (+ `camp_horse_motion.json` fusionné dans `horse`) et courbes testées.
2. `anim.rs` + `anim.wesl` : matériau étendu à étage de sommets (port de `animal_motion.gdshaderinc` et des allures de `fauna.gdshader`) ; données par instance via `MeshTag`.
3. `skin.rs` : villageois, ouvriers, charretier, marchand, garde animés par le lot de figurines skinnées de la bataille (`battle::units::skinned`), clips d'activité (`ACTIVITIES` de `folk_models.gd`).
4. Section de dump `campaign_life.anim`.

## État
Squelette en cours.
