# 0818 — Les cinq livres : cadre « livre à onglets » (XP-R3)

Statut : accepté (lot XP-R3a ; livres Royaume et Chronique)

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
