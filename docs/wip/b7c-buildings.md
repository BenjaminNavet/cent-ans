# B7c — bâtiments : incohérences données ↔ code ↔ interface

Branche `b7c-buildings` (worktree agent). Ne pas fusionner dans main (orchestrateur bulles vague 3).

## Décisions
1. **Améliorations sans régression** : une amélioration porte le total de sa chaîne. Le chargeur
   (`data-model`, `check_buildings`) refuse une amélioration qui perd ou affaiblit un effet du bâtiment
   remplacé (même `effect`/`mode`/`class`/`unit_category`, valeur au moins aussi forte dans le même
   sens). Données corrigées : maison des métiers ⊇ marché, foire ⊇ maison des métiers, boulevard
   ⊇ château. Un prérequis est satisfait par le bâtiment ou l'une de ses améliorations (« marché ou
   supérieur ») : `buildings::provides` (à la fusion : remplacé par `GameData::has_building` d'EQ2). Au départ 1337, les bâtiments d'une chaîne remplacés par une
   amélioration présente dans la même colonie sont retirés (`setup_1337`), pour ne pas cumuler (à la fusion : `normalize_building_tiers` d'EQ2 à la place de `drop_superseded`).
2. **`enables_units` seule source** : `UnitType::required_building` supprimé (schéma, struct,
   données) ; le core refuse le recrutement d'une unité listée par au moins un bâtiment si la colonie
   n'a aucun de ces bâtiments (ou une amélioration). Listes réduites aux unités réellement exigeantes
   (maison des métiers : piquiers flamands, milice au goedendag, couleuvriniers ; atelier d'engins :
   4 engins) pour ne pas priver l'IA (qui ne bâtit pas de bâtiments militaires) de ses unités de base.
3. **Piété/prestige** : déjà branchés (G1/F1, `dynasty::yearly_building_piety`,
   `yearly_court_prestige`, appliqués au souverain chaque hiver). Fiches Codex corrigées.
4. **Coût en ressources** : modèle des ressources = nombre de provinces productrices accessibles
   (`goods`). Une construction puise dans cet approvisionnement (réservé tant qu'elle dure,
   `Construction::drawn`) ; chaque unité manquante est importée à `base_price ×
   resource_import_multiplier` livres (`data/rules/economy.json`), ajoutée au coût (donc vue par l'IA).
   Remboursement d'annulation : 50 % de ce qui a été payé (`Construction::paid`).
5. **`satisfies_classes: []` = aucune classe** (pierre, fer : matériaux de construction et
   d'armement). Champ obligatoire dans le schéma.
6. **Normandie** : `res_stone` ajouté (pierre de Caen).

## État
- [x] 1 [x] 2 [x] 3 (vérifié) [x] 4 [x] 5 [x] 6 [x] UI tooltips/encyclopédie/mock [x] ADR 0053 ; cargo fmt/clippy/test verts
- [x] Codex (26 fiches, validateur vert) [x] build.sh + import + smoke Godot verts [x] pytest 515 verts

Lot terminé, en attente de fusion par l'orchestrateur (branche b7c-buildings).

## Prochaine étape
Aucune (fusion). Note : f1_effects `scholar_ruler…` : tolérance d'arrondi ±1 (points de recherche fractionnaires, baisse au départ due à drop_superseded).
