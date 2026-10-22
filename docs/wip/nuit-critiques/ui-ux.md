# Critique UI/UX (rôle designer) — 2026-10-22

Sources : `docs/manuel.md`, `client/src/campaign/hud/{bar,buttons}.rs`, `client/src/settings/{keymap,rows,model}.rs`,
`client/src/ui/kit/style.rs`, `data/tutorial/steps.json`, un dump + une capture campagne 1280×800
(`out/ux/camp.json`, `out/ux/camp.png`, via `run_bg.sh`, muet, hors écran). Aucun code modifié.
Limite : tutoriel affiché à l'étape 14/14 dès le lancement par script (état persisté), donc le début du parcours est jugé sur les textes, pas sur le rendu.

## 1. Manuel et raccourcis réels divergent (majeur)
- **Problème** : un joueur qui suit le manuel appuie sur des touches qui font autre chose.
- **Preuve** : manuel § 15 : T technologies, P diplomatie, N chronique, M mode mécontentement. Code `keymap.rs` : Technologies = F4, Diplomatie = F3, Chronique = F5, **M = Économie**, mécontentement = Maj+M, Conseil = J, Cour = C. La capture montre d'ailleurs F3/F4/F5 sous les icônes. Le manuel nomme aussi « Cour (C) » alors que le bouton s'intitule « Conseil » (J).
- **Gravité** : majeur (le tutoriel et l'aide F1 renvoient à O/L/F1, mais le manuel est la référence).
- **Correctif** : générer le tableau § 15 depuis `keymap.rs` (test qui échoue si le manuel diverge) ; harmoniser « Cour » / « Conseil ».
- **Effort** : S

## 2. Barre du haut : 13 médaillons ronds sans légende (majeur)
- **Problème** : 9 lanceurs fixes + objectifs + traditions + rançons + menu, tous des icônes de 32 px sans texte ; la lettre-raccourci est un petit keycap illisible à 1280 px. Le sens n'apparaît qu'au survol.
- **Preuve** : `bar.rs` (`LAUNCHER_SIZE = 32`, doc : « every launcher is an icon with its shortcut letter ») ; capture : icônes dorées quasi indiscernables (Recrutement, Économie, Ma faction, Féodalité).
- **Gravité** : majeur (onboarding et découvrabilité ; 4 fenêtres sur 9 n'ont aucun raccourci : `action: ""`).
- **Correctif** : légende courte sous l'icône au-delà de 1600 px de large ; regrouper en 3 familles (Royaume / Savoir / Monde) avec séparateurs ; donner un raccourci aux 4 fenêtres sans touche.
- **Effort** : M

## 3. Chiffres de la barre sans libellé (majeur)
- **Problème** : « 28 000 ₶ », « +20 046 », « 620 = », « 0 = », « 11 % ▲+3 » : le joueur débutant ne sait pas ce que sont 620 (hommes ? bannières ?) ni 0 (prestige).
- **Preuve** : capture ; `bar.rs` assume « icon + number, no labels » ; `hud.top` expose manpower 620, prestige 0, unrest 11. Les icônes « 620 = » et « 0 = » sont difficiles à distinguer.
- **Gravité** : majeur
- **Correctif** : infobulle riche immédiate (délai court) avec décomposition ; option Réglages « libellés de la barre » ; afficher la tendance en couleur ET en flèche (cf. 9).
- **Effort** : S

## 4. Le guide du tutoriel masque la carte et se superpose aux avis (majeur)
- **Problème** : la carte de guide fait ~350×350 px collée en haut à gauche, sur la zone « avis » ; c'est là aussi que se montrent le conseiller et le journal. Sur 1280×800 elle couvre le quart gauche, et la cible à désigner peut se trouver dessous.
- **Preuve** : capture (carte « étape 14/14 » + avis du conseiller côte à côte) ; manuel § 4 « Disposition de l'écran » promet « trois avis au plus » à cet endroit.
- **Gravité** : majeur
- **Correctif** : version compacte (titre + objectif en 2 lignes) repliable en un bandeau ; texte long et « conseil historique » derrière « Plus de détails » ; repousser la carte du côté opposé à la cible (flèche).
- **Effort** : M

## 5. Tutoriel : 14 étapes, trop de lecture, aucune pratique de la bataille en campagne (majeur)
- **Problème** : chaque étape est un paragraphe de 40-60 mots ; l'étape « Finir le tour » empile 5 notions. Aucune étape ne couvre les alertes, la minicarte, ni l'ordre « siège/attaque » (l'action centrale du jeu), le tutoriel de bataille étant séparé.
- **Preuve** : `data/tutorial/steps.json` (`step_ids` : intro … outro ; `end_turn.text`).
- **Gravité** : majeur
- **Correctif** : scinder en « 4 gestes » (sélectionner, marcher, bâtir, finir le tour) puis missions optionnelles proposées par le conseiller ; texte ≤ 25 mots + 1 illustration ; ajouter une étape « clic droit sur une place ennemie » avec résolution auto sans risque.
- **Effort** : M

## 6. Alertes : bruit et mauvaise priorité (mineur à majeur)
- **Problème** : trois alertes d'égale importance dès le tour 1, dont « 32 colonie(s) sans chantier » (chiffre énorme, pas actionnable d'un clic) ; badge « 1 » redondant ; aucune hiérarchie entre urgent (dette, siège) et conseil (recherche inactive).
- **Preuve** : `hud.alerts` : `research_idle`, `idle_army`, `free_slot` avec `blocking: false` ; capture : trois cartes identiques à droite de la cloche.
- **Gravité** : majeur (l'alerte qui compte se noie au 20ᵉ tour).
- **Correctif** : 3 niveaux visuels (danger rouge fixe, action requise, suggestion discrète repliée) ; « 32 colonies » devient « Chantiers : 32 libres » ouvrant la liste B triée par rendement ; un clic = action directe.
- **Effort** : M

## 7. Fin de tour : bouton lointain et retour incomplet (mineur)
- **Problème** : la cloche est au bas droite, sur le trajet des alertes et de la minicarte ; le joueur ne voit pas ce qui reste à faire avant de la sonner (la confirmation ne s'ouvre que « si oubli », par défaut, sans bilan avant/après).
- **Preuve** : `flow.rs` (`confirm_end_turn`: jamais / si oubli / toujours) ; capture : cloche entourée de trois alertes.
- **Gravité** : mineur
- **Correctif** : pastille sur la cloche = nombre d'actions oubliées ; après tour, bandeau « Hiver 1337 : +3 prises, −1 chantier fini » avant le rapport complet.
- **Effort** : S

## 8. Taille de texte : plancher bas et repli en 12-13 px (majeur accessibilité)
- **Problème** : le corps fait 17 px et les légendes 15 px à 900 px de haut (plancher 720p), mais des étiquettes de carte, mémoire des armées, calques parchemin tombent à 12-13 px ; l'échelle d'interface (0,7 à 2,0) n'a pas de réglage séparé du texte.
- **Preuve** : `style.rs` (`UiType`), `grep` : `Px(13.0)` dans `armies/markers.rs`, `armies/memory.rs`, `council/stage.rs`, `font_size: 12.0` dans `parchment/overlay.rs` et `extras.rs`. Capture : étiquettes « Arras 340 7 » petites, contrastées sur fond sombre mais serrées.
- **Gravité** : majeur
- **Correctif** : passer ces cas par `UiType::Caption` (donc l'échelle) ; ajouter un curseur « taille du texte » indépendant (100-150 %) ; plancher 14 px.
- **Effort** : S

## 9. Accessibilité : daltonisme limité au mode, couleur seule pour l'état (mineur)
- **Problème** : « Mode daltonien » et « Contraste renforcé » existent (`rows.rs`), mais le mode n'est documenté que pour la bataille (minicarte) ; la campagne code l'état (revenu positif/négatif, mécontentement, factions sur la minicarte) par des teintes. Les alertes et la barre n'ont pas de variante symbole.
- **Preuve** : grep `colorblind` : usages côté bataille ; capture : minicarte rouge/bleu/jaune sans motifs.
- **Gravité** : mineur
- **Correctif** : motifs ou hachures sur les factions de la minicarte, signes « + / − » et flèches dans la barre ; test de contraste dans `contrast.rs` étendu aux alertes.
- **Effort** : M

## 10. Minicarte et filtres illisibles à 1280×800 (mineur)
- **Problème** : la minicarte occupe ~230×180 px, avec 4 boutons carrés sans libellé dessous (« F », boussole…) ; le cadre de vue est un triangle peu parlant ; les noms de province n'y figurent pas.
- **Preuve** : capture, coin bas droit.
- **Gravité** : mineur
- **Correctif** : légende au survol dès la 1ʳᵉ seconde, redimensionnement (la poignée existe déjà dans `resize_grip.rs`, la mettre en évidence), clic droit sur la minicarte = ordre de marche.
- **Effort** : S

## 11. Aucun « annuler » visible pour un ordre ou un choix (mineur)
- **Problème** : clic droit lance aussitôt la marche (« l'armée part aussitôt », manuel § 4) ; aucun retour en arrière dans la saison, ni confirmation pour un ordre de plusieurs tours.
- **Preuve** : manuel § 4 et § 5 ; `grep` de « annuler » dans `campaign/armies` et `hud` : rien.
- **Gravité** : mineur à majeur pour un débutant (un mauvais clic coûte une saison).
- **Correctif** : conserver la position de départ tant que les points de mouvement ne sont pas dépensés (Ctrl+Z / bouton « Rappeler ») ; aperçu d'abord, validation au deuxième clic pour les trajets de plus de deux tours.
- **Effort** : M

## 12. Panneaux et fenêtres : cohérence « un seul panneau à droite » vs 9 fenêtres (mineur)
- **Problème** : la règle « un seul panneau à droite » est claire, mais 13 lanceurs et ~15 raccourcis sans guide de hiérarchie : où voir les revenus (Économie M, Ma faction, Colonies B) ? Trois endroits se recoupent.
- **Preuve** : `keymap.rs` (« Colonies (revenus, chantiers, menaces) » B ; « Économie (recettes, commerce, monnaie) » M ; panneau de faction sans raccourci) ; manuel § 5 décrit les revenus dans le panneau de faction.
- **Gravité** : mineur
- **Correctif** : une page « Où trouver quoi ? » dans l'aide F1 et dans le tutoriel ; renvois croisés en bas de chaque panneau (« Détail dans Économie »).
- **Effort** : S

## 13. Fin de partie et objectifs peu mis en scène (mineur)
- **Problème** : le manuel dit que le panneau Objectifs suit les buts, mais le rappel est passif (touche O) ; aucun indicateur « 2/4 » permanent près de la date.
- **Preuve** : manuel § 13 ; `buttons.rs` (`hud:open_objectives`) ; capture : aucun compteur d'objectifs.
- **Gravité** : mineur
- **Correctif** : pastille « 2/4 » sur le bouton Objectifs, mise en évidence quand un objectif devient proche (échéance < 10 ans).
- **Effort** : S

## Top 3
1. **Barre du haut et chiffres sans libellé** (constats 2 + 3) : légendes, regroupement, infobulles immédiates ; S à M, plus gros gain d'onboarding.
2. **Manuel ↔ raccourcis réels** (constat 1) : tableau généré depuis `keymap.rs` + test ; S, élimine une cause de frustration dès la première minute.
3. **Tutoriel allégé et non intrusif + alertes hiérarchisées** (constats 4, 5, 6) : cartes compactes, 4 gestes d'abord, trois niveaux d'alerte ; M.
