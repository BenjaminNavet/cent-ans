# RX — rapport `bataille` (10/10/2026)

Périmètre : déroulé d'une bataille, sièges, IA, fin de bataille. Méthode : `cargo test -p sim-battle` (tous verts), 5 captures
(`/private/tmp/claude-501/rx-shots/bataille/` : deploy, contact, melee, siege, result), lecture des ADR 0095/0108 et des règles `data/rules/battle_*.json`.
Limites : machine très chargée (load 20-30, 18 processus Godot d'autres sessions) ; `smoke_battle.gd` et `--autoplay` en lot n'ont pas pu être menés à terme
(voir constat 1) ; pas de sondage statistique d'issues sur plusieurs graines (il n'existe pas de commande `survey` : le test cité en tête de `ep9_decisive.rs` n'existe plus).

## 1. Verdict
- Forces : cœur de bataille très couvert (67+31+81+137+13+58+78 tests verts, déterminisme/rejeu inclus), contrôles TW-style complets et lisibles (barre d'ordres, formations de groupe, capacités avec raccourcis), écran de fin riche (faits notables, sort par régiment, étendards pris), siège spectaculaire et lisible (échelles, bannières, barre d'état des murailles).
- Faiblesses : la vérification automatique de bataille côté Godot est fragile (smoke bloqué), la capture de mêlée (`--closeup --shot-at`) ne cadre pas la mêlée, l'IA attaquante s'engouffre dans un pont, et quelques défauts de finition d'interface (tuiles vides sous le tableau de fin, bandeau de déploiement au centre).
- Pas d'erreur shader `soft_distance` dans les 5 journaux de capture (grep négatif) : la variable ne apparaît que comme uniforme valide de `fire_smoke.gdshader`/`fire_flame.gdshader`. À considérer résolue sauf réapparition en bataille avec feu actif (non reproduit, aucun feu en siège de 2 min).

## 2. Constats

### [majeur] [bug/outillage] `smoke_battle.gd` ne se termine pas en headless sur machine chargée
**Constat** : le test de référence de la bataille (CLAUDE.md) reste figé après `SimFacade: CampaignSim ready`, sans aucune ligne ensuite, pendant plus de 10 min (deux essais, le premier tué par le délai à 600 s, code 144). Impossible de savoir s'il s'agit d'une lenteur (dylib debug, charge) ou d'un blocage ; aucune progression n'est écrite.
**Preuve** : `/private/tmp/claude-501/rx-shots/bataille/smoke.log` et `smoke2.log` (4 lignes seulement) ; la tâche de fond a fini en code 144.
**Correction proposée** : faire écrire à `game/tests/smoke_base.gd` une ligne par étape (`_run_battle`, `_run_siege_battle`, `_check_*`) avec l'heure ; vérifier si la dylib de `game/bin/` est bien celle compilée en release ; le garde-fou à 45 min ne protège pas un agent pressé.
**Coût** : S

### [majeur] [conception/IA] L'attaquant IA/joueur entasse toute l'armée sur le pont unique
**Constat** : à 3:37 (capture contact), l'avant-garde française est au contact du hameau, tandis que le reste de l'armée est encore en colonne serrée sur le pont, les cartes d'unités à 6/80 hommes et 47/60 montrant déjà de lourdes pertes sur l'avant-garde seule. Les formations arrivent par paquets (engagement séquentiel) : la charge « se brise dans le hameau » (journal 03:35) et la bataille est perdue en 5 min 27 s avec 32 % de pertes contre 14 %.
**Preuve** : `contact.png`, `result.png` (France 198 pertes / Angleterre 74, déroute générale à 5:27 ; Archers à l'arc long II : 0 perte, 58 tués).
**Correction proposée** : dans `core/crates/sim-battle/src/ai/` (rôles/horse), temporiser la charge de cavalerie jusqu'à l'arrivée de l'infanterie d'appui (ou lui faire contourner le bourg) ; vérifier avec le test `eq7_cavalry.rs` et un banc sur 6 graines Île-de-France.
**Coût** : M

### [majeur] [équilibrage] Seuil de déroute 40 % d'effectif apte, effet boule de neige très rapide
**Constat** : l'armée est brisée à 32 % de pertes ; trois régiments d'arbalétriers/sergents « en déroute » ont 0 perte propre (contagion). Pour le joueur, une défaite arrive sans qu'il ait pu réagir (5 min 27 de bataille, déploiement passé).
**Preuve** : `result.png` (Arbalétriers I : 0 perte, en déroute) ; `data/rules/battle_outcome.json` (`break_share 0.4`, `break_hold_seconds 30`) ; `battle_rout.json` (contagion).
**Correction proposée** : vérifier sur plusieurs graines la durée médiane (cible ep9 : 5-12 min) ; si la médiane est < 6 min, relever `break_hold_seconds` ou atténuer la contagion pour les unités non engagées. Mesure à faire avant de toucher.
**Coût** : S

### [majeur] [bug/outillage] Le cadrage `--closeup --shot-at=<s>` ne trouve pas la mêlée
**Constat** : avec `--closeup --shot-at=90`, la caméra regarde un champ vide (horloge à 01:30, aucun soldat). Les captures de preuve de mêlée exigées par la revue ne sont donc pas obtenables par cette combinaison ; il faut `--camera=x,z,d,lacet` avec des coordonnées devinées. Le journal indique aussi « 00:00 Les Chevaliers d'Angleterre mettent pied à terre » sur une image à 01:30 (journal non rafraîchi ou anecdote).
**Preuve** : `melee.png` ; `game/scripts/battle/battle_capture_stage.gd` (cadrage closeup).
**Correction proposée** : en `--closeup`, cadrer le barycentre des unités au contact (ou, à défaut, des deux armées), pas la position du premier contact.
**Coût** : S

### [mineur] [finition] Tuiles blanches vides sous le tableau des régiments à l'écran de fin
**Constat** : sous les listes de régiments, quatre cartouches blanches vides (vers y 930) apparaissent, coupées par le bandeau de boutons ; le panneau défile (barre à droite) mais le premier regard montre une zone vide.
**Preuve** : `result.png`.
**Correction proposée** : masquer les cartouches tant qu'elles n'ont pas de contenu ou dimensionner la zone défilante (`battle_aftermath.gd`).
**Coût** : S

### [mineur] [finition] Bandeau de déploiement posé au centre de l'écran sur le terrain
**Constat** : « Les Chevaliers doivent être placés dans votre zone de déploiement » s'affiche au milieu de la carte, en recouvrant les unités à placer ; l'invite « Commencer la bataille » est en haut, loin. À cette altitude les régiments sont de fines lignes, peu cliquables.
**Preuve** : `deploy.png`.
**Correction proposée** : message d'erreur en bandeau bas (au-dessus des cartes d'unités) ou en infobulle au curseur ; caméra de déploiement plus basse par défaut.
**Coût** : S

### [mineur] [finition] Avertissement bruyant hors campagne
**Constat** : lancer la bataille seule déclenche `CampaignSim: method called before new_campaign` à chaque ouverture (portrait du chef), rempli dans 5 journaux.
**Preuve** : `living_portrait.gd:306` (`character_for`) via `battle_hud.gd:468`.
**Correction proposée** : ne pas appeler `get_character` si la sim n'a pas de campagne (garde `sim.has_campaign()` ou équivalent).
**Coût** : S

### [mineur] [conception] Aucune alerte « armée unique sur pont / goulot » ni prévisualisation du pont
**Constat** : le joueur n'est pas prévenu d'un goulet ; le cheminement par un pont est invisible sur le plan de déploiement.
**Preuve** : `contact.png` (colonne sur le pont, rectangles de formation au milieu de l'eau).
**Correction proposée** : l'aperçu de chemin (ADR 0095, `preview_path`) existe : l'afficher aussi en déploiement pour les unités placées de l'autre côté d'un cours d'eau.
**Coût** : M

## 3. À ne surtout pas changer
- L'invariant aperçu = ordre et la table unique de touches `battle_hotkeys.gd` (ADR 0095).
- Le déterminisme/rejeu (`ep13_replay`, `state_digest`) et l'écran de fin (faits notables, étendards pris).
- Le rendu du siège (échelles, murailles, barre d'état, bannières) : lisible dès la première capture.
- Le garde-fou headless (aucune dérive de journaux observée).
