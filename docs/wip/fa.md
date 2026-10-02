# FA — assets libres pour la qualité visuelle (2026-10-02)

Mandat du joueur : « cherche des assets gratuits sur internet et utilise-les si pertinents pour
améliorer la qualité visuelle du jeu », autonomie pour télécharger et modifier ; ajout en cours de
route : « cherche aussi des animations ». Budget 0 $.

Branche `feat/fa`, worktree `../gp-fa`. Brutes hors dépôt : `~/dev/cent-ans-raw/fa/`.
Périmètre : batailles et effets. La carte de campagne appartient à la session TB (`feat/tb`) :
FA n'y touche pas.

## Consigne de reprise
> Lis ce fichier et `git log --oneline -10`, puis continue au premier lot non coché.

## État des lieux (ce qui reste procédural et qu'un asset libre bat)
- Arbres de bataille : rameaux `leaf_spray.png` dessinés (ovales), identiques pour toutes les
  essences → houppiers « en boules » de près (capture `da6_trees_shot`).
- Feu et fumée : planches `flame_flipbook.png` / `smoke_flipbook.png` synthétiques
  (`tools/cent_ans_tools/fire_flipbooks.py`).
- Animations : clips keyframés maison ; essai CMU (NT12) peu concluant
  (`docs/research/mocap-gratuite.md`).
- Déjà couvert par du CC0 (ne pas refaire) : sols, matières de bâtiments, ciels, figurines,
  polices, sons (voir `CREDITS.md`). Audit de départ : `docs/audit/a4-assets-libres.md`.

## Lots
- [x] FA1 — Feuillages des arbres de bataille : vraies feuilles photographiées (ambientCG
      LeafSet, CC0), un rameau par essence (`leaf_spray_<essence>.png`, 1024², mipmaps, BC7),
      `dead_leaves_oak.png`. Données `data/art/battle_tree_leaves.json`, script
      `game/assets/textures/battle/build_fa_leaf_sprays.py`, `BattleTrees.leaf_texture`,
      `--no-fa` pour l'A/B. Jugé sur captures : gain net de près (feuilles lisibles, houppiers
      plus fournis), neutre de loin. Piège : compression S3TC = reflets vert fluo sur les
      imposteurs → compression haute qualité.
- [x] FA2 — Feu et fumée : `tools/cent_ans_tools/vfx_flipbooks.py` recode **Flame03** et
      **WispySmoke03** (Unity Labs, CC0) au format des planches du lot V3 (aucun shader modifié),
      réglages `data/fx/fire_flipbooks.json`. Jugé en jeu (branche `feat/fa-fx`, 18 planches lues) :
      flammes nettement meilleures (brasier aux langues déchiquetées au lieu de trois larmes
      lisses), fumée meilleure (volutes irrégulières au lieu de boules), bombarde non dégradée.
      Flame02 (pied blanc carré, haut éteint) et Cloud02 (disque plein = boules opaques) écartés.
      Réglages ajoutés à l'outil : `heat_max`, `coverage_gamma` (flammes), `density_floor`,
      `density_gamma`, `edge_fade` (fumée). `density_floor` 0,04 retire le voile diffus que le
      gamma relevait en halo (alpha nul sur l'anneau à 90 % du rayon, test pytest).
      Bombarde : `game/tests/fa2_bombard_shot.gd` (âges de fumée fixes, graines fixées, gros
      plan et vue moyenne) ; planche `~/dev/cent-ans-raw/fa/fx-shots/fa2-bombarde-avant-apres.png`.
      Le grand disque pâle vu sur les captures de bombarde n'est pas la planche : c'est la
      traînée du boulet (`siege_assault_fx.gd`, `_trail_mat`, disque flou) qui s'empile à la
      bouche quand la simulation est en pause ; présent à l'identique avec l'ancienne planche. `data/fx/siege_fire.json` : flammes 3,5-7 m → 4,5-8,5 m.
      `s2_fire_shot.gd` réparé (caméra propre par-dessus les toits, interface écartée,
      `--flipbooks=<dossier>` pour l'A/B, aussi sur `sg3_siege_shot.gd` ; aide
      `game/tests/fx_flipbook_override.gd`). Planche avant/après :
      `~/dev/cent-ans-raw/fa/fx-shots/fa2-avant-apres.png`.
      **Non vérifié à l'image** : feu naval (`naval_fire.gd`), fumée de camp en campagne
      (`army_figures.gd`, périmètre TB) et feux de camp de bataille (source posée dans un siège
      invisible avec l'ancienne planche comme avec la nouvelle : mise en scène à revoir, pas un
      effet des planches). **Défaut restant** : fumée brun-rouge près des foyers (lueur et
      lumières ponctuelles du shader, antérieur à FA2) ; jeune fumée encore très sombre et dense
      juste au-dessus des flammes.
- [ ] FA3 — Animations : agent dans `../gp-fa-anim` (`feat/fa-anim`), reciblage Mesh2Motion +
      KayKit (CC0) sur le rig fin, drapeau `--fa-anim`, note `docs/wip/fa3-anim.md`. Brutes
      `~/dev/cent-ans-raw/fa/anim/`. La session principale juge les planches Blender puis décide
      des clips par défaut.
- [ ] FA5 — Interface : agent visuel dans `../gp-fa-ui` (`feat/fa-ui`), note
      `docs/wip/fa5-ui.md`. Lettrines et bordures réelles (Cleveland Museum of Art, CC0), sceaux
      de cire (Met Open Access), matières ambientCG / Poly Haven. Brutes
      `~/dev/cent-ans-raw/fa/ui/`. Pas de vrai parchemin tuilable CC0 trouvé.
- [ ] FA6 — Campagne, vue parchemin seulement (TB refond la carte 3D : TB3 villes, TB4 traces,
      TB5 mer et côtes, TB6 lumière) : agent visuel dans `../gp-fa-map` (`feat/fa-map`), note
      `docs/wip/fa6-parchemin.md`. Rose des vents, navires, monstres marins réels (Atlas catalan
      1375, Dürer ; domaine public) à la place des ornements dessinés de CM2. Brutes
      `~/dev/cent-ans-raw/fa/campaign/`.
      Non retenus pour l'instant : rochers photogrammétrés Poly Haven (rock_07/09,
      rock_face_01, mountainside ; à proposer à TB pour `rock_outcrops.yaml`), kits Kenney
      (cartoon), navire OGA-BY, eau Keith333 (CC BY, non carrée). Lacunes confirmées : pas de
      cogue ni de feuillus 3D CC0 téléchargeables sans compte (piste : Polyy.AI sur itch.io,
      Quaternius Ships, à récupérer par navigateur).
- [ ] FA7 — Herbe des batailles : agent visuel dans `../gp-fa-grass` (`feat/fa-grass`), note
      `docs/wip/fa7-herbe.md`. Constat sur capture rapprochée (`--closeup --terrain=forest`) :
      touffes identiques, raides, sombres, espacées (« plants d'aloès ») = défaut le plus net des
      batailles. Vrais brins ambientCG Foliage001-008 (CC0), atlas de plusieurs touffes,
      `--no-fa-grass` pour l'A/B.
- [ ] FA4 — ADR, captures avant/après, fusion `--ff-only`.

## Sources retenues (licence vérifiée)
- ambientCG (Lennart Demes), CC0 1.0 : `https://ambientcg.com/get?file=<id>_2K-JPG.zip`.
- Unity Labs Paris, flipbooks VFX, CC0 (billet
  `https://blog.unity.com/technology/free-vfx-image-sequences-flipbooks`, pas de fichier de
  licence dans les zips) : `https://unity3d.com/files/labs/downloads/vfx/assets01/<Nom>/<Nom>-flipbooks.zip`.
- Mesh2Motion (CC0, dépôt officiel GitHub) et KayKit Character Animations 1.1 (Kay Lousberg,
  CC0 ; récupéré d'un miroir GitHub tiers, licence citée depuis kaylousberg.com).
- Kenney Particle Pack / Smoke Particles, CC0 (sprites stylisés : secours seulement).

## Captures
Plafond levé par le joueur le 10-02 (« budget illimité de captures ») pour la session principale
et les agents visuels du chantier. Rester à ≤ 1280 px de large et assembler avant/après.

## Journal
- 10-02 : worktree, état des lieux, recherche effets terminée, recherche animations lancée.
- 10-02 : FA1 commité ; FA2 cuit, agent visuel lancé ; FA3, FA5, FA6 lancés en parallèle (4
  agents FA + 4 agents TB = 8 sur 10). pytest sur `feat/fa` : 1476 OK, 4 échecs hors FA
  (`test_water_detail`, `test_entity_icons` aussi sur main ; `test_ink_icons`,
  `test_relief_update` propres au worktree).
- 10-02 : FA1 vérifié en bataille (forêt, vue rapprochée et déploiement, avec et sans
  `--no-fa`) : lisières identiques de loin, aucune régression. FA7 lancé (5 agents FA + 4 TB).
- 10-02 : FA2 jugé et réglé en jeu (Flame03 + WispySmoke03), `feat/fa-fx`.
