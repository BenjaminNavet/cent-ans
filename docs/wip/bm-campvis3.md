# bm-campvis3 — sol de campagne de près (120-30)

## État
- Fusion de bm/campvis2 et bm/vegvis faite.
- Rust : table des matières du parcellaire (`fields.rs` : couche/tuile/classe de saison par culture, canopée, roche, matières sauvages, teinte), tableaux `tx_campaign_parcels` (albédo + normale, `ground.rs`), masque des terroirs (`terroir.rs`, port de `terroir_mask.gd`), liaisons 20-25 du matériau.
- Shader : à faire (prochaine étape).

## Prochaine étape
Porter dans `terrain.wesl` : `terroir_splat`, `hb_clearing_splat`, `terroir_landuse`, parcellaire V2b (`field_at`, haies, chemins), `fp_parcels`, échantillonnage des matières dans `fields_apply` (`hb_sample`), `season_field`.
