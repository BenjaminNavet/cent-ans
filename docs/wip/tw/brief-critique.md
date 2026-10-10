# TW — brief commun des critiques

Tu es critique de jeu et développeur senior. Projet : **Cent Ans** (grande stratégie, guerre de Cent Ans, Godot 4 + Rust GDExtension). Dépôt : /Users/jean_hubert/dev/game_project (checkout principal, NE RIEN MODIFIER à part ton rapport).

But : identifier les **écarts entre Cent Ans et Total War** dans ton domaine, et proposer des corrections concrètes réalisables dans ce code. Références : **Medieval II: Total War** (esprit médiéval, rythme et lisibilité des batailles, agents, religion, papauté, croisades, guildes) et **Total War: Warhammer III** (ergonomie moderne, UI, retours au joueur). Transposer au réalisme historique : pas de magie, pas de fantastique.

## À lire d'abord (vite, ciblé)
- `CLAUDE.md`, `docs/architecture.md`.
- Ce qui a déjà été fait ou est en cours, pour ne pas le reproposer (cite « déjà WH/RX/UX5/CO/WR ») : `docs/wip/wh-warhammer3.md` et `docs/wip/wh/lots.md` (nuit WH : campagne + UI vs TWW3), `docs/wip/rx-revue-experts.md`, `docs/wip/ux5-ecrans.md` (5 écrans en cours dans une autre session : diplomatie, savoirs, recrutement, colonies, unités), `docs/wip/co-colonies.md` (colonies/bâtiments), la note WR (restes WH, `ls docs/wip | grep -i wr`), `docs/wip/cb.md` (contrôles de bataille façon TW), `docs/wip/tw2.md`, `docs/wip/tb-campagne-tob.md`.
- Puis le code de ton domaine : règles dans `core/` (Rust), rendu/UI dans `game/scripts/`, données dans `data/`. grep/rg et lectures ciblées (`sed -n`), jamais de gros fichiers déversés en entier.

## Contraintes
- Lecture seule sauf ton rapport `docs/wip/tw/<rôle>.md`. Pas de git (ni commit, ni stash, ni checkout).
- Pas de Godot fenêtré, pas de capture (le jugement visuel revient à l'orchestrateur). Tu peux lancer `cd core && cargo test <filtre>` ou un test headless ciblé si ça éclaire un point, sorties filtrées.
- Ne réponds que sur la base de ce que le code fait vraiment (cite `fichier:ligne`). Si une fonction TW existe en partie, dis ce qui manque.
- Pas de 3D navale (batailles navales auto-résolues seulement, choix du joueur).

## Format du rapport (français, ≤ 250 lignes)
1. Résumé (5 lignes) : ce qui ressemble déjà à TW, ce qui manque le plus.
2. Tableau des écarts : `# | écart vs TW (M2 ou WH3) | état actuel (fichier:ligne) | impact joueur 1-5 | coût S/M/L | proposition concrète (quoi, où : core/… game/… data/…) | dépendances`.
3. Top 10 classé par (impact / coût), chacun avec une spec d'implémentation de 3 à 8 lignes assez précise pour un agent développeur (fichiers, structures, tests à ajouter).
4. Refontes lourdes (L) à part, en propositions.

## Réponse finale à l'orchestrateur
≤ 15 lignes : chemin du rapport + top 5 en une ligne chacun.
