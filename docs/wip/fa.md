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
- [ ] FA2 — Feu et fumée : `tools/cent_ans_tools/vfx_flipbooks.py` recode Flame02 et Cloud02
      (Unity Labs, CC0) au format des planches du lot V3 (aucun shader modifié), réglages
      `data/fx/fire_flipbooks.json`. Planches cuites et commitées, tests pytest verts.
      **Reste** : jugement en jeu. `s2_fire_shot.gd` ne cadre plus de ville en feu (mise en
      scène obsolète) → agent visuel dédié, worktree `../gp-fa-fx`, branche `feat/fa-fx`.
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
- [ ] FA4 — ADR, captures avant/après, fusion `--ff-only`.

## Sources retenues (licence vérifiée)
- ambientCG (Lennart Demes), CC0 1.0 : `https://ambientcg.com/get?file=<id>_2K-JPG.zip`.
- Unity Labs Paris, flipbooks VFX, CC0 (billet
  `https://blog.unity.com/technology/free-vfx-image-sequences-flipbooks`, pas de fichier de
  licence dans les zips) : `https://unity3d.com/files/labs/downloads/vfx/assets01/<Nom>/<Nom>-flipbooks.zip`.
- Mesh2Motion (CC0, dépôt officiel GitHub) et KayKit Character Animations 1.1 (Kay Lousberg,
  CC0 ; récupéré d'un miroir GitHub tiers, licence citée depuis kaylousberg.com).
- Kenney Particle Pack / Smoke Particles, CC0 (sprites stylisés : secours seulement).

## Captures lues (budget 6)
6 / 6 — budget atteint (planche avant, essences été/hiver, A/B d'essences ×3, incendie raté).
Toute nouvelle vérification visuelle passe par un agent visuel dédié.

## Journal
- 10-02 : worktree, état des lieux, recherche effets terminée, recherche animations lancée.
- 10-02 : FA1 commité ; FA2 cuit, agent visuel lancé ; FA3, FA5, FA6 lancés en parallèle (4
  agents FA + 4 agents TB = 8 sur 10). pytest sur `feat/fa` : 1476 OK, 4 échecs hors FA
  (`test_water_detail`, `test_entity_icons` aussi sur main ; `test_ink_icons`,
  `test_relief_update` propres au worktree).
