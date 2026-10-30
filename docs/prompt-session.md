# Prompt de départ d'une session Cent Ans

À coller tel quel au début d'une nouvelle session (Claude Code, Opus en orchestrateur).

```
Tu travailles sur Cent Ans (grande stratégie, guerre de Cent Ans ; core Rust + client Bevy 0.20). Autonomie totale : ne me demande rien, ne t'arrête jamais en attente d'une validation ; ne me sollicite que pour une dépense hors budget ou une action irréversible hors dépôt (pousser, publier une release de jeu ou de modèles).

## Démarrage (peu de jetons)
- Lis CLAUDE.md, docs/architecture.md, puis la note docs/wip/ du chantier en cours (boucle juge : docs/wip/boucle-juge-2.md) et `git log --oneline -20`. Ne refais pas l'exploration : le dépôt est la mémoire, la conversation est jetable.
- Liste `git worktree list` : branches d'agents finies à fusionner, worktrees fusionnés à supprimer (merge-base --is-ancestor, propres).

## Objectifs permanents
- Boucle juge : audits profonds par piliers (visuel, bataille, économie et politique, expérience), puis lots de refonte, sans limite tant que je ne l'arrête pas ou qu'il reste du majeur.
- Le visuel de campagne est la priorité n° 1 : réaliste, semi-réaliste, jamais low poly.
- 60 fps en Ultra (Mac/Metal d'abord). Mesures de fps seulement sur machine calme.
- Recentrer le jeu sur 6-8 maisons jouables (R6).
- Crédits Meshy restants à dépenser ; Z-Image pour la 2D ; TRELLIS 2 interdit sur fal.

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
1. Opus planifie, découpe, intègre. Sous-agents Haiku par défaut, Sonnet seulement pour les lots durs (IA de bataille, économie, shaders) ; jamais Opus.
2. 3-4 agents en parallèle au plus. Sérialiser les lots qui touchent la même zone (même shader, même module de règles, mêmes registres).
3. Réserver le numéro d'ADR au lancement du lot ; l'agent l'écrit directement (pas de XXXX, pas de renumérotation).
4. Brief d'agent : spec courte, fichiers, tests, numéro d'ADR, « départ = main local » (`git merge-base HEAD main`, sinon `git reset --hard main`), symlinks des assets ignorés (data/map/pyramid, game/assets/models/dn). Rapport ≤ 15 lignes (résultat, branche, échecs, points ouverts) ; le détail va dans docs/wip/<lot>.md.
5. Dans les agents : seed bots --quick seulement ; pas de passe longue, banc court. Une passe longue sur main par lot de fusions, en arrière-plan.
6. Fusions par un agent de fusion Sonnet par lot de 2-4 branches (rebase sur main, conflits = union des deux côtés, ADR, ff-merge), pas par la session principale.
7. Un seul check complet sur main par lot de fusions (tools/full_check.sh + smoke + pytest + seed quick).
8. Lecture ciblée : grep, head, dumps d'état filtrés ; jamais de gros fichiers ni de sorties brutes. Captures : 5 par tâche (10 en travail visuel), 640 px, une seule après un groupe de corrections ; les agents mécaniques n'en prennent pas.
9. Ne jamais finir un tour « en attente » : attendre agents et checks par Monitor ou boucle until en arrière-plan, puis enchaîner.
10. Mettre à jour la note wip du chantier à chaque lot de fusions ; la session suivante reprend depuis elle. Une session par vague, /clear entre deux chantiers.
```

Avant de lancer : désactiver dans `/mcp` les serveurs inutiles au projet (rea, Chrome, Gmail, Calendar, Drive, Hugging Face) ; dans `/config`, modèle de sous-agent par défaut = Haiku.
