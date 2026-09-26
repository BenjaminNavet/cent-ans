# FG3 — Matières cuites des figurines fines

Branche : `feat/fg3-materials` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
Précédents : `fg0-prototype.md` (cuisson test), `fg1-corps.md`, `fg2-equipement.md`, `fg4-cheval.md`.

## État
- [ ] Squelette : commande `bake`, format `CAM2` (UV d'atlas), variante `FG3_BAKED` du shader,
  chargement des textures dans `BattleSkinned`
- [ ] Textures de détail tuilables (Texture2DArray) générées par script
- [ ] Atlas par figurine (LOD0 512², LOD1 256²) : normale de forme + AO + masque
- [ ] Cheval : textures CC0 réduites (1024²) partagées
- [ ] Shader : branches FG3 (atlas + détails selon le code matière), LOD2 sans lecture
- [ ] Retouches : barbes, mailles, casques martelés, teints
- [ ] Captures, tests, mémoire, banc

## Choix d'architecture
- UV d'atlas empaquetée dans UV2.y (11 bits u, 11 bits v, 2 bits source : 1 figurine,
  2 cheval) : aucun attribut de sommet en plus ; LOD2 : 0 (pas de lecture de texture).
- Repère tangent reconstruit au fragment (dérivées écran) : pas de tangentes dans le maillage.
- Variante du shader `#define FG3_BAKED` (comme `BV2_CORPSE`) posée par
  `BattleSkinned.setup_material` pour les figurines fines cuites seulement : le rendu par
  défaut est compilé sans une ligne de FG3.

## Prochaine étape
Implémenter la génération des tuiles et la cuisson.
