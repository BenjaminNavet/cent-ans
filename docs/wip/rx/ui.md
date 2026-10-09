# RX — critique UI/UX

Captures : `/private/tmp/claude-501/rx-shots/ui/` (menu, faction, map, deploy, result). Cadrage 2624x1644.

## 1. Verdict
- Forces : thème parchemin cohérent du menu à la bataille, français propre, aucune clé brute ni chaîne anglaise visible dans `game/scripts/ui`, infobulles complètes.
- Les 137 clés `attach_plain/rich` du code existent toutes dans `data/ui/tooltips.json`. Tous les noms d'unités, bâtiments, techs et traits ont un `name.display`. Seuls 3 effets d'`agents.json` (`opinion`, `heresy`, `favor`) n'ont pas de libellé (sans doute pas de vrais effets).
- Faiblesses : espaces vides (écran de faction, panneau de colonie), texte tabulaire petit à l'écran de résultat, étiquettes tronquées dans la barre de bâtiments, typographie hétérogène (apostrophes, chiffres elzéviriens).
- Non vérifié : `smoke_ui.gd` lancé en headless, aucune sortie après 4 min, tué. Les constats viennent de lecture et de captures.

## 2. Constats

### [majeur] finition — Étiquettes tronquées dans la barre d'emplacements de bâtiments
**Constat** : cellules de 46 px, mais le texte « verrou » s'affiche « verro », « Niv. 3 » et « Bâti » sont serrés. Sur la carte, la barre montre une dizaine de cellules illisibles d'un coup d'œil.
**Preuve** : `game/scripts/map/settlement_slot_bar.gd:12` (`CELL_SIZE 46x56`), `:99-103` ; capture `map.png` (bas de l'écran).
**Correction proposée** : remplacer « verrou » par une icône de cadenas ou un tiret ; raccourcir ; ou `text_overrun_behavior` ellipse avec infobulle ; élargir à 54 px.
**Coût** : S

### [majeur] finition — Écran de résultat : tableaux en 13 px, ligne de cartes vides clippées sous la liste
**Constat** : les tableaux de régiments sont en `ROW_FONT 13`, soit petit en 720p. Sous la liste, des rectangles arrondis vides sont coupés par la zone de défilement, juste au-dessus des boutons. Les mentions « Faits notables » sont encore plus petites.
**Preuve** : `game/scripts/battle/battle_result_screen.gd:23` ; capture `result.png` (y≈935).
**Correction proposée** : `ROW_FONT` 14-15 via `UiType`, vérifier ce qu'est la bande vide (probablement la rangée suivante clippée, à masquer ou à marger) ; ajouter une marge basse au défilement.
**Coût** : S

### [majeur] conception — Écran de choix de faction : trois cartes à 60 % vides
**Constat** : chaque carte ne contient qu'en-tête, souverain et défi, puis plus de la moitié de la hauteur est vide (fond parchemin uni). Seule la carte sélectionnée est éclairée. Les forces et faiblesses n'existent que dans le panneau de droite. Peu de choses pour comparer.
**Preuve** : `faction.png` ; `game/scripts/ui/faction_select.gd`.
**Correction proposée** : y mettre les chiffres clés (trésor, provinces, armée, voisins) ou une carte miniature de la zone de départ ; sinon raccourcir les cartes et donner la hauteur à la description.
**Coût** : M

### [mineur] finition — Sélecteur de difficulté : « Très difficile » passe seul sur une seconde ligne
**Constat** : trois boutons sur la première ligne, le quatrième orphelin, ligne vide à droite.
**Preuve** : `faction.png` (y≈1055) ; `data/ui/front_end.json:2645`.
**Correction proposée** : 4 colonnes égales ou `HFlowContainer` avec boutons à largeur uniforme, ou « Très dure ».
**Coût** : S

### [mineur] finition — Panneau de colonie : grand vide et lignes dupliquées
**Constat** : l'onglet Garnison laisse environ 200 px vides ; deux lignes identiques « Milice urbaine — 120/120, moral 40 » ne sont pas regroupées (« 2 × »). La valeur « Île-de-France » est décalée de quelques px par rapport aux autres valeurs de la colonne.
**Preuve** : `map.png` (panneau Paris). Fichiers : `game/scripts/ui/settlement_panel.gd`, `army_strip.gd`.
**Correction proposée** : regrouper les unités identiques ; fixer la largeur de la colonne des libellés ; hauteur du panneau au contenu.
**Coût** : S

### [mineur] finition — Raccourcis clavier de la barre supérieure collés au bord des boutons
**Constat** : les pastilles de touche (P, C, T, K, O, G, U, B) sont posées en bas à droite des boutons et mordent le bord, avec un « B » isolé sans libellé près d'Unités.
**Preuve** : `map.png` (barre du haut).
**Correction proposée** : donner une marge à la pastille ou l'intégrer au libellé (« Diplomatie (P) ») ; libeller le bouton B ou lui donner une icône avec infobulle.
**Coût** : S

### [mineur] finition — Chiffres elzéviriens (« Journal (1) », « 1337 ») dans les compteurs
**Constat** : la police de titre affiche un « 1 » en petite capitale ressemblant à un « I » ; confusion possible pour des quantités (« Journal (ı) »).
**Preuve** : `map.png` (en haut à gauche) ; `menu.png` (« 1328-1337 » lisible mais irrégulier).
**Correction proposée** : activer les chiffres alignés (feature OpenType `lnum`) pour les compteurs et résultats, garder l'ancien style pour les dates décoratives.
**Coût** : S

### [mineur] finition — Apostrophes hétérogènes
**Constat** : « d’Angleterre » (typographique) côtoie « d'Angleterre » et « d'infanterie » (droit) dans le panneau de faction.
**Preuve** : `faction.png` (panneau de droite) ; données `data/ui/front_end.json`.
**Correction proposée** : normaliser en ’ dans les JSON (script de contrôle dans `tools/`).
**Coût** : S

### [mineur] finition — Valeurs de repli lisibles mais non français
**Constat** : si un nom manque, l'interface affiche l'id sous forme « Milice urbaine » dérivée de `unit_…`, sans accents ; aucun cas observé aujourd'hui, mais aucun test ne les détecte.
**Preuve** : `army_strip.gd:201`, `game_catalog.gd:90`, `encyclopedia.gd:887,897`, `rich_tooltip.gd:119-128,755`.
**Correction proposée** : test headless qui parcourt `data/` et échoue si un id passe par le repli.
**Coût** : S

### [mineur] conception — Panneau de colonie masque la moitié droite de la carte
**Constat** : le panneau occupe environ 30 % de la largeur (1340-1975) sans pouvoir être déplacé et couvre la Manche et la Flandre. Le journal, replié, reste ouvert à gauche.
**Preuve** : `map.png`.
**Correction proposée** : option de panneau réduit (onglets au survol) ou déplaçable ; les cadres PO l'ont déjà prévu (`UiLayout.SIDE_PANEL`).
**Coût** : M

## 3. À ne pas changer
- Menu de démarrage (fond 3D, titre enluminé, hiérarchie des entrées) : lisible, sobre.
- Cohérence du cadre parchemin (bordures enluminées, barres, boutons rouges d'action principale).
- HUD de bataille et phase de déploiement : bandeau de contexte, messages d'erreur rouges, groupes de formation clairs.
- Panneau de faction (forces/faiblesses, difficulté et défi expliqués), écran de résultat (structure, trophées, étendards).
- Tests de clés d'infobulle : aucune clé manquante.
