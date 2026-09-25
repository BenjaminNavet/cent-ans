# FG0 — Prototype et planche de style (figurines fines)

Branche : `feat/fg0-prototype` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.

## État
- [x] Base humaine : MakeHuman via MPFB 2.0.17 (données CC0, code GPL hors dépôt) ;
  `fg_makehuman_base.py` -> `makehuman_base/fg_base_male.blend`
- [x] Ajustement au rig `human` : membres joint à joint (clavicules, bras, jambes), tronc/tête
  en similitude ; poids MPFB `game_engine` renommés + lissage épaules ; poings fermés ; testé
  marche, estoc, arc long, mort (`battle_fine_proto.py -- fit`)
- [x] Cheval : « Rigged Horse » (Lyndon Daniels, OpenGameArt, CC0, textures 2k) ; similitude,
  encolure relevée, déformation RBF sur les articulations du cheval Quaternius, poids par
  chaleur des os (copie du squelette aux os prolongés) ; testé pas, galop, mort
  (`-- horse`). Limite : jarrets/paturons tordus au galop (poids à reprendre en FG4).
- [x] Homme d'armes (`-- infantry`) : coques du corps (haubert, surcot, chausses), jupe
  fendue, bassinet, camail drapé, épée, écu, ceinture, fourreau, souliers ; ≈ 12,3 k triangles.
- [ ] Chevalier monté (cavalier fin sur `cavalry`, caparaçon fin)
- [ ] Cuisson test normal map + ORM + masque de livrée
- [ ] Planche `docs/img/fg/planche_fg0.png` + coûts (LOD, mémoire, `--units=50`)

## Prochaine étape
Cavalier (même corps sur l'armature cavalier de `Mount`), caparaçon fin ; puis cuisson.
