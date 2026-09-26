# 0084 — Pas de compilation incrémentale en profil dev

Date : 2026-09-26. Statut : accepté.

## Contexte

Le 26 septembre, le disque est tombé à 25 Go libres (98 %). Plusieurs sessions Claude
travaillaient en parallèle sur la copie `main`, et chacune compilait `core/` dans le même
`core/target`. Ce cache de compilation pesait alors 51 Go, dont 29 Go dans
`debug/incremental`.

La compilation incrémentale conserve un état du crate par « session » de compilation. Quand dix
sessions modifient des fichiers différents et compilent tour à tour, leurs états ne se
réutilisent pas et s'accumulent. Ils ont tous moins d'un jour, si bien que
`cargo sweep --time 1` ne les retire pas.

## Décision

`[profile.dev] incremental = false` dans `core/Cargo.toml`. Le réglage vaut pour tout l'espace
de travail, et le profil `test` en hérite.

## Conséquences

- `core/target` ne garde plus que `deps` : environ 20 Go au lieu de plus de 50 Go.
- Après une modification, nos crates se recompilent en entier, alors que les dépendances
  restent en cache. Ils sont de toute façon compilés en `opt-level` 2 ou 3 (voir les
  `[profile.dev.package.*]`), niveau auquel l'incrémental faisait déjà gagner peu de temps.
- Le changement de profil provoque une recompilation complète, une seule fois, dans chaque
  worktree.
- Un développeur seul qui veut retrouver l'incrémental peut lancer
  `CARGO_INCREMENTAL=1 cargo build` : la variable d'environnement l'emporte sur le profil.
- Hygiène disque associée : supprimer les worktrees fusionnés et leur `core/target`, puis
  lancer `tmutil thinlocalsnapshots / 999999999999 4`. Les instantanés Time Machine locaux
  retiennent les fichiers effacés : le 26 septembre, ils en gardaient environ 290 Go.
