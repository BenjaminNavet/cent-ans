# CAMP-R6 « Recentrer » — maisons jouables

État : TERMINÉ. Données `playable` posées (8 vrai, le reste faux), ADR 0817.

## Ambitions (ADR 0805), `data/rules/acts.json` › `ambition_sets` / `objective_sets`
Écrits : France, Angleterre, Bourgogne (+ `default`). MANQUENT (non écrits, hors lot) : Bretagne (fac_brittany),
Navarre (fac_navarre), Écosse (fac_scotland), Flandre (fac_flanders), Castille (fac_castile) — elles utilisent `default`.

## Prochaine étape
Aucune. Vérifié : fmt, clippy core+client, tests core, client, seed bots --quick (exit 0), pytest.

Écarts : cartes de choix écrites pour les 5 nouvelles maisons (data/ui/front_end.json) ; 131 cartes retirées ; onglet « Défis singuliers » (fac_crusaders) retiré, donc vide (non jouable).
