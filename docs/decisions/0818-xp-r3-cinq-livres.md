# 0818 — Les cinq livres : cadre « livre à onglets » (XP-R3)

Statut : accepté (lot XP-R3a ; livres Royaume et Chronique) ; amendé par XP-R3b (livres Armées, Cour, Diplomatie)

## Contexte
L'audit d'expérience (2026-10-29, R3) compte environ 22 fenêtres et 10 lanceurs pour la même information. Il propose 5 livres à onglets (Royaume R, Armées A, Cour C, Diplomatie D, Chronique H) et 2 fiches contextuelles. Chaque fenêtre est déjà un constructeur de vue avec son état ; les réécrire coûterait cher et casserait les scripts de fumée.

## Décision
- Un livre est une **rangée d'onglets commune** (`ui/kit/book.rs`) posée en tête de chaque fenêtre qu'il absorbe, décrite en données (`data/ui/books.fr.json`, schéma `ui_books`). Un onglet désigne un couple (panneau, onglet interne). Cliquer un onglet d'un autre panneau envoie `OpenPanel::Hosted { panel, tab }` : le panneau visé s'ouvre (sans basculer) sur l'onglet voulu, les autres fenêtres centrales se ferment. Les vues ne sont pas réécrites ; seule leur rangée d'onglets propre est remplacée par celle du livre, et les panneaux hôtes partagent la largeur `BOOK_WIDTH`.
- Les boutons d'onglet gardent les **anciens noms** (`faction:tab:budget`, `economy:tab:trade`, `chronicle:tab_journal`…). Les lanceurs `hud:open_faction`, `hud:open_economy`, `hud:open_feudal` n'ont plus de bouton (un seul lanceur `hud:open_kingdom`) ; la commande `click` de l'agent et les raccourcis P, M, Maj+F les résolvent encore par la table `aliases` pendant un cycle.
- Livre **Royaume** (R) : Fiche, Budget, Finances, États, Chantiers (panneau de faction), Économie, Commerce, Monnaie (fenêtre Économie), Féodalité, et provisoirement « Ordres » (onglet Cour du panneau de faction, destiné au livre Cour). L'onglet Commerce du panneau de faction fait doublon avec celui de l'Économie : il sort de la rangée du joueur. Livre **Chronique** (H, F5 conservé) : Registre (nouvel onglet qui reprend les affaires et la liste « à surveiller » de l'Audience), Décisions, Lettres, Causes, Journal.
- Touches : R ouvre Royaume (la carte de religion passe à Maj+N), H ouvre la Chronique (la garnison d'armée passe à Maj+H).

## Conséquences
- Les livres Armées, Cour, Diplomatie et l'intendant de province sont des lots suivants : il suffira d'ajouter un livre en données et de remplacer la rangée d'onglets du panneau hôte.
- Le journal et le fil de la carte ne sont pas retirés par ce lot.
- Révise ADR 0002 pour l'interface (un fichier de données par livre, pas par fenêtre).

## Amendement XP-R3b : les cinq livres complets
- **Armées** : Armées (la montre, ex-« Mes unités »), Recrutement, Rançons. La fenêtre des rançons n'a plus de bouton propre (`hud:open_ransoms` est un alias) et s'affiche aussi vide (« Aucun captif ni dette »). Le ravitaillement n'a jamais eu de fenêtre : il reste dans les puces et filtres de la montre.
- **Cour** : les deux écrans du panneau de cour (conseil, fiche de personnage) partagent la rangée du livre : Conseil, Cour, Actes royaux, Chevalerie, Fiche, Compétences, Famille, Décrets (ex-« Ordres » du personnage), Ordres (ex-onglet « Cour » du panneau de faction : chevalerie, appel du pape, croisade ; quitte le Royaume). Le bouton « Conseil » de la fiche disparaît. La salle 3D est déjà le fond de l'onglet Conseil.
- **Diplomatie** : ses trois onglets (Négociation, Guerre, Traités) deviennent la rangée du livre ; la carte diplomatique reste le mode de carte ouvert avec l'écran.
- **Royaume** gagne l'onglet **Techniques** (l'écran des techniques n'a plus de lanceur).
- Plus de bouton pour Conseil, Techniques, Recrutement, Rançons : 6 lanceurs (5 livres + Codex). Alias gardés un cycle : `hud:open_council|tech|recruitment|ransoms|faction|economy|feudal`.
- Touches : celles qui existent restent (C Cour, U Armées, F3 Diplomatie, R Royaume, H/F5 Chronique) ; la lettre A (Armées dans l'audit) et D (Diplomatie) sont la caméra, donc U et F3 sont conservées. J, Maj+R, M, I, Maj+J, F4 restent des raccourcis vers un onglet. Les touches C, U, F3 sont lues par `books.rs`, plus par les panneaux.
- Nouveau message `OpenPanel::Close(panel)` : fermeture d'un panneau hôte sans message de bascule propre.
- **Fiches** : les deux fiches contextuelles sont la fiche de province (avec le siège, panneau arrimé) et la fiche d'armée. L'intendant de province et « Chantiers du royaume » restent à faire (XP-R3c) : ils demandent une règle du cœur (file de chantiers par politique), hors client.
