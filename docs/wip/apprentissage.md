# Apprentissage — première campagne guidée, cap lisible, intendant (wip)

Demande du joueur (30/10) : « revoir le gameplay pour que les nouveaux joueurs comprennent plus facilement ».
Accord donné sur 3 chantiers + le guide générique pour les 5 maisons secondaires. Source : audit
`docs/audits/2026-10-29-experience.md` (C2, C4, R2, R3 intendant) et juge `docs/audits/2026-10-30-juge-jeu-iter3.md` (M2, M3).
ADR : 0848 (apprentissage), 0849 (intendant). À lancer **après FUSION-D** (fichiers communs : progress.rs, agenda.rs, tutoriel).
Agents : Sonnet 5.5, effort medium (mémoire 10-30). Chaque lot : worktree réinitialisé sur main local, squelette commité d'abord.

## État
- [ ] APPR-1 cœur + données
- [ ] APPR-2 client
- [ ] CAP
- [ ] INTENDANT

## Lot APPR-1 — scénario « Printemps 1337 », cœur et données
Remplace le guide-visite (`data/tutorial/steps.json`, 18 étapes) par des **chapitres à objectifs d'état**.
- `data/tutorial/chapters.json` + schéma `data/schemas/tutorial_chapters.schema.json` : 5 chapitres
  (royaume 1 saison, ost 2-3, première bataille 4, siège 5-6, monde 7-8) ; par chapitre : objectif d'état, texte « pourquoi »
  (l'arbitrage enseigné), conseils par maison (France, Angleterre, Bourgogne) + **variante générique** (Bretagne, Navarre,
  Écosse, Flandre, Castille : mêmes chapitres, cibles tirées de l'état de la maison, sans but historique propre).
- But historique de scénario par grande maison (ex. Angleterre : tenir la Guyenne et prendre pied en Flandre avant l'hiver 1339).
- `game_api::progress` : conditions **d'état** (armée ≥ N hommes avec vivres ≥ K saisons, chantier lancé au meilleur
  rendement, bataille livrée, siège mené, offre diplomatique traitée, décision tranchée). Reprendre les conditions de TUTO
  (recrutement, siège, ravitaillement) ; supprimer les conditions « panneau ouvert ».
- **Blocage de fin de saison** : tant que l'objectif du chapitre n'est pas atteint, la commande fin de tour renvoie
  « Le conseil attend : <objectif> » ; commande explicite `SkipChapter`. Hors scénario : aucun blocage.
- Mode de partie `apprenticeship` dans la config de partie ; à la fin du chapitre 5, la partie continue normalement.
- Tests : chaque chapitre atteignable par un script de commandes (seed fixe) pour les 8 maisons ; fin de tour bloquée puis
  débloquée par `SkipChapter` ; seed bots : aucune maison coincée dans un chapitre plus de 3 saisons.

## Lot APPR-2 — client du scénario
- Menu principal : « Première campagne (apprentissage) » à côté de « Nouvelle partie » ; choix parmi les 8 maisons, les 3 grandes en avant.
- Carte de chapitre (réutiliser le parchemin du guide et l'Audience) : objectif, pourquoi, progression ; sceau de fin de tour
  « Le conseil attend : … » + bouton « Passer ».
- **Première bataille de campagne** : la couche didacticiel de `battle_prologue.json` s'active automatiquement (mode apprentissage,
  ou première bataille d'une partie avec Réglages > Tutoriel).
- Dévoilement progressif pendant le scénario : livres et onglets non encore présentés (Féodalité, Monnaie, Commerce, Chevalerie)
  repliés avec « présenté au chapitre N », accessibles quand même.
- Hors scénario, la case Réglages > Tutoriel garde des conseils courts déclenchés par la situation. F1 et l'encyclopédie restent.
- Vérif : scénario de smoke `apprenticeship` (5 chapitres, `SkipChapter`, bataille avec couche), dump d'état ; pas de capture (agent Sonnet).

## Lot CAP — un cap visible dès la saison 1
- Ambitions : Affaire de l'Audience au tour 1 avec un **choix recommandé pré-coché** ; une seule fois, puis rappel discret (pas à chaque saison, cf. juge M3).
- Jauge « où va le royaume » (provinces, trésor, renommée sur 4 saisons : tendance + projection) dans le livre Royaume et au Récit de saison.
- Prochain jalon chiffré à côté des objectifs (« +4 si vous prenez Calais ») et gain de la saison écoulée ; avertissement « en retard sur votre acte ».
- Cœur : vue `game_api` (calculs dans `core/`), seuils en données (`data/rules/`). Tests + seed bots : le score bouge dans les 4 premières saisons pour les 8 maisons.

## Lot INTENDANT — déléguer les chantiers (ex-XP-R3c, partie intendant)
- Règle du cœur : intendant par province avec politique (`best_yield`, `fortify`, `none`) qui remplit les localités libres dans
  la limite d'un budget par saison ; politiques en données. ADR 0849.
- Activé par défaut en mode apprentissage et pour les nouvelles parties (réglable par province et globalement).
- L'alerte « N localités sans chantier » disparaît ; ligne d'agenda seulement si le trésor dort (ADR 0749).
- Client : sélecteur dans la fiche de province + bascule globale dans « Chantiers du royaume ».
- Tests : un tour avec intendant remplit les emplacements sans dépasser le budget ; seed bots : plus de trésor dormant au départ France.

## Prochaine étape
Attendre la fusion de FUSION-D dans main, puis lancer APPR-1, CAP, INTENDANT en parallèle ; APPR-2 après APPR-1.
