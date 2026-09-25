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
