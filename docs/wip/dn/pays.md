# DN-PAYS : campagne vivante hors champs (branche worktree-agent-a3f5b4976ca8a4548)

Périmètre : haies/clôtures (bocage), puits, croix de chemin et calvaires, moulins à vent, salines côtières, ruines, piloris, bord des routes (charrettes, chariots, caravanes). Les troupeaux (prairies/pâtures) existent déjà : `FaunaLayer` (DN-ME2) charge déjà les `dn/fauna/animal_*_lod*.glb` (le glb prime sur le `placeholder`), rien à remplacer.

## État
- [ ] Squelette : note, classe `CountrysideLayer`, données `data/map/map_countryside.json` + schéma.
- [ ] Couche + shader `countryside.gdshader` (un MultiMesh par prop et par cellule, taille tenue à l'écran, fondu par prop).
- [ ] Données : règles (bocage, puits, piloris, croix, moulins, salines, ruines, bord de route).
- [ ] Câblage `CampaignLife` (`--no-countryside`, `--life-off=countryside`).
- [ ] Charrettes FK (`FolkModels`) : remplacement des maillages `merchant_cart` / `peasant_cart` par les glb DN (table en données).
- [ ] Tests : `game/tests/dn_pays_test.gd`, `tools/tests/test_map_countryside_schema.py`, `smoke.gd`.
- [ ] Mesures de coût, captures (budget 5).

## Prochaine étape
Voir la case non cochée la plus haute.
