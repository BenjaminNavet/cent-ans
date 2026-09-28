# OM3 — terrains, climats et religions de l'Est (ADR 0116)

## État
- [x] `Terrain::Steppe`, `Terrain::Desert`, `Climate::Arid`, `Climate::Steppe` ; compilation verte.
- [x] Bataille : profils de décor `steppe`/`desert`, largeur de rivière, relief de plaine, bois/villages.
- [x] Météo de campagne : climats `arid` et `steppe` (données).
- [x] Mouvement (coûts steppe/desert dans movement/rules.json, navgrid.py : cases des provinces de désert), fourrage/attrition (`terrain_supply` d'economy.json), auto-résolution (auto_resolve.json).
- [ ] Teinte du sol de bataille (données) côté Godot.
- [ ] Libellés FR Godot (infobulles, panneau, codex, filtres).
- [ ] Religions (orthodoxe/arménienne/païenne) : revue religion.rs, UI.
- [ ] Tests Rust ciblés, Godot.

## Prochaine étape
cargo test complet ; tests ciblés ; teinte du sol Godot ; libellés FR ; religions.
