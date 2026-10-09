# Lighting artist — état des lieux (09/10, lecture seule)

## 1. État actuel
**Campagne** (`scenes/campaign_map.tscn`) : AgX blanc 6, SSAO r 3 / 1,6, brouillard de profondeur 800-4000. Soleil par saison (`data/fx/atmosphere.json` : 18-21°, az 240-250°, énergie 1,0-1,55), LUT par saison (0,85), HDRI Poly Haven. RV-B (`campaign_lighting.json`, `visual/campaign_lighting.gd`) : ambiance froide × 0,56, exposition 0,98, variation par tour ±3°/±15°, transitions 8 s/5 s, perspective aérienne selon l'inclinaison. `turn_light.gd` (soir doré pendant le tour IA), `map_atmosphere.gd` (nuages, pluie, brouillard de fleuve, aurore), brume du sol ≤ 0,32 (ADR 0156), météo calculée par tour (ADR 0192). SSIL en Haute/Ultra ; SDFGI et volumétrique écartés de la carte.
**Bataille** (`scenes/battle/battle.tscn`) : 4 cascades jusqu'à 420, glow 0,35, brouillard exp. 0,00032. `battle_atmosphere.gd` : 4 préréglages météo **en dur** (`PRESETS`, `VOLUMETRIC_DENSITY`) + HDRI/LUT ; bloc `time_of_day` (PO4) ; `battle_time_of_day.gd` (EP8, images clés dans `battle_staging.json`, ciel procédural aube/soir/nuit). `render_quality.gd/json` : SSAO dès Moyenne, SSIL dès Haute, volumétrique (mauvais temps en Haute, toujours en Ultra), SDFGI Ultra bataille ; atlas d'ombres 8192.

## 2. Forces
- Piloté par données (JSON + schéma + tests, LUT générées).
- AgX partout ; relief protégé (ombre froide, soleil chaud, pas de matin à l'est sur la carte).
- Transitions interpolées ; coûts GPU arbitrés et mesurés (ADR 0156) ; effets plafonnés dès qu'ils délavent.

## 3. Faiblesses
1. Deux mondes de lumière : carte réglée à la main, bataille tirée du ciel (`HDRI_AMBIENT_BOOST` 1,35) ; exposition à 3 endroits ; soleil 1,5 vs 1,75 ; aucune garantie de continuité province → bataille.
2. Deux systèmes d'heure en bataille (PO4 + EP8) ; soleil du matin à l'est en bataille (voulu, mais orientation qui change).
3. Météo bataille en dur dans le code.
4. Images plates en bataille (`docs/img/po/apres/08-bataille-deploiement.jpg` : peu de contraste, voile gris) ; crépuscule EP8 brun monochrome, soleil trop gros.
5. Carte : sol olive uniforme en vue moyenne, mer très sombre, bord net du brouillard de guerre, LUT/HDRI qui sautent à la saison.
6. Coûts non confirmés (machine chargée ; SSAO carte jamais mesuré ; SDFGI Ultra non évalué).

## 4. Améliorations
| # | Action | Impact | Effort | GPU | Dépend de |
|---|---|---|---|---|---|
| 1 | Contrat de lumière commun carte/bataille par saison + test de luminance | Fort | M | nul | tech art, cœur |
| 2 | Contraste en bataille de jour (ombres, `aerial` 0,65→0,4, revoir ×1,35) ; 4 météos × 3 heures | Fort | S | nul | terrain |
| 3 | Fusionner PO4/EP8, sortir `PRESETS` en JSON | Moyen | M | nul | prog |
| 4 | Fondu LUT/ciel au changement de saison | Moyen | S-M | ~0 | — |
| 5 | Crépuscule/nuit : soleil plus petit, plancher d'ambiance, feux de camp | Moyen | S | nul | VFX |
| 6 | Bord du brouillard de guerre adouci | Moyen | S | nul | UI/carte |
| 7 | Banc de coûts sur machine calme, statuer sur SDFGI Ultra | Indirect | S | — | tech |
| 8 | Modelé de la carte en vue moyenne, mer et olive | Moyen | M | faible | terrain, DA |
| 9 | Météo de la province propagée à la bataille | Moyen | M | nul | cœur |

Commencer par 2, puis 1 et 3. Validation à l'œil à faire en session principale.
