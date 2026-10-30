# Prompt de départ d'une session Cent Ans

À coller tel quel au début d'une nouvelle session (Claude Code, Opus en orchestrateur).

```
Tu travailles sur Cent Ans (grande stratégie, guerre de Cent Ans ; core Rust + client Bevy 0.20). Autonomie totale : ne me demande rien, ne t'arrête jamais en attente d'une validation ; ne me sollicite que pour une dépense hors budget ou une action irréversible hors dépôt (pousser, publier une release de jeu ou de modèles).

## Démarrage (peu de jetons)
- Lis CLAUDE.md, docs/architecture.md, puis la note docs/wip/ du chantier en cours (boucle juge : docs/wip/boucle-juge-2.md) et `git log --oneline -20`. Ne refais pas l'exploration : le dépôt est la mémoire, la conversation est jetable.
- Liste `git worktree list` : branches d'agents finies à fusionner, worktrees fusionnés à supprimer (merge-base --is-ancestor, propres).

## Objectifs
Méthode : boucle juge sans limite (jusqu'à mon arrêt ou plus rien de majeur). Chaque itération = des juges frais (visuel avec captures, technique, bataille, campagne, économie, expérience) → constats → lots de refonte → fusion → mesure. Les audits du 29/10 (docs/audits/2026-10-29-*.md) donnent le détail de chaque refonte (spec, critères, coût) : les suivre, ne pas les réécrire. État à jour : docs/wip/boucle-juge-2.md.
Ambition : les juges font des critiques ambitieuses, pas des retouches ; les lots sont des refontes larges, qui peuvent toucher l'architecture du jeu (modules, flux de données, ADR révisés si un juge le justifie). Une itération qui ne change que des réglages est un échec de jugement.

### 1. Visuel de campagne (priorité n° 1, à challenger le plus)
Cible : la carte doit ressembler à une photo aérienne médiévale peinte, semi-réaliste, jamais low poly ni « jeu de plateau gris ». Audit : docs/audits/2026-10-29-visuel-campagne.md (défauts G1-G3, refontes R1-R8, bible de palette).
- Fait : ciel et lumière (R1), frontières en bande (R2), carte de couleur rebâtie (R3a), eau et bathymétrie (R5) + rivières lisibles, lieux « ouest » Meshy (R6b), végétation de plaine (R7), horizon.
- En cours / à faire, dans l'ordre : R3b parcellaire typé (openfield, bocage, vigne, saisons par culture) ; R4 montagne étagée et neige vraie (étages par altitude et exposition, glaciers, brume de vallée) ; R8 parchemin qui monte sur la 3D (fondu 900-1 600, glyphes d'encre Z-Image) ; heure du jour et nuit (G3-3) ; brume lointaine (G3-5).
- Restes connus : routes peu lisibles sur la carte de couleur ; rivières pâles à d 900 et bande riveraine ; plaque de Saint-Denis qui chevauche la cathédrale de Paris (occlusion des plaques d'UI) ; plaques d'horizon en bataille ; haies de bocage ; multiplicateurs de saison codés en dur ; paquet de modèles non republié (attendre mon feu vert).
- Critère : un juge visuel frais, captures 640 px sur les vues de tools/refshots/views.txt, ne trouve plus de défaut « flagrant » (G1) ; mesures HSV contre la planche de palette.

### 2. Performance
- 60 fps en Ultra (16,7 ms) en campagne ET en bataille de mêlée, sur Mac/Metal. Mémoire de campagne sous contrôle (était ≈ 3 Go).
- Mesurer VIS-R1/R2/R6/R7 et chaque refonte visuelle au banc tools/bevy_bench, seulement machine calme (lot FPS lancé seul). Branches à reprendre : MEM3 (mesure mémoire) et FPS mêlée (pré-chauffe des pipelines).

### 3. Bataille
Audit : docs/audits/2026-10-29-bataille.md. Fait : BANC-1/2 (banc d'équilibre, auto-résolution à 80,8 %), IA-CAV (charge groupée), IA-PLAN-1 (planificateur, plan « Tenir »).
- À faire : IA-PLAN-2 plans offensifs (échelons, fixer-déborder, provoquer ; Crécy et Castillon à corriger) → IA-PLAN-3 lisibilité du plan ennemi et des déroutes → COHESION-1 fatigue et cohésion d'armée → COHESION-2 fin de la cadence historique, réglage tir/choc → BANC-3 (tests d'issue = bandes du banc, vitrine historique). Plus tard : avant-garde/bataille/arrière-garde (R2), siège gagné en campagne (R4), captifs et rançons (R5).
- Critères : le banc ne régresse jamais ; les batailles historiques (Crécy, Poitiers, Azincourt, Formigny, Castillon, Auray) donnent l'issue historique majoritairement ; moins de duels sans contact ; hommes d'armes à pied qui tiennent une charge frontale.

### 4. Économie et politique — EN PAUSE (30/10, décision du joueur : ni juge ni lot économique jusqu'à nouvel ordre ; ECO-R2b, en cours, est le dernier lot mené à terme)
Audit : docs/audits/2026-10-29-economie-politique.md. Fait : R0, R1-a/b/d impôt consenti (États, Parlement, subsides, concessions, onglet États), R2-a semonce féodale.
- À faire : R1-c (IA, Angleterre, maltôte ; part de l'armée de la France IA 17-22 % → 35-60 %) ; R2-b lettres de retenue ; R2-c compagnies nées de la paix et des arriérés ; R3 princes et apanages ; R4 légitimité et couronne ; R5 crises nées des causes (peste, travail, révoltes).
- Critère : sonde eco_series (3 graines) dans les bandes de l'audit ; pas de banqueroute en chaîne ; seed bots dans leurs bandes.

### 5. Campagne et récit
Audit : docs/audits/2026-10-29-campagne.md. Fait : R1 (renommée accumulée, actes à dates fixes, ambitions).
- À faire : R6 recentrer sur 6-8 maisons jouables, moins de bruit, des décisions lourdes ; R2 la guerre en saisons (trêve distincte de la paix, chevauchée qui pèse, levées d'été et quartiers d'hiver) ; R5 IA qui joue la partie (plans de campagne, buts d'acte) ; R4 horloges de crise (Grandes Compagnies, peste récurrente, révolte fiscale) ; R3 couronne contestée.
- Dérives seed bots à traiter (après mon choix) : score de règne trop bas, part de l'armée, révoltes, banqueroutes, éliminations.

### 6. Expérience du joueur
Audit : docs/audits/2026-10-29-experience.md. Fait : R1 Audience de saison (sollicitations 61 → 12 par 10 saisons), R4 « Pourquoi ? », R6 lisibilité (typographie, glossaire).
- À faire : R3 « les cinq livres » (22 fenêtres → 5 livres et 2 fiches) ; R2 « Printemps 1337 », première campagne-scénario ; R5 voix du chroniqueur (récit plutôt que catalogue) ; restes d'XP-AUDIENCE (widgets au repos −14 % → −20 %, filtres du Registre, textes encore codés dans objectives.rs, liste des crises à valider).

### Assets
Meshy : crédits restants (≈ 25) pour une retouche, sur décision visuelle. Z-Image local gratuit pour toute la 2D (textures de culture, étages alpins, glyphes d'encre). TRELLIS 2 interdit sur fal ; jamais de low poly ; figures en A-pose. Dépenses dans docs/budget.md (plafond 50 $).

## Règles du projet (rappel, CLAUDE.md fait foi)
- Règles de jeu dans core/ ; données dans data/ validées par data/schemas/ ; client = rendu, UI, entrées.
- Code et commits en anglais, docs et UI en français. Pas de Co-Authored-By. `git commit -m … -- <chemins>`, jamais -a, add -A ni stash. Ne pousse rien.
- Jamais voler le focus ni faire de bruit : client via client/run_bg.sh, CENT_ANS_MUTE=1, --offscreen.
- Commandes lourdes via tools/heavy.sh ; les longues en arrière-plan.
- Avant fusion : fmt, clippy -D warnings, tests (core, client --features dev), smoke, pytest, seed bots --quick.
- Toute modif de data/unit_types ou data/rules/battle_* : régénérer la fixture auto-resolve (ADR 0807).
- ADR pour toute décision d'architecture ; dépenses cloud dans docs/budget.md.
- Constats de la passe longue seed bots : présentés au joueur, rien dans seed-bots-findings.md sans lui.

## Économie de jetons (obligatoire)
1. Opus planifie, découpe, intègre. TOUS les sous-agents (implémentation, juges, fusion, recherche) sont Sonnet 5.5 en effort medium : `model: "sonnet"`, `effort: "medium"` à chaque appel Agent. Jamais Haiku. Seule exception : le juge visuel est Opus (`model: "opus"`). Les juges écrivent constats et lots proposés (spec, critères) dans docs/audits/ ; l'orchestrateur en tire les briefs d'agents.
2. 10 agents en parallèle au plus. Sérialiser les lots qui touchent la même zone (même shader, même module de règles, mêmes registres).
3. Réserver le numéro d'ADR au lancement du lot ; l'agent l'écrit directement (pas de XXXX, pas de renumérotation).
4. Brief d'agent : spec courte, fichiers, tests, numéro d'ADR, « départ = main local » (`git merge-base HEAD main`, sinon `git reset --hard main`), symlinks des assets ignorés (data/map/pyramid, game/assets/models/dn). Premier pas de l'agent : `tools/warm_worktree.sh` (clone copie-sur-écriture des dépendances compilées de main, évite la compilation à froid). Rapport ≤ 15 lignes (résultat, branche, échecs, points ouverts) ; le détail va dans docs/wip/<lot>.md.
5. Dans les agents : `cargo check` et tests ciblés du module touché ; clippy, tests complets et smoke une seule fois, à la fusion. Si la file de tools/heavy.sh dépasse 15 min d'attente, lancer moins de lots. Aucune sonde dans les agents (seed bots, battle_bench, eco_series, fixture auto-resolve, smoke) : elles tournent uniquement à la fusion finale dans main, choisies par `tools/checks_for_diff.sh`. Exception : les lots d'IA de bataille peuvent lancer battle_bench (≈ 30 s) et les lots d'équilibre économique eco_series, pour régler en connaissance de cause. Un critère de sonde non tenu à la fusion renvoie le lot en correction. Une passe longue seed bots sur main par lot de fusions, en arrière-plan.
6. Fusions par un agent de fusion Sonnet 5.5 (effort medium) de préférence une seule fois pour tous les lots de l'itération ; si un lot retarde les autres, fusionner par paquets de lots prêts sans l'attendre (rebase sur main, conflits = union des deux côtés, ADR, ff-merge), pas par la session principale.
7. Un seul check complet sur main par lot de fusions (tools/full_check.sh + smoke + pytest + seed quick) ; `tools/checks_for_diff.sh <base>` liste, d'après les fichiers modifiés, les vérifications utiles (battle_bench, eco_series, seed bots, smoke, pytest, fixture auto-resolve).
8. Lecture ciblée : grep, head, dumps d'état filtrés ; jamais de gros fichiers ni de sorties brutes. Captures : 5 par tâche (10 en travail visuel), 640 px, une seule après un groupe de corrections ; les agents mécaniques n'en prennent pas.
9. Ne jamais finir un tour « en attente » : attendre agents et checks par Monitor ou boucle until en arrière-plan, puis enchaîner.
10. Mettre à jour la note wip du chantier à chaque lot de fusions ; la session suivante reprend depuis elle. Une session par vague, /clear entre deux chantiers.

```

Avant de lancer : désactiver dans `/mcp` les serveurs inutiles au projet (rea, Chrome, Gmail, Calendar, Drive, Hugging Face) ; dans `/config`, modèle de sous-agent par défaut = Sonnet (claude-sonnet-5-5).
