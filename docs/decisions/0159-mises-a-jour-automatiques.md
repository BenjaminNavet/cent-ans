# 0159 — Mises à jour automatiques : le lanceur suit la branche `stable`

Date : 2026-10-02. Statut : accepté. Complète les ADR 0117 (lanceur depuis les sources) et 0153
(lanceur Windows).

## Contexte

Les joueurs lancent le jeu depuis un clone, avec `Lancer Cent Ans.*` → `tools/launch.sh`, qui
recompile le cœur, réimporte et télécharge le relief quand quelque chose a changé. Il manquait
le premier maillon : récupérer les commits. Chaque joueur devait faire `git pull` à la main.

Suivre `main` directement livrerait chaque push tel quel, y compris un push qui ne compile pas
ou qui casse le démarrage. Un jeu exporté qui se télécharge lui-même (≈ 800 Mo par version,
runner macOS, exécutable non signé) est un autre chantier, à reprendre pour une sortie publique.

## Décision

1. **Une branche `stable`, déplacée par la CI seulement.** `windows.yml` tourne désormais à
   chaque push sur `main` (hors pushes ne touchant que `docs/`). Quand le job `smoke` passe
   (compilation du cœur, import, `smoke.gd` par le lanceur, sous Windows), le job `promote`
   avance `stable` sur ce commit par l'API GitHub, **en avance rapide uniquement**
   (`force=false`). Il crée la branche au premier passage vert. Personne ne pousse sur
   `stable` à la main.
2. **Le lanceur se met à jour s'il est sur `stable`, et seulement là.** Étape 0 de
   `tools/launch.sh` : `git fetch <remote> stable`, puis `git merge --ff-only`, puis le
   lanceur se relance lui-même (`--no-update`) pour que la suite s'exécute avec le nouveau
   script. Un checkout de développement (`main`, branches, worktrees) n'est jamais touché :
   une ligne rappelle seulement comment activer les mises à jour.
3. **Jamais bloquant, jamais destructeur.** Hors ligne, clone divergé, fichier modifié sur
   place que la mise à jour toucherait : le lanceur le dit en une ligne et lance la version
   installée. Aucun `reset`, `stash` ni `checkout` forcé : git garde les modifications locales
   ou refuse.
4. **Désactivation** : `--no-update`, `CENT_ANS_NO_UPDATE=1`, et toujours en CI (`CI` défini :
   le workflow teste le commit qu'il a extrait).
5. **Exécutable Windows en cours d'exécution.** Windows interdit d'écraser un `.exe` qui
   tourne mais permet de le renommer. Quand la mise à jour modifie `Lancer Cent Ans.exe`, le
   lanceur le renomme en `.exe.old` (ignoré par git, supprimé au lancement suivant) et met une
   copie identique à sa place avant la fusion.
6. **Concurrence CI** : un run à la fois par branche, seul le dernier en attente est conservé.
   Une rafale de pushes ne lance pas un run par commit et `stable` avance quand même.

## Conséquences

- Un joueur installe avec `git clone -b stable …` ; un clone existant passe sur `stable` une
  fois (`git fetch && git switch stable`). Ensuite un double-clic suffit.
- Délai entre un push et sa disponibilité : la durée du run Windows (≈ 20 à 30 min).
- Le filtre est le smoke Windows : un défaut propre à macOS ou Linux, ou un défaut de jeu que
  le smoke ne voit pas, atteint quand même `stable`.
- Réécrire l'historique de `main` (comme la purge des captures du 30/09) fait échouer
  `promote` : c'est voulu. Il faut alors déplacer `stable` à la main, et les clones des joueurs
  doivent être refaits ou réalignés (`git reset --hard origin/stable`).
- Un joueur qui modifie un fichier suivi reste bloqué sur sa version dès qu'une mise à jour
  touche ce fichier ; le message de git nomme le fichier.
- Le point 5 n'est pas vérifié sur un vrai Windows (la CI saute la mise à jour). Les autres
  cas sont couverts par `tools/tests/test_launch_update.py` (dépôts git temporaires).
