# VIS-R1 : ciel, lumière, air de la carte de campagne

Spec : `docs/audits/2026-10-29-visuel-campagne.md` lot R1 (section g). Worktree agent-aa544b4a9560eb05d.
PAUSE demandée par le joueur : rien n'a été compilé ni mesuré ; tout ce qui suit est écrit mais NON VÉRIFIÉ.

## Fait (non compilé, non testé)
- Caméra : `pitch_near_deg` 20 / `pitch_mid_deg` 28 à `pitch_mid_distance` 150 dans `data/ui/camera_feel.json` (+ schéma), `camera/rig.rs` (`pitch_deg` par morceaux, tests mis à jour + test d'horizon).
- Lumière : `data/fx/atmosphere.json` campaign.seasons (soleil 32-36°, énergie 1,8-2,4, teinte chaude), `data/fx/campaign_lighting.json` (ambiance 0,75/0,80/0,90, énergie 0,40/0,44, bornes 26-40), test `lighting::sun_is_at_least_1_8_times_the_sky`.
- Ciel : dôme dégradé `client/src/campaign/weather/dome.rs` + `dome.wesl` (câblé dans `weather/mod.rs`, dump `sky` dans `atmosphere::dump`), `data/fx/campaign_sky.json` + schéma `fx_campaign_sky.schema.json` (+ `tools/tests/test_schemas.py`). Dernière erreur de compilation connue corrigée par sed (`depth_write_enabled = Some(false)`, `let Some(mut material)`), pas revérifiée.
- Brume unique : `campaign_map.json` morning_mist.opacity 0 (plus de brume peinte de vallée dans terrain.wesl), `campaign_post.json` valley_mist.sea_share 1.0, aerial.clear_max 0.3 (+ code `post.rs`, schéma), couleurs aérienne alignées sur l'horizon du ciel.
- Palette : bloc `campaign` + cibles dans `data/fx/palette.json` (+ schéma), tests dans `client/src/palette.rs`.
- Core : `first_turn_clear` (règle `campaign_weather.json`, schéma, `compute_weather` tour 0 dégagé, test `cm2_weather`).

## Prochaine étape
1. `cc.sh check --features dev` (script scratchpad : source sccache-env.sh + heavy.sh cargo), corriger ; puis clippy + tests client (`shader_check`) ; core : `cargo test -p sim-campaign --test movement cm2_weather`, fmt, clippy.
2. Lancer les vues `camp_0060`, `camp_0025`, `camp_alps 0120`, `v17` (CENT_ANS_WEATHER=clear sauf v17) via run_bg.sh, mesurer avec `out/audit/visuel/stats.py` (critères 1-4), régler `gain`, énergies, aérien.
3. Banc pan d 30 Ultra A/B (critère 5). Prototype Atmosphere Bevy 0.20 : exige `Hdr` sur la caméra (la passe post de campagne tourne après tonemap en LDR) donc repli dôme retenu sauf mesure contraire.
4. ADR `docs/decisions/XXXX-vis-r1-ciel-et-lumiere-de-la-carte.md` (amende 0156, 0710, 0755), mise à jour des vues `tools/refshots/views.txt` si besoin.

## Points ouverts
- Hiérarchie de valeur de la bible (c) : labour (lum. 0,37) plus sombre que le pré (0,45), tuile plus sombre que le pré : le test ne vérifie que mer < forêt < pré < blé < chaume < neige.
- Pas d'aube rose (décision) ; nuages de saison du dôme non faits (cirrus/cumulus existants gardés).
