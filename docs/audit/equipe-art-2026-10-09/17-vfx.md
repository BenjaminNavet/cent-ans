# VFX artist — état des lieux (09/10, lecture seule)

## 1. État actuel
**Bataille**
- `battle_effects.gd` (1 019 l.) : poussière teintée par le sol, mottes, gerbes et écume aux gués, bombardes (éclair, fumée, boulet, gerbe), traits en MultiMesh à balistique shader. Pools 10/6/8, `EFFECT_DISTANCE` 700 m.
- `battle_volleys.gd` (volées, traits fichés, flèches enflammées) ; `battle_blood.gd` + `battle_gore.gd` (réglage « Sang » 3 niveaux, `battle_gore.json`) ; `battle_smoke.gd` (feux, incendies, fumée qui s'attarde, `max_sources`).
- Sièges : `siege_fire_fx.gd` (flipbooks, OmniLight qui vacillent, ruines), `siege_assault_fx.gd`, `wall_collapse_fx.gd`, `siege_marks_fx.gd`, `siege_engines_fx.gd`.
- Météo : `battle_atmosphere.gd` (clair/brouillard/pluie/neige, volumétrique, `FogVolume`, GPUParticles 9 000/7 000, `ripple` sur l'eau).
- Shaders partagés `fire_*`, `fx_flipbook`, `fx_blackbody`, `fx_noise` ; planches Unity Labs CC0 via `vfx_flipbooks.py` (FA2, ADR 0164).

**Campagne** : `life_effects.gd` (cheminées, panaches, flammes, oiseaux, moulins) ; `campaign_weather_view.gd` (sol mouillé, neige, brume, éclairs) avec shader de particules maison ; `order_ripple.gd`, `reachable_bubble`, `faction_borders.gd` ; `water_glint`, `fleet_wake`, `folk_flood` (LR-18), vent partagé.

**Qualité** : `RenderQuality.scale_particles`.

## 2. Forces
- Effets dérivés du cœur Rust, déterministes.
- Coût maîtrisé (pools, MultiMesh, coupure à distance, plafonds).
- Réglages en données avec schémas ; cohérence carte/bataille (mêmes planches, panache continu SZ4) ; outils de capture A/B.

## 3. Faiblesses
- Défauts connus (`docs/wip/fa.md` l. 53) : fumée brun-rouge près des foyers, jeune fumée trop sombre et dense.
- Jamais vérifiés à l'image : feux de camp de bataille, fumée de camp en campagne, crue LR-18.
- Météo de bataille pauvre : pas d'orage, d'éclaboussures, de flaques, de neige soufflée, de traces ; pluie en quads unis.
- Combat : pas d'étincelles d'impact, de distorsion de chaleur, de secousse aux bombardes/effondrements, de fumée de poudre en nappe ni d'armes à feu portatives.
- Poussière, éclats, gerbes d'eau = disques flous `StandardMaterial3D`.
- Pillage visible seulement par fumée + icône.
- **Dette** : 10 scripts utilisent encore `ParticleProcessMaterial` (cause de la fuite PF1 corrigée en campagne seulement).
- Pluie de bataille sans profil de distance.

## 4. Améliorations
| # | Action | Impact | Effort | GPU | Dépend de |
|---|---|---|---|---|---|
| 1 | Corriger la fumée près du feu (`fire_smoke.gdshader`, `siege_fire.json`) | Fort | S | nul | éclairage |
| 2 | Capturer et valider feux de camp, bivouac, crue LR-18 | Moyen | S | nul | QA |
| 3 | Particules de bataille sur shaders maison (comme PF1) | Stabilité | M | neutre | moteur |
| 4 | Étincelles d'impact en mêlée, déclenchées par les pertes du cœur | Fort | S-M | faible | sim, audio |
| 5 | Orage, éclaboussures, sols mouillés brillants en bataille | Fort | M | faible-moyen | météo, audio |
| 6 | Planches dédiées poussière, terre, eau | Moyen-fort | M | nul | DA |
| 7 | Secousse légère (option d'accessibilité) | Moyen | S | nul | UX |
| 8 | Distorsion de chaleur (Ultra) | Moyen | M | moyen | rendu |
| 9 | Fumée de poudre en nappe | Moyen | M | moyen | game design |
| 10 | Pillage animé en campagne (ADR 0099) | Moyen | M | faible | carte |
| 11 | Neige soufflée, traces dans neige et boue | Moyen | M-L | moyen | sol |
| 12 | Onde d'ordre en bataille | Moyen | S | nul | UI |

Gains rapides : 1, 2, 4 ; traiter 3 avant tout nouvel émetteur.
