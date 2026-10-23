# 0775 — Focus du HUD (F9, portée unique), focus initial sûr, avis ordonnés, libellés abrégés, aide de siège

Statut : accepté. Amende ADR 0752 (Conseiller) et 0753 (focus). Lot UX2.

## Contexte
Juge iter-03 (partie B) : F8 introuvable et déjà pris par la console de développement ; Tab puis Entrée choisissait « Non » (tutoriel) ou « Ne plus afficher » (briefing) ; en bataille le focus n'atteignait qu'un bouton (la couche du dessus par `GlobalZIndex`) ; libellés du HUD absents sous 1500 px ; Diplomatie, missions et Chronique s'empilaient à la fin du tour 1 ; le conseiller restait muet pendant les sièges ; 4 échecs de `ui_kit.layout` au choix de faction ; deux lignes pour une même chute de place.

## Décision
- **Touche** : le focus du HUD nu s'engage avec **F9** (`focus::ENGAGE_KEY`), plus F8 (console). Écrite dans l'aide F1 (section « Clavier et manette (sans souris) »). Seul conflit : la surcouche `render` des outils de développement (opt-in `CENT_ANS_DEVTOOLS`), qui garde F9.
- **Portée** : sous une modale, une fenêtre centrale ou un menu d'avant-partie, seule la couche du dessus ; sur le HUD nu, tous les boutons visibles forment une seule portée (barre d'ordres, cartes d'unité, formations, aptitudes, lanceurs de campagne). Bataille : 1 candidat avant, 61 après (déploiement ignoré, dump `ui_focus`).
- **Focus initial sûr** : composant `FocusDefault` (kit), posé sur le bouton de confirmation de chaque modale (« Oui », « Continuer », « À la guerre ! »). Le premier Tab / flèche / F9 y atterrit, sinon sur le premier de l'ordre de lecture. Mesuré : dialogue du tutoriel, Tab donne « Oui ».
- **Manette** : stick gauche = flèches (seuil 0,6, un déplacement par poussée), en plus de la croix, A, B. Toujours non testée sur matériel, dit dans l'aide ; pas de caméra ni d'ordres à la manette.
- **Libellés du HUD** : `data/ui/hud_labels.json` a une forme abrégée (`short` : « Hom. », « Prest. », « Dés. ») utilisée sous 1500 px logiques ; jamais de chiffre nu ; l'infobulle garde le mot entier. `top_bar_fits` vrai à 1024x768, 1280x800, 1440x900, 1920x1080.
- **Avis ordonnés** (fin de tour 1) : la fenêtre Diplomatie ne s'ouvre pas sous une modale en cours (rapport de saison, décision) ; elle attend (`OfferWatch.deferred`, pastille et toast déjà là) puis s'ouvre. L'offre de missions attend une modale ou une fenêtre centrale (toast « elles s'ouvriront dès que l'écran sera libre »). La Chronique ne s'ouvre jamais d'elle-même.
- **Causes vides** : texte explicatif en données (`data/ui/causes.fr.json`, clé `empty`).
- **Choix de faction** : panneaux avec `inner_padding("panel", 3)`, détail du pitch dans un `scroll_body`, panneau de difficulté `flex_shrink: 0` : `ui_kit.layout.failures` 4 → 0 à 1280x800.
- **Doublon du fil** (cœur) : « Affamée, la garnison de X capitule. » ou « se rend sans combattre » et la chute qui suit (avec cause) forment une seule ligne (`siege::capture`).
- **Siège** : un siège mis par un autre devant une place du joueur donne l'alerte `siege_against` (texte de conseil en données : secours, sortie, panneau de la place).

## Conséquences
- Limites : le focus ne fait toujours pas défiler automatiquement vers un bouton hors de vue ; le focus initial « sûr » est le bouton de confirmation (une confirmation destructrice garde donc Entrée = Confirmer, comme le bouton principal à la souris) ; la voix de `adv_siege_against` n'est pas enregistrée (sous-titre seul) ; sondes `ui_kit.layout` de 1 px sur `hud:feed` / `hud:journal` / `advisor:band` sans guide (11,2-11,4 px au lieu de 12) restent ouvertes.
