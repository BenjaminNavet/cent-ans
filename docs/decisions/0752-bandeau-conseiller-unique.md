# 0752 — Un seul bandeau Conseiller

Statut : accepté. Lot UX (juge iter-02, partie B, constats 2 ; partie art 6 et 18).

## Contexte
Au tour 0 de la campagne, quatre zones de conseil se disputaient l'écran : le journal, la carte « Conseil » (`hud/hint.rs`, « Y aller / Masquer »), la bulle ou le sous-titre de Jean le Bel (toast en haut au centre en campagne, `AdvisorBubble` en bataille) et les alertes, sans compter la carte du guide. Le texte du conseiller restait figé au centre du haut de l'écran, masquait la carte, et rien ne gardait la trace de ce qui avait été dit.

## Décision
- `ui/advisor_band.rs` : le bandeau Conseiller est la **seule** zone de conseil. Toute source envoie un message `BandSay { key, text, priority, seconds, action, standing }` ; l'état pur `BandState` (testé) tient une file triée par priorité (`Info < Advice < Warning < Urgent`, FIFO à priorité égale), **un seul message actif**, le temps de lecture (`4 s + 0,33 s par mot`, 4 à 16 s, gelé tant que le pointeur est sur le bandeau), le **repli automatique** après lecture, et l'**historique** (40 derniers) lu par le journal (« Conseiller », 4 dernières lignes dans le journal déplié).
- Un message de priorité supérieure préempte l'actif. Un message `standing` (un conseil qui reste vrai : « choisissez une technique ») se replie en **puce** « Conseiller ▸ » au lieu de disparaître ; un clic le rouvre ; la source le retire par `AdvisorWithdraw`. Un conseil inchangé et déjà replié ne ressurgit pas.
- Boutons : `advisor:go` (action principale, légende donnée par la source), `advisor:later` (« Plus tard » pour un conseil durable, « Compris » sinon), `advisor:chip`. Le bandeau émet `AdvisorAction { key, kind }` ; la source agit (`hud/hint.rs` : mêmes actions qu'avant).
- Hôtes : campagne, bas gauche au-dessus de la bande d'armée (`hud/mod.rs`, `advisor:host`) ; bataille, tête de l'emplacement `Journal` (`audio/advisor.rs`). Tant que la carte du guide est affichée (même zone), `AdvisorBand.suppressed` masque tout sauf l'urgent (un seul narrateur à la fois, cf. ADR 0446).
- Sources branchées : le conseil « que faire maintenant » (textes `data/ui/next_hints.json`), les sous-titres du conseiller vocal (`audio/advisor.rs`, textes `data/voice/advisor.json`). L'ancien `AdvisorBubble` et la carte `hud:hint` disparaissent ; les alertes (cloche, pastilles de droite) restent séparées car ce sont des états, pas des conseils.
- Message renommé `BandSay` : `audio::advisor::AdvisorSay` (déclencheurs vocaux) existe déjà.

## Conséquences
- Une zone de conseil au lieu de quatre ; le tour 0 n'affiche plus que le guide (bas gauche) puis, quand il s'efface, le bandeau.
- Les scripts qui cliquaient `hud:hint_go` / `hud:hint_hide` cliquent désormais `advisor:go` / `advisor:later` (aucun script du dépôt n'y faisait référence).
- Limite : la liste des 14 étapes du guide reste dans la carte du guide (`tutorial:toc`) ; son déplacement dans l'écran Aide (F1) n'est pas fait.
