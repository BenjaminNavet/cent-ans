# Critique production / chef de projet (22/10/2026)

Périmètre lu : `CLAUDE.md`, `docs/roadmap.md`, `docs/status.md`, `docs/budget.md`, échantillon de `docs/wip/` (465 fichiers), `git log`, branches et worktrees. Aucun client lancé, aucune dépense.

## 1. Feuille de route et état figés au 24/09, avant la migration Bevy

- **Problème** : `docs/roadmap.md` s'arrête à V6 (24/09, 18 lignes, tout « terminé »). `docs/status.md` date du 09/10 et parle encore de Godot, de « 310 tests », de « 0,00 $ / 50 $ », de « mock GDScript ». Rien sur Bevy, les chantiers BL/TS/VQ, la refonte des systèmes. Personne ne peut dire en une page où en est le projet.
- **Preuve** : `git log -- docs/status.md docs/roadmap.md` (dernier commit 09/10 et 17/10 pour un retrait de Godot) ; 994 commits depuis.
- **Gravité** : haute (pilotage à l'aveugle, reprise impossible sans relire 465 notes).
- **Correctif** : réécrire `status.md` en une page (jalons réels M11+ : Bevy, TS, VQ, choc, refonte systèmes ; chiffres actuels) et ajouter à la roadmap les jalons post-V6 avec dates et état. Purger les mentions Godot.
- **Effort** : S

## 2. Aucune définition de « version jouable livrable » ni de vertical slice

- **Problème** : la roadmap déclare « Jeu complet et finalisé » alors que le playtest du 17/10 a produit 7 testeurs + 11 lots de correctifs, que 40 fps en ultra n'est pas atteint, et qu'une « nuit de playtest » démarre le 22/10 avec toutes les cases vides. Pas de critères de sortie (durée de partie, fps cible sur machine cible, absence de bloquants, tutoriel validé par un tiers). Le périmètre « 1337-1453, 29 factions, 3D partout » est livré en largeur, jamais figé en tranche.
- **Preuve** : `docs/status.md` §Où en est-on ; `docs/wip/fix-playtest-1017.md` (restes) ; `docs/wip/nuit-playtest-2026-10-22.md` (état vide).
- **Gravité** : haute.
- **Correctif** : écrire un `docs/release-criteria.md` : une tranche (une faction, 1337-1360, 1 campagne + 3 batailles + 1 siège), seuils (fps, crash 0 sur N parties seed-bots, tutoriel fini sans aide), checklist signée. Tout lot hors critère passe en backlog.
- **Effort** : S (rédaction) puis M (mesure)

## 3. Dérive de périmètre : l'effort va au rendu et aux assets, pas à la boucle de jeu

- **Problème** : sur les ~2 000 commits récents, une grande part porte sur forêts, neige, brume, rochers, sous-bois, plaques de villes (TS-T1 à T10, VQ, MX, DN, VM). Pendant ce temps le cœur ludique reste connu comme faible : ~2,7 révoltes par partie, mêlée « neutre en données », IA de campagne et économie à refondre (`refonte-systemes.md`, « aucun lot n'est implémenté » hors mesure). Le CLAUDE.md autorise 10 agents parallèles ; la capacité est dépensée là où c'est le plus facile à paralléliser.
- **Preuve** : `docs/wip/ts-*.md`, `ts-style-total-war.md` (25 agents) vs `docs/wip/refonte-systemes.md` (chantiers 1 lot A fait, 2-4 non commencés) ; log du 15/10 (543 commits en un jour).
- **Gravité** : haute.
- **Correctif** : geler les lots visuels (hors défauts flagrants, cf. mémoire « seuil de revue des assets ») ; le prochain sprint est exclusivement refonte-systèmes dans l'ordre déjà fixé (révoltes, mêlée, économie/IA), mesuré par seed bots.
- **Effort** : L

## 4. Budget : plafond de 50 $ dépassé de plus du double, discipline à deux vitesses

- **Problème** : 117,39 $ dépensés (OpenRouter 59,82 $, fal 57,57 $) contre un plafond v1 de 50 $, « maintenu comme référence historique » avec des enveloppes nommées en plus. Les crédits Meshy (abonnement) sont hors tableau et suivis dans une liste texte. Les instructions projet disent « plafond 50 $ v1 » : texte et réalité divergent. Aucun plafond global révisé, donc pas de signal d'alerte.
- **Preuve** : `docs/budget.md` synthèse ; `CLAUDE.md` (« plafond 50 $ v1 »).
- **Gravité** : moyenne (dépense approuvée par enveloppes, mais gouvernance floue).
- **Correctif** : fixer un plafond global révisé, signé par le joueur, dans `budget.md` et `CLAUDE.md` ; intégrer les crédits Meshy au tableau (ligne « crédits » séparée) ; rappeler que la nuit du 22/10 a une enveloppe ≤ 5 $ à consigner.
- **Effort** : S

## 5. 994 commits non poussés, aucune sauvegarde distante

- **Problème** : `main` est 994 commits devant `origin/main` (dernier push le 14/10). Le joueur a interdit le push, mais le dépôt est la « mémoire » du projet : un disque mort ou une erreur de worktree (déjà arrivé, cf. mémoire sur les snapshots TM et stash) perd 8 jours de travail. Le dépôt est public : l'écart entre ce qui est publié et la réalité est aussi un risque de lecture trompeuse.
- **Preuve** : `git status -sb` (`devant 994`) ; `git log -1 origin/main` (14/10).
- **Gravité** : haute.
- **Correctif** : demander un go explicite pour pousser vers un remote privé de sauvegarde (ou un `git bundle` hebdomadaire hors dépôt) ; vérifier avant qu'aucun secret ni asset NC n'est dans l'historique. Ne pas toucher au dépôt public sans accord.
- **Effort** : S

## 6. 32 branches non fusionnées, dont le chantier BL entier « en pause » depuis le 17/10

- **Problème** : le backlog BL (traiter tous les items de `docs/backlog.md`, jusqu'à 26 agents) a été coupé le 17/10 ; son travail vit dans ~30 branches `bl/*` et `bl/integ` hors de main, qui diverge depuis. Plus la branche change, plus la fusion coûtera. Les décisions (`bl/decided1-3`, `eqeco`, `eqwar`, `iabat`, `iacamp`, `naval`, `negot`) recoupent la refonte-systèmes (économie, IA de campagne, diplomatie) : risque de double travail ou de conflits de règles.
- **Preuve** : `git branch --no-merged main | wc -l` = 32 ; `git show bl/integ:docs/wip/bl-backlog.md` (EN PAUSE) ; `docs/wip/refonte-systemes.md`.
- **Gravité** : haute.
- **Correctif** : décision binaire avant toute refonte : (a) triage d'une demi-journée de `bl/*` (fusionner les lots prêts, abandonner le reste par une note ADR « abandonné »), ou (b) abandon formel. Ne pas lancer la refonte économie/IA avant ce tri.
- **Effort** : M

## 7. Dette documentaire : 465 notes `wip`, 301 ADR, doublons de numéros

- **Problème** : `docs/wip/` contient 465 fichiers, dont 53 avec des cases ouvertes ; impossible de savoir lesquels sont vivants. Les numéros d'ADR dupliqués (0117, 0146, 0149, 0205, 0357) cassent la règle « une décision écrite n'est pas rediscutée » (référence ambiguë). Les notes sont déjà référencées depuis la mémoire de l'agent comme « archive », mais non déplacées.
- **Preuve** : `ls docs/wip | wc -l` = 465 ; `ls docs/decisions | sed 's/-.*//' | sort | uniq -d`.
- **Gravité** : moyenne.
- **Correctif** : script d'archivage : déplacer vers `docs/wip/archive/` toute note dont le chantier est fusionné et sans case ouverte ; renuméroter ou suffixer (0117b) les ADR en doublon et mettre à jour `INDEX.md` ; un index `docs/wip/README.md` de 10 lignes (chantiers vivants).
- **Effort** : M

## 8. Performance cible non atteinte, mesure non fiable

- **Problème** : l'objectif 40 i/s en qualité ultra n'est pas atteint (playtest 17/10) ; la barrière « 40 fps » du lot VQ est notée « non vérifiée (banc silencieux) », `cperf2` interrompu. Les lots TS ajoutent encore de la charge (brume, sous-bois, ombres, neige) sans budget de frame. C'est le principal risque pour un vertical slice sur machine moyenne.
- **Preuve** : `docs/wip/vq-qualite-visuelle.md`, `docs/wip/fix-perf.md`, mémoire projet (VQ, playtest 10-17).
- **Gravité** : haute.
- **Correctif** : définir une machine de référence et un budget (ms par passe), un banc `--offscreen` reproductible lancé en fin de chaque lot visuel, échec si régression > 10 %. Mesurer d'abord l'état actuel après TS-T10 avant tout nouveau lot de rendu.
- **Effort** : M

## 9. Équilibrage de campagne non validé après les derniers changements de règles

- **Problème** : `status.md` documente des écarts d'équilibrage ouverts (trésor de l'Empire, France dominante, G5). Depuis, ADR 0711 (choc des charges) est mergé avec ses paramètres « neutres en données » et des tests d'équilibre rouges puis verts, et la refonte vient. Aucun état de référence de l'équilibre n'est écrit : on ne peut pas dire si une refonte améliore ou dégrade.
- **Preuve** : `docs/status.md` §Reste ; `docs/wip/choc-charges.md` § Suite ; commits `bb4ff89b9`, `6a8d61d36`.
- **Gravité** : moyenne.
- **Correctif** : figer une ligne de base seed-bots (passe longue, 12 min) dans `docs/wip/seed-bots-baseline.md` avant la refonte ; chaque lot de règles la compare. Appliquer la règle CLAUDE.md : le joueur choisit ce qui entre dans `seed-bots-findings.md`.
- **Effort** : S

## 10. Nuit de playtest sans protocole ni critère de sortie

- **Problème** : la note du 22/10 est vide (cases à cocher non cochées, « Constats : à remplir »). Le mandat (« tout tester toute la nuit », captures réelles, fenêtre visible) est large, en tension avec les règles du CLAUDE.md (budget de 5 captures, focus non volé) qui n'ont été levées que par une phrase. Risque : une nuit de consommation de quota qui livre des constats non triés, comme le playtest 17/10 qui a déclenché 11 lots.
- **Preuve** : `docs/wip/nuit-playtest-2026-10-22.md` ; `docs/wip/nuit-critiques.md`.
- **Gravité** : moyenne.
- **Correctif** : ajouter au début de la note : tranche testée, parcours exact, grille de gravité (bloquant / gênant / cosmétique), plafond de lots de correctifs (max 5 pour les bloquants), sortie au matin en un tableau trié. Les cosmétiques restent au backlog.
- **Effort** : S

## 11. Dépendance à un seul opérateur humain et à des procédures orales

- **Problème** : plusieurs chantiers attendent le joueur : playtest réel du portage Windows (« only real-PC playtest left »), publication des modèles (« classifier blocks »), push, go pour T1R (« plan only, awaiting go »), bloc de crédits Meshy. Aucun tableau des décisions en attente. Les mandats vivent dans la mémoire de l'agent plutôt que dans le dépôt, contre la règle « le dépôt est la mémoire ».
- **Preuve** : mémoire projet (windows, T1R, MX) ; `docs/wip/windows.md`, `docs/wip/t1r-refonte-t1-fal.md`.
- **Gravité** : moyenne.
- **Correctif** : `docs/wip/decisions-en-attente.md` : une ligne par décision attendue du joueur (objet, date, coût, blocage), relue chaque matin. Y déplacer les mandats actuellement seulement dans la mémoire.
- **Effort** : S

## 12. Risque juridique et de diffusion des assets générés

- **Problème** : le projet est public, avec un mélange d'assets générés (OpenRouter, fal, Meshy, TRELLIS, Z-Image) et de ressources tierces (Poly Haven, Freesound CC0, mocap vidéo). Les règles « jamais d'assets NC dans le dépôt » existent, mais aucun inventaire des licences par source n'est la condition d'une release. Les modèles « release pending (classifier blocks) » montrent déjà un point de friction.
- **Preuve** : `docs/budget.md` (fournisseurs) ; mémoire (repo public, relief package, MX models release pending).
- **Gravité** : moyenne (bloque une diffusion publique, pas le développement).
- **Correctif** : un `docs/licences-assets.md` généré par script (source, licence, CGU du fournisseur sur usage commercial, chemin) et une porte « release » qui échoue si une ligne est vide.
- **Effort** : M

## 13. Trop de parallélisme : la capacité d'intégration est le goulot

- **Problème** : jusqu'à 10 agents en parallèle, 25 agents pour TS, 26 pour BL ; 543 commits le 15/10. L'intégration (merges « Merge main into ts/tN » en cascade) et la relecture Opus restent séquentielles : ~30 commits de merge pour 10 lots. Les régressions s'accumulent entre lots (corrections « fix(ts-t7c) », « tighter slope caps » en série) et la vérification humaine n'existe qu'a posteriori.
- **Preuve** : `git log --oneline | head -60` (suite de Merge main into ts/*, corrections de lots déjà fusionnés) ; `docs/wip/ts-style-total-war.md`.
- **Gravité** : moyenne.
- **Correctif** : limiter à 4-6 lots simultanés qui ne touchent pas les mêmes fichiers ; une porte automatique avant fusion (tests + smoke + budget perf) ; une passe seed-bots quick obligatoire par vague.
- **Effort** : M

## Top 3 recommandé

1. **Définir le vertical slice et les critères de sortie (constat 2) et remettre status/roadmap à jour (constat 1)** : sans cible, tout le reste est du développement sans fin. Une demi-journée.
2. **Geler le rendu, mesurer la perf (8), faire le tri des 32 branches BL (6) puis lancer la refonte des systèmes (3)** avec une ligne de base seed-bots (9). C'est la boucle de jeu qui est le vrai manque.
3. **Sauvegarder les 994 commits hors machine (5) et réviser le budget/plafond (4) avec un go explicite du joueur.** Faible effort, risque de perte et de dérive élevé.
