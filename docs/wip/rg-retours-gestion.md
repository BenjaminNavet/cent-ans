# RG — retours du joueur sur la gestion (2026-10-11)

Source : retour du joueur du 11/10 (17 points : fenêtres, recrutement, chantiers, diplomatie, commerce, déplacement, conseil…).
Audit préalable (4 agents Explore) résumé ci-dessous ; pointeurs détaillés dans chaque lot.

## État constaté (audit)
- Fenêtres : aucune classe commune ; toutes passent par `MapUI.register_panel` (`map_ui.gd:1167`) / `PanelStack.register`. Aucun redimensionnement.
- « Avis » = libellé par défaut des alertes sans `SHORT_LABELS` (trade, mission, royal_act, level_up, loyalty, captain) ; `HudController._on_alert_activated` n'a pas de branche → rien ou caméra seule. « Croisade » : caméra vers la cible, aucune fenêtre ; pas d'ordre « rejoindre » (on rejoint en déclarant la guerre au détenteur, ADR 0327).
- Recrutement : `Recruit`/`RecruitInto` liés à une colonie, l'armée doit être DANS la colonie ; aucun ordre de fusion côté joueur (`MergeArmies` existe, IA seule).
- Chantiers : coût payé d'avance, `CancelBuild` rembourse 50 % fixe (`buildings.rs:26`) ; file de 3 par colonie ; emplacements 6/4/3/3/2.
- Vue province : construction possible seulement dans la ville de la province ; fenêtre Colonies en lecture seule ; `idle` ≠ « peut construire » (ignore emplacements et file).
- Auto-résolution : moral, armure, expérience, général, techs, terrain, météo, gués, murs déjà pris en compte. **Inertes** : effets de général `army_experience`/`army_ranged` (trait vétéran, capitaine d'élite) non lus par `side_from_army`.
- Déplacement : grille cuite (`navgrid.png`, coûts terrain, routes ×0,75, cols forcés à coût montagne, grands fleuves infranchissables hors ponts/gués). Pas de route constructible, pas de navigation fluviale, expérience du général sans effet direct.
- Diplomatie : montants d'or en paliers fixes ; brouillon non remis à zéro après envoi ; aucun gain commercial chiffré avant signature ; bouton « Commerce » = calque des routes. Monnaie : livre tournois `₶`, mais `℔` dans trade_panel/faction_panel/edict_section.
- Économie : pas de fenêtre dédiée (panneau de faction) ; prévision du solde seulement dans l'infobulle ; `get_income_breakdown` exposé mais jamais appelé ; pas de prévision Troubles/Hommes/Prestige.
- Pas de spécialisation de colonie, pas d'arbre de construction, pas de conseil.

## Lots
Vague 1 (Sonnet, spécifiés) :
| Lot | Contenu | Statut |
|---|---|---|
| RG1 | Fenêtres redimensionnables (poignée générique) | FUSIONNÉ (e4e7b5155) ; coin seul, autres panneaux à largeur fixe à vérifier |
| RG2 | Alertes « Avis » cliquables + fenêtre Croisade (explication, rejoindre) | FUSIONNÉ (ff0e5fa90) ; « Rejoindre » non essayé en vraie partie |
| RG3 | Diplomatie : montant libre, remise à zéro, gain commercial/tour, nom de la monnaie, `₶` partout | FUSIONNÉ (e29f14094) ; SpinBox non vues à l'œil, tribut figé 8 saisons |
| RG4 | Annulation de chantier au prorata + avertissement | FUSIONNÉ (c57ba6d97) ; paramètres dans economy.json |
| RG5 | Icône « constructible » (Colonies + vue province) et construction depuis la vue province | FUSIONNÉ (c57ba6d97) ; placement du marteau et du compagnon à voir |
| RG6 | Auto-résolution : effets de général inertes branchés | à faire |
| RG7 | Recrutement depuis l'armée en territoire propre + fusion d'armées côté joueur | à faire |
| RG8 | Prévision Troubles/Hommes/Prestige/Solde au prochain tour + principales causes | FUSIONNÉ (3d5218d0c) ; ~150 ms par recalcul (cache par révision), debounce si saccade |

Vague 2 (conception, ADR) :
| Lot | Contenu | Statut |
|---|---|---|
| RG9 | Déplacement : bâtiment route, navigation fluviale, montagnes/cols, expérience du général, compétences d'armée | à faire |
| RG10 | Spécialisation des colonies (vocations, bonus de cohérence) | à faire |
| RG11 | Panneau Bâtiments refait + arbre de construction | à faire |
| RG12 | Menu Économie avec onglet Commerce (traités, échanges de ressources, nouvelles routes, change, monnaie) ; retire l'onglet commerce de la diplomatie | à faire |
| RG13 | Conseil (PRIORITÉ joueur) : charges, conseillers, salle 3D — ADR 0341 | 13b figures (45eb521a1) + 13c salle (6c773345d) FUSIONNÉS ; 13a cœur en cours ; table trop plate, tabouret disproportionné |
| RG14 | Icônes de ressources générées en local (Z-Image) | à faire |

## Prochaine étape
Lancer la vague 1 (worktrees `rg-N`), fusion ff-only dans main lot par lot.
