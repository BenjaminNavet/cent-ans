# DA — Direction artistique (orchestration)

Demande du joueur (25/09 soir) : « tu es le directeur artistique, que manque-t-il au jeu
comparé à Total War et Crusader Kings ? » puis « ok pour la recommandation ».
Bible : `docs/design/2026-09-25-bible-da.md` (à lire avant tout lot DA).
Branche d'orchestration : `feat/da-direction-artistique` (worktree `../gp-da`).
Budget propre : **15 $** (section « Direction artistique » de `docs/budget.md`).

## Lots

| Lot | Objet | Dépend de | Coût prévu | État |
|---|---|---|---|---|
| DA0 | Bible DA + captures d'état `docs/img/da/` | — | 0 $ | fait |
| DA1 | Armoiries des maisons + héraldique sur les figurines (surcot, écu, caparaçon ; les nobles portent les armes de leur seigneur) | DA0 | 0 $ (procédural) | vague 1 |
| DA2 | Portraits vivants : banque d'archétypes pour les personnages nés en jeu, vieillissement, marques (couronne, blessure, maladie, deuil), cadres par rang | DA0 | ≤ 8 $ | vague 1 |
| DA3 | Langage unique des marqueurs de ville sur la carte (forme = type, écu = propriétaire, taille = rang) | DA0, après ZG4b | 0 $ | vague 2 |
| DA4 | Musique : thèmes par culture, bataille en couches pilotée par l'intensité | DA0 | ≤ 3 $ | vague 2 |
| DA5 | Famille d'icônes d'action unique (trait d'encre) | DA0 | ≤ 3 $ | vague 2 |
| DA6 | Bataille rapprochée : herbe/cultures à lisière douce, arbres lointains | après EP6 | 0 $ | vague 3 |

Retirés de la recommandation initiale après inventaire : accessoires d'époque (pieux, pavois,
vignes : déjà faits en BV1 ou en cours dans EP6) ; « beauté de la carte » (déjà au niveau, réduit
à DA3 lisibilité).

## Journal

- 25/09 ~22 h : inventaire, captures à jour depuis `../gp-da` (l'arbre principal avait un cache
  de classes Godot périmé : `FineGeoLayer` introuvable ; import refait dans le worktree), bible
  écrite. Prochaine étape : lancer DA1 et DA2 (agents en worktree).
- 25/09 ~22 h 30 : vague 1 lancée — DA1 (héraldique, wip `da1-heraldique.md`) et DA2 (portraits
  vivants, ≤ 8 $, wip `da2-portraits-vivants.md`), agents en worktrees, chacun fusionne d'abord
  `feat/da-direction-artistique` pour la bible. L'orchestrateur fusionne (jamais les agents).
  Vague 2 (DA3 marqueurs après ZG4b, DA4 musique, DA5 icônes) après retour de la vague 1.
- 25/09 ~23 h : **PAUSE demandée par le joueur** ; reprise dans un autre terminal. DA1 et DA2
  ont reçu l'ordre de commiter un `wip:` et de s'arrêter. Reprise : `git worktree list | grep
  agent` pour retrouver leurs worktrees/branches, lire `docs/wip/da1-heraldique.md` et
  `docs/wip/da2-portraits-vivants.md` sur ces branches, vérifier la dépense DA2 dans
  `docs/budget.md` avant toute génération.
- 25/09 ~23 h : pause confirmée par les deux agents (rien fusionné dans main).
  - DA1 : branche `worktree-agent-addd87848de848c53` (worktree
    `.claude/worktrees/agent-addd87848de848c53`), commit `6fc9d9fc`. Fait : `data/heraldry/houses.json`
    (51 maisons, sources, `arms_of`, `vassal_of`, `badges`) + schéma. 3 substitutions (Artevelde,
    Béhuchet, Le Bel), Petrarca incertain. Reste : test du schéma, grammaire `heraldry.py`
    (écus de faction inchangés à l'octet), `house_heraldry_texture` + interface, atlas
    `sampler2DArray` dans le shader skinné, captures, A/B perf, ADR.
  - DA2 : branche `worktree-agent-ade9b18af852c5bfe` (worktree
    `.claude/worktrees/agent-ade9b18af852c5bfe`), commit `495470d9`, **0 $ dépensé**. Fait :
    `data/portraits/archetypes.json` + schéma (122 archétypes + 25 variantes âgées), générateur
    `portrait_archetypes.py` (`--dry-run` : 147 images ≈ 6,69 $), captures avant `docs/img/da2/`.
    Reste : pytest, sonde 3 images, génération, tout le GDScript, captures après, ADR.
