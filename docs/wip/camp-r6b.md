# CAMP-R6b : ambitions des 5 maisons restantes

État : jeux écrits (60 ambitions, 4 par acte) dans `data/rules/acts.json` › `ambition_sets` pour fac_brittany,
fac_navarre, fac_scotland, fac_flanders, fac_castile. Préfixes d'id br/na/sc/fl/ca ajoutés au schéma
`acts_rules.schema.json`. Test `seed-bots/tests/ambitions.rs` étendu aux 8 maisons : chaque ambition déplace une
grandeur de la sonde de plus de 10 % sur 40 tours (effets renforcés en prestige/impôt/commerce pour les maisons
à petite économie).
Écart : Flandre acte III non sondable (une province, rayée par le saut d'horloge de la sonde à 1416), exclue du test.
Pas de codex d'ambitions dans le modèle (rien à ajouter).
Reste : fmt, clippy, tests core, seed bots --quick, pytest schémas.

TERMINÉ : fmt, clippy, tests core, seed bots --quick, pytest schémas passés.
