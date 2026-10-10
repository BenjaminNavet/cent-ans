# WH — lots de correction

Sources : `docs/wip/wh/<rôle>.md` (§3 top 10). Brief commun : `docs/wip/wh/brief-lot.md`.

| lot | source | ADR | état |
|---|---|---|---|
| idle | tour #1 #3 #4 #6 ; carte top1 + top4 (fuite brouillard alertes) ; ui top2 + top8 (raccourcis d'armée) | 0270 | FAIT (wh/idle, docs/wip/wh-idle.md) ; reste : option par type d'alerte |
| hover | carte top2 top3 top5 top6 top7 top10 (jetons parchemin, bulles survol armée/colonie, n° de tours, ZOC ennemies, dernières positions vues) | 0271 | FAIT (wh/hover, tests wh_hover_*; restes : disque de fond ZOC, fantôme jusqu'à expiration) |
| armya | armees top1 (entretien croissant + chef requis), top2 (rayon de renfort), top3 (vitesse par composition), top10 (lève le siège) | 0272-0273 | FAIT (branche wh/armya) : top1, top2 (sans arrivée tardive en 3D), top3, top10 ; UI : info-bulle surcoût + « à N km » |
| econ | economie top1 (ordre public décomposé), top2 (revenus par source), top3 (impôt par province), top4 (édits coûteux), top5 (plafond d'emplacements), top7 (bâtiments de cité) | 0274-0275 | FAIT (wh/econ, voir docs/wip/wh/econ.md) |
| chars | personnages top1 (actes royaux), top2 (blessures temporaires), top3 (recruter un capitaine), top6 (XP élargie + annonce de niveau), top9 (faits d'armes) | 0276-0277 | vague 1 | **FAIT** (branche wh/chars) ; reste : `cv3_ai_stances` (graine fragile, voir docs/wip/wh/chars.md)
| diploa | diplomatie top1 (aperçu des alliés avant guerre), top2 (étiquette + durée des modificateurs), top3 (alliés/ennemis sur la fiche), top4 (révoquer l'accès), top10 (historique des ruptures) | 0278 | vague 1 — FAIT (wh/diploa, à fusionner) |
| armyb | armees top4 (recruter dans l'armée), top5 (ordre Sortie), top6 (chevauchée), top9 (IA repos) ; ui top9 (sommation de siège) | 0279 | FAIT (branche wh/armyb, docs/wip/wh/armyb.md) : top4 `RecruitInto`, top5 `Sortie` (auto-résolue seulement), top6 vivres + rendement décroissant, top9 `seek_place` (le repos en place existait : NT6c), ui top9 `DemandSurrender` ; UI : destination des recrues, bouton Sortie, bouton Sommer |
| uicards | ui top1 (XP sur cartes), top3 (captifs), top4 (pop/mécontentement colonie), top5 (pertes estimées), top6 (file de recrutement), top7 (pastilles barre haute), top10 (fiche d'unité) | 0280 | PARTIEL (wh/uicards) : top1, top4, top5, top6, top7, top10 + po_ui_test C2/C3 faits ; reste top3 (captifs : effet de prestige/chevalerie à définir) |
| turn | tour top2 (sauvegarde auto), top5 (suivi de missions), top7 (dilemmes conditionnels), top8 (rapport de saison auto), top9 (missions de faction), top10 (bilan de fin) | 0281 | FAIT (wh/turn, tests wh_turn_* ; restes WR : offre de mission en choix et filtre de genre du journal FAITS (wr/turn, ADR 0304) ; 16 options annotées seulement) |
| diplob | diplomatie top5 (ligue anti-hégémon), top6 (JoinWar), top7 (appel d'un allié au joueur), top8 (non-agression/défensive), top9 (ultimatums IA) | 0282-0283 | vague 2 — FAIT (wh/diplob, à fusionner) ; restes : l'IA ne propose ni pacte ni défensive, pas de JoinWar IA→joueur |
| charsb | personnages top4 (loyauté), top5 (assassinat), top7 (déclencheurs de traits), top8 (compétences par rôle), top10 (aide d'agent) | 0284 | vague 2 | **FAIT** (branche wh/charsb, voir docs/wip/wh/charsb.md) |
| mapb2 | carte top8 (rotation souris + suivi), top9 (ping minicarte + clic droit) ; economie top9 (onglet commerce) | 0285 | vague 2 — FAIT (branche wh/mapb2) |
