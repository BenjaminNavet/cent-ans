# 0307 — Loyauté : valeur de départ, rançon refusée, titre donné à un pair

Statut : accepté

## Contexte
Restes du lot WH `charsb` (ADR 0284) : la loyauté démarrait à 100 pour tous (bonus de moral de +3 aux généraux pendant les deux
premières saisons, sans distinction) et rien ne la faisait baisser en dehors de la dérive vers la cible. Les 12 compétences de rôle
n'avaient pas d'icône.

## Décision
- **Départ** (`data/rules/loyalty.json`, `loyalty::initial_loyalty`, appelé à la fin de l'initialisation 1337 et par `CharacterState::new`) :
  `start_base` (68) + `start_kin_bonus` (10, même maison que le souverain) + effets `loyalty` des traits × `effect_weight`
  ± `start_jitter` (5, hachage stable de l'identifiant, sans toucher au flux aléatoire) − `start_ambition_malus` (10, trait `trait_ambitious`).
  Le défaut serde des sauvegardes reste 100 : aucune sauvegarde existante ne change.
- **Rançon refusée** : chaque saison, un captif non souverain, en termes d'argent, dont le seigneur a en caisse au moins le montant
  de sa rançon sans la payer, perd `ransom_refused_loss` (4). Un seigneur sans le sou n'est pas blâmé ; les termes `Hold`/province
  (choix du geôlier) ne comptent pas. Journal discret (joueur seul) au franchissement du seuil d'alerte.
- **Titre donné à un pair** : à chaque `grant_title`, les généraux et gouverneurs (ni captifs, ni souverain, ni bénéficiaire) du donateur
  dont le prestige est au moins celui du bénéficiaire moins `rival_prestige_gap` (30) perdent `rival_loss` (8), `rival_ambition_loss` (4)
  de plus s'ils sont ambitieux. Même règle de journal.
- **Icônes** : les 12 compétences `requires_role` reçoivent des dessins game-icons déjà dans le cache/catalogue (`icons_catalog.py`,
  `icons.json` régénéré hors-ligne), 0 $.

## Conséquences
- Le bonus de moral de loyauté n'est plus universel au début ; il se mérite (traits loyaux, ordre de chevalerie, maison du souverain).
- Les captifs peu appréciés et les généraux écartés d'un fief se rapprochent de la défection : l'équilibrage se règle dans les données.
