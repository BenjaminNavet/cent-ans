# WIP — H8 rédaction massive du Codex

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §1 et §5.3. Branche : worktree agent H8.

## Compteur
- Fiches écrites (H8) : 41 / ~120 (61 au total dans `data/codex/`)

## Familles
- [x] `_todo.md` (hors cuisine et médecine) : bretigny, calais, jacquerie, etienne_marcel,
      charles_le_mauvais, grandes_compagnies, bordeaux, jacob_van_artevelde, gabelle,
      grand_schisme, azincourt, jeanne_darc, traite_troyes, orleans, universite_paris,
      ost_feodal, compagnies_ordonnance, franc_archer
- [~] Personnages : faits charles_vi, charles_vii, henri_v, jean_sans_peur, philippe_le_bon,
      philippe_le_hardi, isabeau_de_baviere, froissart, jean_le_bel, nicole_oresme,
      john_wyclif, jan_hus, philippa_de_hainaut, louis_de_nevers, louis_de_male,
      pierre_le_cruel, robert_knolles, jean_de_berry, jean_de_gand, isabelle_de_france,
      charles_iv (+ religion : lollards, hussites).
      Restent : jean_de_luxembourg, henri_de_grosmont, louis_de_baviere (déjà cités), puis
      christine_de_pizan, guillaume_de_machaut, gautier_de_mauny, john_chandos,
      olivier_de_clisson, jeanne_de_penthievre, jeanne_de_flandre, wat_tyler, david_ii,
      charles_de_blois, jean_de_montfort, robert_d_artois, benoit_xii, louis_d_anjou,
      jeanne_de_bourgogne, louis_d_orleans (alias « duc d'Orléans » pour éviter que
      « Orléans » seul ne vise le siège).
- [ ] Dynasties, lieux, factions : valois, plantagenets, capetiens_directs,
      succession_bretagne (Montfort et Blois) ; paris, londres, avignon, gand, bruges, rouen,
      reims, dijon ; factions bourgogne, navarre, castille, comte_de_flandre, saint_empire,
      royaume_de_france, angleterre, ecosse, bretagne (entity `fac_*` ; la papauté est déjà
      couverte par `cdx_papaute_avignon`, entity `fac_papacy`).
- [ ] Guerre : castillon, roosebeke, nicopolis, bataille_des_harengs (id de `_diet_links.md`,
      catégorie bataille, dans le périmètre), arbalete (Génois), hommes_d_armes, harnois,
      bombarde, trebuchet, rancon, chevalerie (adoubement), tournoi, jarretiere,
      ordre_etoile, toison_or, herauts.
- [ ] Événements : armagnacs_bourguignons, cabochiens, maillotins, revolte_paysans_1381
      (+ wat_tyler), ciompi, bal_des_ardents, paix_arras, concile_constance.
- [ ] Société, économie, institutions : trois_ordres, seigneurie (servage), etats_generaux,
      taille, aides, ecu, franc_a_cheval, mutations_monetaires, foires_champagne,
      lombards (changeurs), corporations, parlement_paris, hanse (id de `_diet_links.md`,
      économie, dans le périmètre). « draperie flamande » est déjà un alias de
      `cdx_flandre_laine`, « mutation monétaire » de `cdx_livre_tournois`.
- [ ] Religion et savoirs : ordres_mendiants, pelerinage, reliques, oxford, enluminure,
      librairie_charles_v, tres_riches_heures, ars_nova, horloges, papier ; careme (id de
      `_diet_links.md`, religion) possible.
- [ ] Héraldique (heraldique, emaux_metaux, meubles, brisures, lire_blason), calendrier
      (calendrier_julien + style de Pâques, travaux_des_mois, heures_canoniales, fetes),
      vie quotidienne (vetements + lois somptuaires, etuves, echecs, des, paume).
- [ ] Liens `[[…]]` dans `data/characters`, `data/events` (champ `text`), `data/technologies`
- [ ] Onglets de `codex_window.gd` (libellés courts pour 9 onglets à 980 px)
- [ ] Tests finaux : pytest complet, `cargo test`, smoke Godot

## Constats
- Champs formatés par `CodexText.format` : `characters.description` (fiche personnage),
  `events.text` (fenêtre de chronique), `technologies.description` (infobulle riche).
- Non formatés : `factions.description` (non affichée nulle part), `events.title` et
  `options.text` (texte brut dans la chronique et le journal) → pas de liens.
- Le titre d'une fiche sert aussi d'alias d'auto-liaison : éviter les titres génériques
  (« La taille », « Les échecs ») → « La taille royale », « Le jeu d'échecs ».
- Alias déjà pris : Flandre, draperie flamande, Gascogne, Poitiers, mutation monétaire,
  Palais des Papes, le dauphin Charles (Charles V).

## Notes de reprise
- Les ids cités mais pas encore écrits sont listés en fin de `_todo.md` (section H8). Script de
  régénération (liens → ids manquants, ids écrits retirés) : lire tous les `[[…]]` et
  `see_also` de `data/**/*.json`, retirer les ids écrits et ceux des `_*.md`, réécrire la
  section « En attente de rédaction (H8) ».
- `tools/cent_ans_tools/codex.py` : les ids de tout `data/codex/_*.md` sont tolérés (règle de
  chevauchement limitée à `_todo.md`), comme annoncé par l'orchestrateur ; main n'avait pas encore
  ce changement au moment de la fusion.
- Ne pas rédiger les ids de `_diet_links.md` / `_herb_links.md` hors ceux signalés ci-dessus.
- Commandes worktree : pas de `cd … &&`, pas de variables en tête de commande ; commandes git
  simples, une par appel.

## H8b (reprise, session historien 2)
- Script de régénération de la section H8 de `_todo.md` : copie dans le scratchpad de la session
  (`regen_todo.py <data_dir>`) ; logique décrite ci-dessus, en plus : ids des autres `_*.md` exclus.
- Décisions : `cdx_artevelde` (liste `_event_links.md`) = fiche existante `cdx_jacob_van_artevelde`,
  liens des événements repointés ; `cdx_succession_bretagne` fusionné dans
  `cdx_guerre_de_succession_de_bretagne` (Knolles repointé) ; Jarretière et Étoile écrites sous les
  ids `cdx_ordre_de_la_jarretiere` et `cdx_ordre_de_l_etoile`.
- Lot 1 fait : les 10 fiches de `_event_links.md` (black_agnes, combat_des_trente, gallicanisme,
  gautier_de_mauny, guerre_de_succession_de_bretagne, henri_de_grosmont, hugues_quieret,
  ordre_de_l_etoile, ordre_de_la_jarretiere, vicariat_imperial).
- Lot 2 fait (guerre) : arbalete, bombarde, castillon, chevalerie, harnois, hommes_d_armes,
  nicopolis, roosebeke, rancon, toison_or.
- Lot 3 fait (dynasties, États) : capetiens_directs, valois (entity fac_france), plantagenets
  (entity fac_england), bourgogne, navarre, castille, saint_empire, comte_de_flandre, ecosse.
  Pas de fiches séparées royaume_de_france / angleterre : les dynasties portent les factions.
- Lot 4 fait : louis_de_baviere, jean_de_luxembourg (alias « Jean l'Aveugle », pas « Jean de
  Luxembourg », homonyme du capitaine bourguignon), david_ii, paris, gand.
- Fiches H8b écrites : 34.

## Prochaine étape
Écrire les ids de la section H8 de `_todo.md` (déjà cités), puis le reste des familles.
