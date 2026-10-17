# Rapport QA — bataille (gameplay et simulation)

Versions : runs avant 09:07 = binaire 7e0c4bb12 (b1, b2, crecy, siege, watch Poitiers, cb, cf, bdeploy/orders) ; dd (re-vérif) et nav = binaire neuf (3dd37ab5c) probable. Dumps et captures : `$P/bataille/`.
Limites : le script ne sait pas faire de clic droit / glisser souris ; les ordres n'ont été testés que via `CENT_ANS_ORDERS_DEMO`/`BDEPLOY_DEMO`, les cartes d'unité, `KeyP`, la barre de vitesse. Pas de plantage, ni `ERROR`/`panicked` dans les journaux (seulement 3 WARN bevy_light/prepass/winit inoffensifs).

## Défauts

### 1. [majeur, à confirmer en jeu réel] `--state Battle` n'ouvre jamais la phase de déploiement
- Repro : `$P/play.sh --state Battle --script dd.txt` (dump après 120 images). Vérifié sur 7e0c4bb12 (b1_*.json) ET sur le binaire neuf (dd_1.json).
- Observé : `battle_orders.deployment = {active:false, opened:false}`, `battle.tick = 33` : la sim a déjà avancé. Les démos `CENT_ANS_BDEPLOY_DEMO`/`ORDERS_DEMO` ne testent donc rien (`deploying` active:false, `after_place` = `before_place`, positions inchangées).
- Attendu : déploiement figé à tick 0 (comme via le menu Batailles historiques / personnalisée, où ça marche : tick 0, bandeau « Commencer la bataille », zone).
- Cause probable : `client/src/battle/orders/deploy.rs:131` exige `sim.ticks() == 0` ; avec l'entrée directe la sim tourne avant que la caméra existe (`deployment.tried` attend `camera_state`). Chemin menu non touché ; donc surtout dev/preuves, mais fait aussi croire que le déploiement fonctionne.

### 2. [majeur] Fin de bataille de siège : verdict et faits incohérents
- Repro : menu Batailles > Batailles de démonstration > `Demo_siege_paris`, laisser tourner ×4 (siege.txt) ; fin à 5 min 54 s.
- Observé (siege_4.json, siege_s3.png) : défaite de la France (défenseur), fin causée par la place prise (`capture_points square: taken`), mais le texte dit « L'ost de France a été mis en déroute » et le tableau des régiments indique « tient le champ » pour les 4 régiments français ; pertes France 173 (41 %), Angleterre 23. Le défenseur passif perd donc sans qu'il y ait déroute.
- Attendu : fait « la place a été prise / tenue » (BattleEnd::SquareHeld) cohérent avec les sorts des régiments.
- Cause probable : `core/crates/sim-battle/src/sim/morale.rs:153-195` (end SquareHeld) vs texte du HUD `client/src/battle/hud/result.rs` qui retombe sur la phrase de déroute.

### 3. [majeur, design à confirmer] Armée perdante qui reste sur place après la bataille de campagne
- Repro : `CENT_ANS_STAGE_BATTLE=1 CENT_ANS_NO_DEPLOY=1 $P/play.sh --state Campaign --script cf.txt`, bataille jouée passivement jusqu'au bout, puis `result:return`.
- Observé (cf_after.png) : « Défaite » mais les deux armées (567 et 540) restent côte à côte à Paris ; `pending_battles=[]`, le bilan ne cite ni retraite, ni butin, ni prisonniers (`captives:[]`, `lost:[]`, +10 XP du général seulement). Pertes cohérentes (620→567 identique à la barre de campagne : OK).
- Attendu : le perdant se replie ou le journal dit pourquoi les armées coexistent.

### 4. [mineur] Bataille de campagne : l'IA défenseuse ne contre-attaque jamais et la partie dure ~17 min pour rien
- Observé : cf_*.json, 620 vs 540, ×4 : aucun contact (France 620→567, Angleterre 540 constant) pendant 1002 s, fin par « Le combat retombe : France se retire » (BattleEnd lull, EP9). Défaite pour un joueur passif : cohérent, mais l'IA défenseuse reste totalement statique ; dans la bataille personnalisée l'IA ennemie met ~330 s (sim) pour entrer en contact (cb_6.json : premier choc à 5:28).

### 5. [mineur] Écran de résultat : totaux ≠ cartes affichées
- Observé (crecy_s7.png, crecy_8.json) : « Survivants 1 098 » (Angleterre) alors que `battle.defender.soldiers = 2093` et que les cartes montrent des régiments intacts (130/150/180) ; les pertes affichées (3 682, 77 %) incluent la poursuite après déroute (core `sim/mod.rs:767-771`, ADR 0321) mais les cartes montrent l'état avant poursuite. Même chose Poitiers (3993 vs 4585) : incohérence visible pour un joueur. Légende ou cartes à aligner.
- Autre : en mode « Regarder » (IA contre IA) l'écran affiche « Défaite » pour le spectateur (watch_14.json : `title: Défaite, won:false`), au lieu du vainqueur neutre.
- Le panneau de résultat (1280×800) est coupé en bas (« Butin » rogné) : défilement `ResultScroll` sans indicateur visible ; le bouton « Retour à la campagne » n'est atteignable qu'en défilant. Le sous-titre « durée · vainqueur » est quasi illisible (crème sur crème, siege_s3.png ; peut être une capture en plein fondu).

### 6. [mineur] Bataille navale : « chances 100 % » incohérent
- Repro : `CENT_ANS_STAGE_NAVAL=1 $P/play.sh --state Campaign --faction fac_france --script nav.txt` (nav_dialog.png/json).
- Observé : 6 nefs / 620 hommes contre 12 navires / 704 hommes : « Victoire presque certaine », « Chances de victoire estimées : 100 % » alors que la barre d'équilibre est à ~47 % (`share 0.468`) ; 100 % absolu douteux avec la moitié de navires. Résolution auto fonctionne (`naval_resolved: 1`), pas de 3D navale : conforme.

### 7. [cosmétique] Fenêtres de flux
- Dialogue « Bataille en vue » (cf_dialog.png, 1280×800) : trop haut, illustration rognée en haut, boutons de combat hors écran (cliquables seulement par nom ; à vérifier au défilement) ; le résumé « 2 × Chevaliers… » chevauche la carte « Archers montés » de droite ; la bulle du conseiller (« Jean le Bel ») recouvre l'en-tête des modales (aussi navale, bilan de bataille).
- Le discours avec le bandeau « Commencer la bataille (Entrée) » se superpose (dep_1.png). La sim reste en pause pendant tous les discours (≈ 600 images dans le b1 sans `NO_SPEECH`), à passer par Échap.
- Menu historique : une modale « Jouer le didacticiel de bataille ? » s'intercale après chaque choix (scripts : `click_if prologue_invite:no`).

## Ce qui marche bien
- Crécy (défenseur) : 13 180 soldats, 14 min 17 s ; Poitiers en spectateur (IA contre IA) : 15 min 45 s, vainqueur Angleterre (historique), massacre « pas de quartier » indiqué ; siège de Paris : trébuchets, tours (492 volées), brèche, porte brisée, maisons en feu ; toutes les batailles se terminent (`finished: true`) bien avant MAX_DURATION 3600 s.
- Pertes évoluent de façon monotone ; replay `REPLAY_DEMO` : empreintes identiques (matches:true, 5501 ticks, déterminisme OK).
- Ordres (`ORDERS_DEMO`) : sélection, boîte, move, glisser avec largeur, attaque : 3 acceptés / 0 refusés, coordonnées reprises par la sim ; signet F2 OK ; abilités (Serrer les rangs) et cartes d'unité fonctionnelles ; déploiement via menu OK (zone, formations de groupe, démarrage par Entrée).
- Vitesse ×4 fiable (≈ 1 s sim / 0,25 s réelle) ; résultats de campagne cohérents (567 h après bataille = dump). Aucun plantage.

## Non testé
Clic droit/souris réels, formations en cours de bataille, archers vs cavalerie ciblés, météo variée, Azincourt, bataille de campagne avec ordres du joueur (donc butin/prisonniers jamais vus : captifs [] sans combat), siège depuis la campagne, sauvegarde auto avant bataille.
