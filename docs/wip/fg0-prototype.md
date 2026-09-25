# FG0 — Prototype et planche de style (figurines fines)

Branche : `feat/fg0-prototype` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.

## État
- [x] Base humaine : MakeHuman via MPFB 2.0.17 (données CC0, code GPL hors dépôt) ; `fg_makehuman_base.py` -> `makehuman_base/fg_base_male.blend`
- [x] Ajustement au rig `human` : membres joint à joint (clavicules, bras, jambes), tronc/tête en similitude ; poids MPFB `game_engine` renommés + lissage épaules ; testé marche, estoc, arc long, mort (`battle_fine_proto.py -- fit`)
- [ ] Cheval : base CC0 ou remodelage du cheval Quaternius sur le rig `cavalry`
- [ ] Prototype homme d'armes (`infantry_0`) et chevalier (`cavalry_0`)
- [ ] Cuisson test normal map + masque de livrée
- [ ] Planche `docs/img/fg/planche_fg0.png` + coûts

## Prochaine étape
Équipement fin de l'homme d'armes (coques dérivées du corps : haubert, surcot + jupe, chausses ; bassinet + camail, épée, écu), puis cheval.
