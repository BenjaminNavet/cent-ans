# HIST3 — corrections d'histoire, de cohérence et de texte (juge iter-04)

Source : `out/juge/iter-04-histux.md`. État : corrections faites, voir le rapport de lot.

## Constats et suites

| # | Constat | Verdict | Suite |
|---|---|---|---|
| 1 | Discours de Crécy : « la pluie mouille leurs cordes comme les nôtres » | Confirmé (les Génois ont été gênés, pas les archers anglais qui détendirent leurs arcs) | `data/speeches/battle_speeches.json` (`weather.rain`) : texte neutre, valable pour toute bataille sous la pluie |
| 2 | « Cour plénière » chez les comtes d'Empire | Infirmé en partie : acte royal générique (`act_cour_pleniere`), terme aussi employé hors Capétiens (cour plénière de Hainaut, Hoftag impérial) | Aucun changement de nom |
| 3 | Transylvanie propose une alliance à la France | Confirmé : `plan_alliances` (`diplomacy/ai.rs`) ignorait la suzeraineté | Un vassal (`suzerain` renseigné) ne cherche plus d'alliances ; test `vassals_never_offer_alliances` |
| 4 | « ft » / « livres » | Le HUD affiche `₶` (livre tournois, ADR 0346) ; le fil disait « livres » | Textes d'événements du fil (revenus, butin, présents, rançon) via `economy_balance::livres` (`₶`). Les messages d'erreur et les libellés d'ordres gardent « livres » (mot plein, lisible) |
| 5 | Fil tronqué par « … » | Confirmé | `shorten` coupe à la limite d'un mot ; le texte complet est déjà dans l'infobulle. Pas de retour à la ligne (non vérifiable sans lancer le client) |
| 6 | 10 provinces françaises, pas de Reims/Troyes/Rennes/Harfleur | Limite assumée de la carte | Voir ci-dessous. Aucune colonie ajoutée |

## Pourquoi aucune colonie n'est ajoutée (point 6)

- Reims et Troyes sont déjà couvertes par la province composée Île-de-France (Barrois, Champagne, Picardie, Valois, ses sources les citent), Harfleur par la Normandie, Rennes par la Bretagne.
- ADR 0291 : au plus 5 colonies par province (`data/map/settlement_cap_rules.json`). Île-de-France (Paris, Saint-Denis, Meaux, Amiens, Coucy) et Normandie (Rouen, Caen, Château-Gaillard, Cherbourg, Dieppe) sont pleines. Bretagne n'en a que 3.
- Toute colonie exige aussi des dérivés de la carte composée (`settlements_px`, `settlement_graph`, `settlement_edge_paths`, `towns_1340`, `fine_anchors`, `starting_fit`) : interdit par le périmètre du lot.

## Proposition d'ADR (sans code)

Titre : « Lieux de mémoire hors plafond de colonies ».
- Contexte : Reims (sacre), Troyes (traité de 1420, foires), Rennes (capitale bretonne), Harfleur (siège de 1415) portent des événements et des actes mais ne sont pas des lieux sur la carte.
- Option A, la moins chère : une couche de « lieux historiques » purement informative (`data/landmarks_v2`, pas des colonies, sans garnison ni économie), cliquable et liée au codex et aux actes (`act_sacre_reims` ciblerait Reims).
- Option B : relever `max_per_province` à 6 pour les seules provinces composées (IdF, Normandie) et régénérer `cent-ans geo settlements` ; coût : équilibrage (garnisons, entretien, `starting_fit`) et sept dérivés à régénérer.
- Option C : découper de vraies provinces (Champagne, Bretagne orientale) : une refonte de la carte composée, hors de portée d'un lot de données.

## Balayage de textes (≈ 45 chaînes)

Vérifiées par échantillon : événements (Sluys, Artevelde 1345, Gand 1340, Neville's Cross, Poitiers, Azincourt, Arras, Boccanegra 1339, Bardi, Philippe le Hardi, prince Noir…), codex (rançon, quarantaine de Raguse 1377, gabelle, Cerda, Froissart, Navarre, couleuvriniers…), factions du menu (Schleswig, Bourbon, Tyrone, Liège, Brandebourg, Pavie, Tripoli), didacticiel.
Corrigés :
- « le Pale » (mot du XVe siècle) remplacé par « les terres anglaises de Dublin » dans `front_end.json`, `fac_leinster`, `prov_dublin`, `prov_leinster`, `tit_ireland`.
- `cdx_chamois` : « antilope » devient « caprin » ; bundle du codex régénéré.
Points ouverts : `fac_hama` « Ta lignée a fondé le sultanat » (la branche de Hama descend du neveu de Saladin) ; la commande `cent-ans codex-bundle` cherche `tools/data` (chemin `parents[2]` à corriger, contourné en appelant `codex.write_bundle`).
