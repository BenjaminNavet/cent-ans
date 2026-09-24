# WIP — S1 personnages manquants (audit § 7)

Source : `docs/histoire/audit-2026-09-23.md` § 7 « Personnages de 1337 manquants importants ».
Déjà présents (non retouchés) : Gautier de Mauny, William Montagu, Agnès Randolph.

## À créer (12 personnages + fiches Codex)

| Id | Faction | Province | État |
|---|---|---|---|
| chr_jeanne_de_valois_hainaut | fac_hainaut | prov_hainaut | fait |
| chr_hugues_quieret | fac_france | prov_normandie | fait |
| chr_nicolas_behuchet | fac_france | prov_normandie | fait |
| chr_godefroy_d_harcourt | fac_france | prov_normandie_ouest | fait |
| chr_olivier_iv_de_clisson | fac_brittany | prov_bretagne | fait |
| chr_louis_i_de_bourbon | fac_france | prov_bourbonnais | à faire |
| chr_gaston_ii_de_foix_bearn | fac_france | prov_bearn | à faire |
| chr_jean_i_d_armagnac | fac_france | prov_toulousain | à faire |
| chr_humbert_ii_de_viennois | fac_empire | prov_dauphine | à faire |
| chr_jacques_iii_de_majorque | fac_aragon | prov_mallorca | à faire |
| chr_pierre_roger | fac_france | prov_normandie | à faire |
| chr_petrarque | fac_papacy | prov_comtat_venaissin | à faire |
| chr_jean_le_bel | fac_hainaut | prov_hainaut | à faire |

Non créés (aucune faction adaptée dans `data/factions/` : pas de Mérinides ni de Danemark) —
fiche Codex seule : Abu al-Hasan Ali, Valdemar IV Atterdag.

## Libertés notées
- Jean Ier d'Armagnac : pas de province `prov_armagnac` dédiée ; placé à `prov_toulousain`
  (Armagnac limitrophe, domaine royal), à corriger si une province Armagnac est créée plus tard.
- Jeanne de Valois : famille reliée à `chr_guillaume_i_de_hainaut` et `chr_philippe_vi` sans
  modifier ces fichiers existants (lien à sens unique).

## Prochaine étape
Lot 2 (Bourbon, Foix-Béarn, Armagnac, Humbert II, Jacques III, Pierre Roger, Pétrarque, Jean le Bel)
puis les 14 fiches Codex, puis mise à jour de `docs/wip/historien-suite.md` (ligne S1) et
`docs/histoire/README.md` (pistes pour la suite), puis tests.
