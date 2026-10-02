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
- [ ] FA1 — Feuillages des arbres de bataille : vraies feuilles photographiées (ambientCG
      LeafSet, CC0), un rameau par essence. Données `data/art/battle_tree_leaves.json`, script
      `game/assets/textures/battle/build_fa_leaf_sprays.py`, `BattleTrees`.
- [ ] FA2 — Feu et fumée : séquences Unity Labs « VFX image sequences & flipbooks » (CC0) à la
      place des planches synthétiques. Brutes : scratchpad de session puis
      `~/dev/cent-ans-raw/fa/vfx/`.
- [ ] FA3 — Animations : sources redistribuables (recherche en cours : KayKit, Mesh2Motion,
      Quaternius UAL par navigateur), puis reciblage sur le rig fin (chaîne NT12).
- [ ] FA4 — Crédits, ADR, captures avant/après, fusion `--ff-only`.

## Sources retenues (licence vérifiée)
- ambientCG (Lennart Demes), CC0 1.0 : `https://ambientcg.com/get?file=<id>_2K-JPG.zip`.
- Unity Labs Paris, flipbooks VFX, CC0 (billet
  `https://blog.unity.com/technology/free-vfx-image-sequences-flipbooks`, pas de fichier de
  licence dans les zips) : `https://unity3d.com/files/labs/downloads/vfx/assets01/<Nom>/<Nom>-flipbooks.zip`.
- Kenney Particle Pack / Smoke Particles, CC0 (sprites stylisés : secours seulement).

## Captures lues (budget 6)
1 / 6 (planche avant : essences + deux vues de bataille).

## Journal
- 10-02 : worktree, état des lieux, recherche effets terminée, recherche animations lancée.
