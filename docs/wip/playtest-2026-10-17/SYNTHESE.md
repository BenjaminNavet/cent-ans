# Campagne de test du 17/10 — synthèse

Rapports bruts des testeurs copiés depuis le dossier temporaire de la session ; les captures et dumps cités (`$P/...`) n'ont pas été conservés.

7 testeurs Sonnet + 1 diagnostic Opus. Versions : A = 7e0c4bb12 (jusqu'à 09:07), B = main 3dd37ab5c.
« B » = confirmé sur la version actuelle. Rapports détaillés dans ce dossier.
Aucun plantage, panique ni ERROR dans aucun journal.

## Bloquant (vu par le joueur, plein écran 2560×1440 Retina, ultra)
1. Terrain de campagne entièrement noir. Cause non trouvée (piste : MSAA caméra 3D vs caméra UI). → diag-rendu-ultra.md
2. Étiquettes monde en double et décalées — cause : `UiScale` (1,76 sur Retina) non appliqué ; corrigé dans main (a90580726), à confirmer par le joueur.
3. Étiquettes/plaques monde par-dessus le menu pause. Cause non trouvée.

## Majeur
| # | Domaine | Défaut | Ver. | Rapport |
|---|---|---|---|---|
| 4 | UI | Recrutement : cartes d'unités écrasées, « Sceller la levée » hors cadre (`panels/army/view.rs:842-900`) | B | ux, ui-campagne |
| 5 | UI | Dialogue pré-bataille trop haut, boutons hors écran | B | ui-bataille, ux |
| 6 | UI | Écran de résultat : défilement invisible, « Retour à la campagne » caché | B | ui-bataille, bataille |
| 7 | UI | Titre « Défaite » / vainqueur crème sur parchemin, illisible | B | ui-bataille |
| 8 | UI | Économie : tableau du budget déborde | B | ui-campagne |
| 9 | Perf | Campagne 18-30 i/s (pire à d=60 : 66 ms) ; régression probable A→B de 12-20 ms | B | performance |
| 10 | Perf | Temps d'image qui monte et ne redescend pas (38 → 95 ms après conseil + 18 tours) | A+B | performance |
| 11 | Bataille | `--state Battle` n'ouvre jamais le déploiement ; démos de déploiement/ordres ne testent rien (`orders/deploy.rs:131`) | B | bataille |
| 12 | Bataille | Siège de Paris pris mais écran « L'ost de France mis en déroute » | A | bataille |
| 13 | Campagne | Après défaite, les deux armées restent à Paris, ni retraite ni butin/prisonniers (design ?) | A | bataille |
| 14 | UX | Chargement : journal, chronique, `feudal.recent` vidés ; conseiller rejoue l'ouverture | A | ux |
| 15 | UX | J (Conseil) et C (Cour) sans effet ; conseiller annonce (T) au lieu de F4 | A | ux |
| 16 | Carte | Rochers noirs tramés géants sur toutes les montagnes (`countryside/rocks.rs`) | A | carte-campagne |
| 17 | Carte | Baie du Mont-Saint-Michel = dalle noire sans le Mont (mapgeo11 arrivé en B) | A | carte-campagne |
| 18 | Carte | Lac Léman : plan flottant + trou noir | A | carte-campagne |
| 19 | Carte | Villes 1:1 sur l'eau (Tamise grise sous le pont, Constantinople sur le Bosphore) | A | carte-campagne |
| 20 | Carte | Rivières en aval : ruban noir enterré par le relief | A | carte-campagne |
| 21 | Carte | Mât géant sur le clocher de Saint-Denis | A | carte-campagne |
| 22 | Anim | Charrette + bœuf près de Paris en bouillie noir/blanc trouée | A | animation |
| 23 | Perf | Crécy mêlée 41,5 i/s, à la limite ; pic isolé de 444 ms | B | performance |

## Mineur (sélection)
- Panneaux qui s'empilent : bulle du conseiller sur les modales (cache « Commencer la bataille », titres), fin de saison = guide + mission + armée + 4 toasts, F3/F4 ouvrent sous le menu pause, F1 + tutoriel, F1→Échap ouvre la pause (A).
- Traditions d'armée débordent (A) ; taille UI « Très grande » tronque la barre du haut (A) ; sélection de faction : boutons coupés, Échap sans effet.
- IA défenseuse passive (1002 s sans contact) ; IA de bataille personnalisée lente (~330 s).
- Résultat de bataille contradictoire (1098 vs 2093 survivants) ; « Défaite » en mode Regarder ; naval « chances 100 % » à 6 contre 12.
- Paris 9/6 emplacements ; entretien hommes d'armes 0 vs 735 ₶ ; « à paris » brut ; « Bataille de Île-de-France » ; « côte ouest » pour l'Île-de-France ; discours d'Édouard III au siège français.
- Alerte « points de recherche perdus » vs Technologies « s'accumulent ».
- Soldats figés après la fin de bataille (686 en déroute qui ne partent pas).
- Pavois sans texture laissés en place ; chevaux à franges noires ; morts trop clairs.
- Forêts/collines lointaines d'aspect low poly ; forêts d'hiver tramées ; oiseaux/chameaux trop grands ; Pétra informe ; étiquettes qui se chevauchent ; météo brouillard sans effet ; côtes en escalier.
- Mémoire 5-9 Go en campagne ; journal annonce 213 provinces (attendu ~440) ; eau construite deux fois au démarrage.

## Limites de la campagne
Pas de souris (pas d'infobulles, de survol, de clic sur la carte, de clic droit en bataille) ; rendu hors écran fixe 1280×800 ; pas de vraie fenêtre capturable ; muet.
Avant 09:00 les runs utilisaient les réglages et sauvegardes du joueur (autosauvegarde « Avant la bataille » de 08:29 écrasée) ; isolés ensuite (`sandbox/`).
