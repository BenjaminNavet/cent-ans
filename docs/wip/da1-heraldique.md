# DA1 — Armoiries des maisons et héraldique des figurines

Branche : `worktree-agent-addd87848de848c53` (worktree `.claude/worktrees/agent-addd87848de848c53`),
basée sur main + merge de `feat/da-direction-artistique` (bible DA). Bible :
`docs/design/2026-09-25-bible-da.md`. **En pause** (demande du joueur, 25/09).

## État exact
- [x] 1 données : `data/heraldry/houses.json` (51 maisons = toutes les valeurs `house` des
  personnages ; blason, description, source, certitude attesté/incertain/substitution, `arms_of`
  pour 13 maisons aux armes identiques à une faction, `factions`, `vassal_of` pour les bannerets ;
  `badges` = croix de livrée du commun France/Angleterre/Écosse/Bourgogne) + schéma
  `data/schemas/heraldry_houses.schema.json` (validé à la main, 0 erreur). Substitutions :
  Artevelde (Gand), Béhuchet (Le Mans simplifié), Le Bel (perron de Liège) ; incertain : Petrarca.
- [ ] 2 outil `heraldry.py` (rien de modifié encore)
- [ ] 3 chargement/UI  - [ ] 4 shader  - [ ] 5 captures/perf/ADR
- Worktree prêt pour Godot : dylibs copiées dans `game/bin/`, `--import` fait.

## Prochaine étape (plan arrêté)
1. Test pytest de schéma (`tools/tests/test_heraldry_houses.py`) : schéma valide, noms = maisons des
   personnages, `arms_of`/`vassal_of` = factions existantes.
2. `heraldry.py` : garder les écus de faction identiques à l'octet (comparer les
   `Image.tobytes()` sha256 avant/après ; extraire la finition de `render_shield` en
   `_finish_shield`). Ajouter une grammaire v2 réservée aux maisons (`parse_house_blazon`) :
   champ (`d'X`, `d'hermine`, `bandé/burelé/échiqueté/fuselé en bande d'X et d'Y`, `coupé`,
   `écartelé en sautoir`, `semé de fleurs de lis / billettes`), `écartelé : aux 1 et 4 … ; aux 2
   et 3 …` récursif ; pièces et meubles repérés par mot-clé dans l'ordre du texte, nombre = mot
   précédent, teinte = première teinte qui suit (`componé/échiquetée d'X et d'Y` = motif ; `vair`,
   `hermine` = motifs) ; mot-clé précédé de `en` = disposition, pas une pièce ; `chargé de` = meuble
   posé sur la pièce précédente (chef, lambel) ; `accompagnée de` = autour de la bande. Nouveaux
   meubles : orle, chef, franc-quartier, sautoir, chevrons, fasces, bâton, cotices, barres ondées,
   étoile, rose, tourteau, coussin, fusée, billette, dauphin, gonfanon, perron, lionceaux.
   Maisons `arms_of` : rendu du blason de la faction. Sortie `game/assets/heraldry/houses/<id>.png`
   + commande CLI `assets heraldry` étendue ; tests déterministes et « écus distincts ».
3. Godot : `game/scripts/ui/house_arms.gd` (lecture des données via `SoundBank.data_path`, nom →
   id, `house_of(character_id, sim)` : `sim.get_character` sinon `data/characters/<id>.json`,
   vassaux par `vassal_of`) ; `PortraitLoader.house_heraldry_texture(house)` avec repli faction ;
   brancher `character_sheet.gd` (écu + repli portrait), `court_panel.gd`, `family_tree_view.gd`,
   `general_seal.gd`, `battle_hud.gd` / `pre_battle_dialog.gd` (général).
4. Shader skinné : le code `C_ARMS` (écus, pavois, caparaçon, UV) existe déjà et reçoit les armes
   de faction ; ajouter `sampler2DArray arms_atlas` (Texture2DArray construite une fois par
   bataille : armes des 2 factions + maisons des généraux + vassaux, 128², mipmaps),
   `ivec4 arms_layers` (seigneur + 3 vassaux), `vassal_share = 0.3`, `lord_arms` (unité noble),
   choix par hachage de `v_var` (aucune donnée par instance, aucun coût CPU). Projection poitrine/dos
   sur `C_LIVERY` en position de repos : boîte mesurée (infantry_0 : x ±0.14, y 1.0-1.45, faces
   n.z > 0.35 avant / < -0.35 dos ; cavalier ≈ +0.33 m en y — calculer la boîte au chargement
   dans `BattleSkinned._load_mesh` à partir des sommets livrée des os Torso/Chest). Commun : croix
   de `badges`. Cadavres : même uniformes par camp. Shader rigide : texture `heraldry` = maison du
   général pour les unités nobles. Script d'inspection des maillages : relire le format `CAM1`
   (21 floats/sommet, code matière = couleur.a).
5. Captures `docs/img/da1/` (`--standard-shot=foot|mounted`, `--closeup`), A/B `--benchmark`
   (≤ 3 %), ADR suivant libre dans main (0055 pris au 25/09 → vérifier), mise à jour bible § 10.
