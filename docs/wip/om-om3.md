# OM3 — terrains, climats et religions de l'Est (ADR 0116)

## État
- [x] `Terrain::Steppe`, `Terrain::Desert`, `Climate::Arid`, `Climate::Steppe` ; compilation verte.
- [x] Bataille : profils de décor `steppe`/`desert`, largeur de rivière, relief de plaine, bois/villages.
- [x] Météo de campagne : climats `arid` et `steppe` (données).
- [ ] Mouvement, ravitaillement/fourrage, attrition, bataille (auto-résolution), économie.
- [ ] Teinte du sol de bataille (données) côté Godot.
- [ ] Libellés FR Godot (infobulles, panneau, codex, filtres).
- [ ] Religions (orthodoxe/arménienne/païenne) : revue religion.rs, UI.
- [ ] Tests Rust ciblés, Godot.

## Prochaine étape
Parcourir sim-campaign (movement, supply, attrition, auto_resolve) : tables par terrain dans data/rules.
