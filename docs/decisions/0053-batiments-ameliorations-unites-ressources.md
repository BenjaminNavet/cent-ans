# 0053 — Bâtiments : améliorations sans régression, `enables_units`, ressources de chantier

Date : 2026-09-25 — lot B7c (vague 3 des bulles).

## Contexte
L'audit B4 a relevé des incohérences entre données, core et interface : une amélioration effaçait
des effets (boulevard −2 fortifications, maison des métiers sans le commerce du marché, arsenal),
et bloquait les bâtiments qui exigeaient le niveau remplacé ; `enables_units` n'était qu'affiché
tandis que six unités portaient un `required_building` distinct ; le coût en pierre était affiché
sans être prélevé ; `satisfies_classes: []` valait « toutes les classes » dans le code.

## Décision
1. **Une amélioration porte le total de sa chaîne.** Règle vérifiée au chargement
   (`data_model::upgrade_regressions`) : chaque effet du niveau remplacé doit réapparaître avec la
   même cible et une valeur au moins aussi forte dans le même sens. Un prérequis est satisfait par le
   bâtiment ou l'une de ses améliorations (`GameData::has_building`, lot EQ2, repris à la fusion à la
   place de `buildings::provides`) : bâtiments requis, unités,
   compagnons, régimes. Au départ, une colonie ne garde que le plus haut niveau de chaque chaîne
   (`GameData::normalize_building_tiers`, lot EQ2, à la place de `buildings::drop_superseded`).
   Écartée : cumuler les effets de toute la chaîne dans le core — l'interface lit les JSON bruts et
   aurait affiché des effets partiels ; les données des chaînes religieuses et militaires étaient
   déjà écrites en totaux.
2. **`enables_units` est la seule source** du bâtiment exigé pour recruter ; `required_building`
   des unités est supprimé. Les listes sont réduites aux unités réellement exigeantes (maison des
   métiers, atelier d'engins) : l'IA ne bâtit pas de bâtiments militaires et aurait perdu ses
   milices, chevaliers et hommes d'armes.
3. **Ressources de chantier** : dans le modèle de flux existant (une unité par province productrice
   accessible, `goods`), un chantier réserve les unités qu'il utilise jusqu'à son achèvement ; le
   manque est importé à `base_price × resource_import_multiplier` livres, compté dans le coût
   (donc dans le choix de l'IA). Pas de blocage : la pierre est rare (12 provinces) et bloquer les
   châteaux anglais ou bourguignons aurait figé les fortifications.
4. **`satisfies_classes` obligatoire ; liste vide = aucune classe.**

## Conséquences
- Paris et les villes de départ ne cumulent plus marché + maison des métiers + foire ni église +
  cathédrale : légère baisse des effets de départ (recherche, piété).
- Les sauvegardes antérieures chargent (`Construction::paid`/`drawn` par défaut ; remboursement sur
  le coût en argent).

## Complément SV2 (2026-09-25) — ressources des unités
La même règle s'applique au recrutement : le `cost.resources` d'une unité (engins : bois, fer)
est tiré de l'offre libre de la faction (`free_supply`) à la commande et réservé par la recrue en
file (`QueuedRecruit::drawn`) jusqu'à son entrée en garnison ; le manque est importé à
`base_price × resource_import_multiplier` et ajouté au coût (`RecruitOption::import_cost`).
Refus seulement si le trésor ne couvre pas le coût total. Il n'existe pas d'annulation de
recrutement, donc pas de remboursement ; la perte de la colonie vide la file et libère la réserve.
L'IA prix ses recrues d'un même tour contre l'offre que ses recrues précédentes ont tirée
(`ai::campaign::reprice_recruits`).
