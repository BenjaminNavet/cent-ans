# bm-campvis3 — sol de campagne de près (120-30)

## État : terminé
- Fusion de bm/campvis2 et bm/vegvis.
- Rust : table des matières du parcellaire (`terrain/fields.rs` : couche/tuile/classe de saison par culture, canopée, roche, matières sauvages, teinte), tableaux `tx_campaign_parcels` (albédo sRGB + normale, `terrain/ground.rs`), masque des terroirs (`terrain/terroir.rs`, port de `terroir_mask.gd` : 3894 finages, 0,3-0,8 ms), liaisons 20-25 du matériau (`terrain/material.rs`).
- Shader `terrain/terrain.wesl` : `terroir_splat`, `terroir_landuse` (`LanduseTexture`), `hb_clearing_splat`, parcellaire V2b (`field_at`, haies, chemins, balks, vignes), `fp_parcels`/`fp_splat`, `fields_apply` = port de `hb_apply` avec échantillonnage des matières (`hb_sample`, normales), `season_field`. Empreinte de `fields_apply` = dérivée écran comme Godot (`max(|dFdx p|, |dFdy p|)`).
- Arbres : `shade_foliage` (`vegetation/billboard.wesl`) multiplie le terme solaire par l'ombre portée (cascade du `CampaignSun`, biais selon la direction du soleil : pas d'auto-ombrage des cartes). Vosges 40 : 5 % des pixels assombris, moyenne inchangée.
- `political_flat_gain` : sans effet dans Godot (`political_amount` = 0 par défaut, jamais réglé ; ADR 0175 garde la couche telle quelle) : rien à porter, `fill.wesl` non modifié (déjà `colorized`).

## Mesures (Beauce 1.6,48.25, été, clair, sans brume ; zone 480×370)
| vue | Bevy avant | Bevy après | Godot |
|---|---|---|---|
| 120 | 97,94,64 lstd 9,8 | 94,91,62 lstd 9,9 hf 2,9 | 100,87,56 lstd 17,3 hf 3,3 |
| 60 | 96,94,65 lstd 11,1 | 91,90,62 lstd 12,0 hf 3,3 | — |
| 30 | 96,95,65 lstd 7,4 hf 1,9 | 93,92,64 lstd 9,4 hf 2,6 | 77,69,46 lstd 22,3 hf 2,8 |

Le grain (hf) rejoint Godot ; l'écart de lstd vient surtout des bouquets d'arbres et de leurs ombres (végétation, hors lot) et d'une couleur de base plus verte (Godot plus doré) : bande de luminance ±15 % autour de la colormap, fidèle à Godot.

## Trous gris autour de Paris (demande towns2)
Non reproduits côté terrain : 4 caps à 25 (2.35,48.85), `missing_pages` 0, `settled` dès 40 images. Avec la météo par défaut, la province de Paris est au tour 0 en **brouillard** (`focus_kind fog`, intensité 0,92, `morning_mist` 0,96, `fog_banks` 220, brume grise 0,8, densité 0,02) : le voile gris et les taches claires sont la météo, pas le sol. Avec `CENT_ANS_WEATHER=clear CENT_ANS_NO_FOG=1`, sol complet.

## Restes
- Saisons hors été : la teinte de saison peut s'appliquer deux fois (colormap + `season_field`) ; à vérifier en automne/hiver.
- Canal « brûlé » du masque des terroirs non dessiné ; masque construit une fois par campagne (pas de mise à jour de la dévastation en cours de partie).
- `fp_parcels` : prés de fond de vallée et finages 1:1 des villes non portés.
- Couleur de base de près plus verte que Godot (à comparer sur une référence Godot sans arbres).
