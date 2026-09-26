# Nettoyage disque — WIP

Mis à jour le 2026-09-26. Statut : **fait (lot 3 le 26/09 au soir)**, à relancer si le disque se remplit. Mandat : une fois que les sessions ont fini leur
travail, supprimer les reliquats (worktrees fusionnés, caches `core/target`).

## Contrainte

Le mode auto refuse à l'agent les suppressions (`git worktree remove`, `rm -rf`). L'agent
prépare la commande, et le joueur la lance lui-même avec le préfixe `!`.

## Fait

- **Lot 1** : 12 worktrees fusionnés supprimés, avec leurs branches (`gp-suites3/zoom/relief/historien/tw-merge`,
  `gp-br1`, `gp-br1-base`, `sv1-4`, `m5a`), plus `game_project-mapmodes/core/target`.
- **Lot 2** : 4 worktrees d'agents verrouillés mais fusionnés et inactifs (`adc8c89`, `a19e7d9`,
  `a81b596`, `abf402a` = `da6`), `gp-epic-merge` et `gp-night-merge/core/target` supprimés.
- `cargo sweep --time 1` sur `core/target` de `main` : 10 Go retirés.
- `tmutil thinlocalsnapshots / 999999999999 4` : de 25 Go à 318 Go libres. **Leçon :** les
  instantanés Time Machine locaux retiennent tout ce qu'on supprime, et c'est le levier n° 1.
- ADR 0084 (`3bff25b8`) : `[profile.dev] incremental = false`. Les sessions parallèles
  empilaient 29 Go de `debug/incremental`.

- **Lot 3** (après la pause) : `core/target/debug/incremental` de `main` (30 Go) supprimé ;
  caches `core/target` des worktrees en pause vidés (`eq6` 13 Go, `pb3f`, `pb3g`, `aacdb07`) ;
  `gp-night-merge` et `gp-sz-merge` supprimés (branches `integration/night` et `integration/sz`) ;
  8 instantanés Time Machine purgés. Résultat : **397 Go libres (56 %)**.

## État à la pause (historique, avant le lot 3)

- 209 Go libres (77 %), 17 worktrees, environ 9 sessions Claude actives.
- `core/target` de `main` : 67 Go, dont **30 Go de `debug/incremental`** laissés avant l'ADR 0084.
  Ce dossier est inutile désormais : le supprimer quand aucun `cargo` ne tourne.
- Prêts mais gardés volontairement : `gp-night-merge` (2,6 Go, cache déjà vidé) et `gp-sz-merge`
  (2,5 Go, branche `integration/sz`). Ce sont des worktrees d'intégration qu'un orchestrateur
  peut réutiliser.

## Outils

- `tools/disk_cleanup/cleanup_status.sh` : liste les éléments `READY`, c'est-à-dire les worktrees
  fusionnés, sans modification suivie, sans processus dedans, et inactifs depuis 30 min (60 min
  s'ils sont verrouillés). Il inclut `core/target` de `main` si aucun cargo ne tourne et que rien
  n'y a bougé depuis 30 min.
- `tools/disk_cleanup/cleanup_cmd.sh` : génère la commande `!` correspondante (déverrouillage,
  suppression du worktree, puis `git branch -d`).

## Restant (à garder tant que le travail n'est pas fusionné)

Worktrees non fusionnés : `gp-da` (DA2), `gp-dc3` et `gp-densite` (DC, actifs), `fg3-materials`,
`sz4b` (verrouillé), `pb3f`/`pb3g`, `eq6`, `mapmodes`, `ade9b18`, `aacdb07`. À supprimer une fois
fusionnés ou abandonnés.

## Prochaine étape (reprise)

1. `df -h /System/Volumes/Data` et `pgrep -x claude | wc -l` : les sessions sont-elles finies ?
2. `bash tools/disk_cleanup/cleanup_cmd.sh` : donner la commande au joueur.
3. Si aucun cargo ne tourne : `rm -rf core/target/debug/incremental` (30 Go).
4. En fin de journée : `gp-sz-merge`, `gp-night-merge`, et les worktrees non fusionnés restants
   une fois leur travail intégré ou abandonné (toujours vérifier `git log main..<branche>`).
5. Toujours terminer par `tmutil thinlocalsnapshots / 999999999999 4`.
6. Sinon, relancer la surveillance : Monitor qui exécute `cleanup_status.sh` toutes les 5 min
   et n'émet que les nouveaux éléments prêts.
