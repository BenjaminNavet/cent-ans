# XXXX — HIST4 : aptitude historique du conseil, monnaie par faction, contrastes par défaut

Statut : accepté (29/10/2026, lot HIST4 de la boucle juge 2). Numéro à fixer à la fusion.

## Contexte
Le juge histoire/UX de l'itération 2 (`out/juge2/iter-02-histux.md`) relevait un conseil d'Angleterre incohérent en 1337 (Isabelle
chancelière, Philippa trésorière, Montagu aumônier, Bohun connétable avant 1338), une monnaie « livres » commune à toutes les factions,
des petites factions qui perdent de l'argent dès le premier tour et des contrastes de palette sous le seuil.

## Décision
1. **Conseil.** L'attribution automatique et la nomination passent par `council::office_allows` (règle du cœur, pilotée par
   `data/council/offices.json`) : `excluded_roles` (épouse, prétendant) et `excluded_traits` (disgrâce) valent pour toutes les
   charges ; `holder_roles` limite une charge (l'aumônier est un clerc) ; `title_keyword` écarte qui porte un titre homonyme dont la
   date de début est postérieure à l'année courante ; `preferred_roles` passe avant la compétence (chancelier et trésorier :
   clercs ou civils, connétable : homme de guerre). Les charges restreintes choisissent d'abord. Un courtisan généré compte comme
   laïc de la noblesse. Trois prélats anglais sont ajoutés (Jean et Robert de Stratford, Richard de Bury) pour que le conseil
   d'Angleterre ait ses clercs. Faute de clerc, la charge cléricale reste vacante (pas d'effet), comme toute charge sans titulaire.
2. **Monnaie.** `data/economy/currencies.json` (catalogue) et `currency` par faction (schéma `faction.schema.json`). La valeur reste
   l'unité de compte commune (la livre tournois de l'ADR 0793) ; seul le nom change. Le client fixe la « monnaie d'affichage »
   (`data_model::currency::set_display`) à la création ou au chargement d'une campagne ; les textes du cœur lisent `MONEY`,
   les textes de données `{money}` et `{money_symbol}`. Les écrans qui montrent une autre faction (choix de faction) lisent la
   monnaie de cette faction. Le cœur ne fixe jamais la monnaie d'affichage : ses tests restent en livres.
3. **Petites factions en déficit.** Cause mesurée : la ponction de l'opulence (ADR 0788) sur un trésor de départ de quatre saisons de
   revenu ; le solde structurel est positif. Le plafond de départ `treasury_max_income_seasons` passe de 4 à 1 (`data/settlements/rules.json`),
   `data/rules/starting_fit.json` est régénéré : le trésor de départ ne dépasse plus ce qui rend la ponction supportable. Aucun
   revenu n'est ajouté.
4. **Contrastes.** `GOLD`, `FAIR`, `GOOD`, `POOR` atteignent 4,5:1 sur le parchemin, `TAB_UNSELECTED_INK` 4,5:1 sur la barre
   d'onglets ; `GOOD` et `POOR` diffèrent en luminance (environ 1,7:1 au lieu de 1,3:1). Test `default_palette_meets_wcag_aa`.
5. **Écartés.** Lothian reste écossais (ADR 0730) ; les voix des conseillers gardent l'apostrophe droite (clé de cache audio,
   `tools/typo_apostrophes.py`) ; l'ordre de l'Étoile est bien daté de l'automne 1351 par son propre texte.

## Conséquences
- Les factions sans clerc n'ont pas d'aumônier (effet de trouble absent) ; l'équilibre est suivi par les seed bots rapides.
- Un trésor de départ plus bas pour les factions dont le revenu est faible.
