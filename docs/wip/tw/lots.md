# TW — lots de correction

Sources : `docs/wip/tw/<rôle>.md` (§3 top 10). Brief commun : `docs/wip/tw/brief-lot.md`.
Écartés (déjà ailleurs) : captifs tranchés en fin de bataille (WR captives), renforts lointains affaiblis (WR armies), sortie jouée (WR sortie), 5 écrans UX5.

## Vague 1
| lot | source | ADR | agent | état |
|---|---|---|---|---|
| bsim | simulation top1 (spear_wall), top3 (moral par type), top4 (discours à effet réel), top5 (cadence `reload_s` par type), top7 (état « ébranlé » + icône) | 0320 | dev | FUSIONNÉ (main) — reste : valeurs `reload_s` (lot balance) |
| pursuit | ia-sieges top1 (poursuite + prisonniers de troupe après bataille 3D, vers rançons campagne), simulation top9 (XP gagnée selon kills), ia-sieges top10 (butin étendards/bagages → or/prestige, vérifier) | 0321 | dev | FUSIONNÉ (main) — taux à équilibrer (`battle_outcome.json`) |
| retreat | ia-sieges top2 (retraite IA en rase campagne), transitions top6 (`BattleEnd::Withdrawal` pour la retraite générale), transitions top8 (le défenseur peut se replier hors siège/embuscade) | 0322 | dev | FUSIONNÉ (main) |
| bctrl | contrôles top1-top7 (ordres minicarte, pivot sur place, signets caméra, pause auto sur alerte, unité suivante au repos, attaque au pas Alt, sélection par classe) | — | mech | FUSIONNÉ (main) — reste : infobulle « attaque au pas » (lot polish) |
| bfeel | ressenti top1 (anneau d'ordre), top2 (barks halt/formation/retraite : textes + repli si clip absent), top3 (pastille munitions), top4 (ralliement : cœur expose `rallied`), top5 (ralenti chute du général), top6 (plan de victoire), top9 (infobulle du repère) ; ia-sieges top3 (temps restant / nuit, HUD) | 0323 | dev | FUSIONNÉ (main) — reste : recentrage général, bark flanked (lot polish), clips voix |
| trans | transitions top1 (écran de résultat de l'auto-résolution), top3 (risque sur le bouton Auto), top4 (chargement passable), top5 (carte « Dans l'Histoire »), top9 (briefing de début de campagne) | 0324 | mech | FUSIONNÉ (main) — reste : touche pour passer le chargement, briefing à la reprise (lot polish) |
| m2a | campagne top1 (désigner l'héritier), top2 (retinue 40), top3 (missions : compteurs agents/mariages/rançons), top9 (désertion sur solde impayée) | 0325 | mech | FUSIONNÉ (main) |
| m2b | campagne top4 (excommunication élargie + interdit), top5 (prétention dynastique par mariage), top6 (conversion de province), top7 (croisade papale) | 0326-0327 | dev | FUSIONNÉ (main) — dérive guerres g2 307/250 (lot wardiag) |

## Vague 2 (après fusion de la vague 1)
| lot | source | ADR | agent | état |
|---|---|---|---|---|
| balance | simulation top2 (duels A/B pierre-feuille-ciseaux, recalibrage `data/unit_types`), après bsim | 0328 | dev | en cours |
| siege | ia-sieges top4 (tours en données), top6 (contre-batterie), top7 (second point d'assaut) ; après WR sortie | 0329 | dev | FUSIONNÉ (main) — `tower_engine_factor` 0,3 (garde sg3 Avignon) |
| reinf | ia-sieges top5 (renforts à arrivée différée) + top9 (armée de secours en siège) ; après WR armies | 0330 | dev | FUSIONNÉ (main) — secours absent de l'auto-résolution et de la prévision |
| ai-deploy | ia-sieges top8 (l'IA réagit au déploiement du joueur), simulation top8 (ordre « poursuivre » joueur) | 0331 | dev | FUSIONNÉ (main ; pas de bouton HUD → lot polish) |
| misc | transitions top7 (carte du site en avant-bataille), top10 (compositions nommées), contrôles top8 (raccourcis reconfigurables), campagne top8 (marchand), top10 (objectifs de victoire) | 0332 | mech/dev | FUSIONNÉ (main ; misc-ui + misc-camp) — reste : bouton court/long à la nouvelle partie, seuil de prestige |
| polish | restes UI : bouton Poursuivre, infobulle attaque au pas, bark flanked, recentrage général, chargement au clavier, briefing à la reprise | — | mech | en cours |
| ai-camp | IA de campagne : héritier, conversion, croisade, réconciliation papale | 0334 | dev | en cours |
| wardiag | dérive des guerres déclarées après m2b | — | dev | en cours |
| balance | simulation top2 (duels A/B pierre-feuille-ciseaux, recalibrage `data/unit_types`), après bsim | 0328 | dev | FAIT (branche tw/balance) — matrice dans docs/wip/tw/balance.md |
| siege | ia-sieges top4 (tours en données), top6 (contre-batterie), top7 (second point d'assaut) ; après WR sortie | 0329 | dev | à faire |
| reinf | ia-sieges top5 (renforts à arrivée différée) + top9 (armée de secours en siège) ; après WR armies | 0330 | dev | à faire |
| ai-deploy | ia-sieges top8 (l'IA réagit au déploiement du joueur), simulation top8 (ordre « poursuivre » joueur) | 0331 | dev | à faire |
| misc | transitions top7 (carte du site en avant-bataille), top10 (compositions nommées), contrôles top8 (raccourcis reconfigurables), campagne top8 (marchand), top10 (objectifs de victoire) | 0332 | mech/dev | à faire |
| ai-deploy | ia-sieges top8 (l'IA réagit au déploiement du joueur), simulation top8 (ordre « poursuivre » joueur) | 0331 | dev | FUSIONNÉ (main ; pas de bouton HUD, raccourci P) |
| misc | transitions top7 (carte du site en avant-bataille), top10 (compositions nommées), contrôles top8 (raccourcis reconfigurables), campagne top8 (marchand), top10 (objectifs de victoire) | 0332 | mech/dev | misc-ui FAIT (branche tw/misc-ui : transitions top7+top10, contrôles top8) ; reste campagne top8 (marchand) et top10 (objectifs) |
| polish | restes GDScript/UI : bouton HUD « Poursuivre », infobulle « Attaque au pas » (Alt), bark `flanked` (texte + repli, clips à générer : fr_flanked_01-02, en_flanked_01-02), recentrage doux sur le général qui tombe (`general_fall_slowmo.recenter`), chargement passable à Espace/Échap/Entrée, briefing absent d'une sauvegarde du tour 0 | — | dev | FAIT (branche tw/polish ; test `tw_polish_test.gd`) |
| oeil | contrôle visuel final (≤ 10 captures) | — | session | fin |
