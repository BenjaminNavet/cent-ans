# VQ cground — sol de campagne de près

## Diagnostic (17/10)
- Le port Bevy contient déjà tout le pipeline Godot : parcelles Voronoï (`fp_parcels`), champs peints (`fields_apply`, pack `tx_campaign_parcels`), terroirs, saisons par culture, macro GA4, grain micro. L'écart « aplats » de `bm-fidelity.md` n'est pas une absence de code.
- Causes du sol plat : (1) `hb_relative` borne chaque parcelle à ±15 % de luminance (`lum_band`) et garde 85 % de la teinte de la colormap : tous les champs ressemblent à la colormap ; (2) à d = 25..200 un pixel couvre 15..300 m (empreinte 0,02..0,4 px carte), donc ni sillons ni grain sous le mètre ne sont lisibles (les mips les effacent) : seule la structure de 100 m à 3 km compte (parcelles, ton, humidité) ; (3) les ombres de nuages dominent l'écart-type.

## Fait
- `ground_detail.wesl` (module importé par `terrain.wesl`, 3 points d'appel : `gd_macro` après `fields_apply`, `gd_parcel_k` dans `fields_apply`, facteur appliqué APRES `hb_relative`) + `ground_detail.rs` (uniforme binding 40, lecture des données, synchro niveau).
- Contenu : dérive macro de sol (ton + chaud/froid, 2-20 km) ; par parcelle : contraste de luminance, saturation, stade de culture par saison (vert frais / or pâle / brun labouré, parts par saison dans les données) ; niveau 2 : lanières (38-95 m) avec balks et lanières labourées, mouchetures 20-200 m.
- Données : `data/fx/ground_detail.json` (schéma `ground_detail.schema.json`), bouton `ground_detail` (0/1/2) par préréglage dans `render_quality.json` (low 0, medium 1, high 2, ultra 2, legacy 0), surcharge `CENT_ANS_GROUND_DETAIL=0|1|2`.

## Mesures
- Écart-type de luminance du sol (zone basse de la capture, Bevy release hors écran) : d=40 0,0714 -> 0,0829 (+16 %), d=100 0,0683 -> 0,080 (+17 %). Cible Godot (~23 contre 14, x1,6) non atteinte : les ombres de nuages et la colormap dominent encore.
- A/B alterné 3 paires, banc carte d=40 pan 20 s, machine à charge 125 (bruit) : p50 38,3 -> 39,5 ms, p95 76,0 -> 78,3 ms (dans le bruit ; coût shader : quelques bruits de valeur par fragment, pas de texture). À remesurer sur machine calme.

## Reste
- Lanières invisibles au-delà de d~20 (pixel > 40 m) : utiles seulement au plancher caméra ; pas de sillons (sous-pixel partout dans la plage demandée).
- Ombrage de relief du sol, micro-détail « qui survit aux mips » : non faits (rien de lisible à 15-300 m/pixel) ; piste : normales de parcelle plus fortes, occlusion de haies.
- Cible d'écart-type : monter `luma_contrast` / `tints` dans le JSON après regard du joueur ; tests clippy/cargo complets laissés à l'agent de fusion si non verts ici.
