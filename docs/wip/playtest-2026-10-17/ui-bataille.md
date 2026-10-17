# Rapport UI bataille

Versions : A = binaire 7e0c4bb12 (avant 09:07), B = 3dd37ab5c (après). Captures dans `$P/ui-bataille/` (A : pre1, deploy1/2, sel1, r2, r3, s1, s2, m1-m3, c3 ; B : v_pre, v_deploy, v_siege, v_res, v_crecy).
Fenêtre de test 1280x800.

## Majeurs

1. **Dialogue pré-bataille : débordement vertical, boutons hors écran** (A et B, pire en B). Repro : `CENT_ANS_STAGE_BATTLE=1 play.sh --state Campaign --script` (`wait 200`, `shot`). Observé : bannière rognée en haut ; en B les trois boutons Retraite / Résolution automatique / Combattre sont hors cadre (invisibles) ; en A ils sont rognés au bord bas. En B les cartes de régiments anglais passent à la ligne et la ligne « 2 × Chevaliers… » les chevauche (v_pre.png). Le dialogue n'a pas de défilement (`flow/dialog.rs`, seuls `Overflow::clip` aux l.230 et 313). Une bataille ne peut pas être lancée à la souris sur 800 px de haut. Cause probable : hauteur du corps non bornée (bannière 132 px, carte de site, texte).
2. **Écran de résultat : bouton « Retour à la campagne » invisible et sans indice de défilement** (A et B : r3/r4, e1, v_res). Le panneau `ResultScroll` (`battle/hud/result.rs:562-566`, `scroll_y`) dépasse 800 px ; le tableau France est coupé sur « Sergents montés » et le bouton `result:return` (l.880) est tout en bas sans barre de défilement. Le script `click result:return` fonctionne, mais un joueur voit un écran sans issue. Colonne « Sort » du tableau anglais collée au bord droit du cadre.
3. **Sous-titre et titre de résultat quasi illisibles** (A et B) : « éfaite » est crème sur parchemin, et le sous-titre « 6 min 40 s · vainqueur : Angleterre » est presque invisible (r3.png, v_res.png). Contraste d'environ 1:1.

## Mineurs

4. **Bulle du conseiller (Jean le Bel) par-dessus les modales et le HUD de déploiement** (A et B). Sur le dialogue pré-bataille, elle recouvre la bannière ; au déploiement, elle masque la barre « Commencer la bataille » et le discours du général (deploy1.png ; v_deploy.png : le bouton « Commencer… (Entrée) » est à moitié caché). Elle porte aussi un texte de début de partie (« Voici votre royaume… ») qui revient à chaque bataille et au retour en campagne (e2.png, par-dessus la modale « Défaite »). Étage de z : infobulle/avis au-dessus des modales.
5. **Titres sans élision** : « Bataille de Île-de-France », « Siège de Île-de-France » (barre du haut, modale de bilan). Source : `battle/scene/mod.rs:236,237,295` (`format!("… de {}")`) ; attendu « d'Île-de-France ».
6. **Bataille historique remplacée par le didacticiel sans indication** (A et B) : `click Battle_crecy_attacker` ouvre d'abord « Jouer le didacticiel de bataille ? » ; Entrée = Oui lance une escarmouche de 1337, titrée « Bataille personnalisée : Angleterre contre France ». C'est conforme à la conception (menu3), mais la modale est posée sur la liste sans nom de la bataille choisie, et `click Non` échoue si on le lance moins de 20 images après (le nœud apparaît tard).
7. **Aide F1 (J) mal empilée** (c3/c4) : le texte d'aide passe sous le cadre du didacticiel, la barre de déploiement et le discours, et sa fin est rognée par le panneau de formations (« boutons sous l… »).
8. **Discours de déploiement incohérent** (B, `v_siege.png`) : siège de Paris, le sceau du chef est à l'hermine/France, les cartes sont françaises, mais le discours est signé « ÉDOUARD III ». En A il disait « LE CAPITAINE : voici l'Anglais… ». À vérifier : orateur = adversaire ?
9. **Cartes du résultat sans illustration** : bandes noir/rouge génériques (r3.png), alors que les cartes du HUD ont leurs illustrations.
10. **Ligne « Unité ralliée » de la colonne d'alertes sans icône** (e1/r4, A) ; alertes sinon lisibles.
11. **Barre du haut** : « Nuit dans 51:35 » passe à la ligne et la barre se décale à gauche en cours de bataille (e1.png) ; repères d'unités les plus lointains rognés/derrière la barre du haut (play2.png).
12. **Infos de site absurdes** : « côte ouest » pour Île-de-France (intérieur des terres) dans la barre et le dialogue.
13. **Menu Batailles** : « Rejeux » est actif (le README le dit grisé), non testé au-delà. Bataille personnalisée : « Fortification 2 » et ± affichés alors que le siège est désactivé ; budget camp 2 « 11800/12000 » avec armée par défaut ; champ « Nom de la composition » flou.

## Cosmétiques
14. Titre « Ordres du chef » chevauche le bord supérieur du cadre.
15. Au loin (r2/e1/play2), certaines unités sont rendues en tirets blancs/gris uniformes (imposteurs) bien visibles, contraste fort avec les autres.
16. Siège de Paris en A (s1.png) : la caméra de déploiement est collée dans un mur de tour (vue entièrement bloquée) ; corrigé en B (v_siege.png, mais les remparts n'y sont plus visibles).

## Ce qui marche bien
Cartes d'unités (effectifs, barres, pastilles, infobulles), minicarte et vitesses (`speed:4` actif), menu des formations (8 entrées), repères d'unités (couleurs de camp, barres, sélection dorée), alertes (regroupées « ×n »), journal, rapport de forces, bilan par régiment, retour en campagne et modale de bilan sans élément résiduel, aucun ERROR/panique dans les journaux, listes historiques et démos claires, écran de bataille personnalisée complet.

## Non testé / limites
Infobulles au survol et curseurs (pas de commande de survol), clic droit d'ordres, bataille du côté défenseur, « Regarder » (poitiers_watch : run non terminé faute de slot), Crécy joué jusqu'au bout, batailles navales, résolutions automatiques, 1280x800 uniquement (autres tailles non testées). Défauts 1 à 3 revérifiés sur B ; 7, 9-15 observés en A seulement.
