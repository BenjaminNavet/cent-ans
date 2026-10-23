# Lot EVENTS : histoire conditionnée (ADR 0779)

Branche `worktree-agent-a8a233370fcc4ce72`. Sources : `out/juge/iter-03-histux.md` (partie A), `iter-02-histux.md`, `iter-03-design.md`.

## État : terminé (vérifications finales faites en fin de lot)

- Moteur : conditions `not`, `battle_balance`, `domain_below`, `strength_ratio`, `ruler_captive`, `flag` ; effet `set_flag`. Décompte permanent des batailles terrestres (`ChronicleState::battle_tally`, alimenté par `battle_history::record`) ; drapeaux dans `ChronicleState::flags`.
- Brétigny : 3 scénarios x 2 camps (6 événements) ; Poitiers : 3 lectures ; Crécy : 5 lectures dont le miroir anglais ; Jeanne d'Arc conditionnée au domaine royal (< 55 % des provinces de 1337) ; `evt_sursaut_national` pour les autres factions ; 3 chaînes à choix (Castille, Bretagne, Étienne Marcel) avec drapeaux.
- Données générées par un script jetable (hors dépôt) ; les JSON de `data/events/` sont la source.

## Mesure : part de l'histoire tirée par une partie simulée

Sonde `core/crates/ai/examples/events_probe.rs` (IA stratégique partout, joueur compris, 1337-1453, graine 1) :

| Joueur | Événements tirés (sur 24 de la liste) | Batailles terrestres Angleterre > France / France > Angleterre |
|---|---|---|
| France | 8 | 151 / 51 |
| Angleterre | 11 | 193 / 36 |

Union des deux parties : 16 des 24 (67 %), moyenne 9,5 par partie (40 %). Une partie n'en tire que les lectures qui correspondent à son déroulement : dans ces parties l'Angleterre mène aux batailles, donc Crécy est un désastre français et un triomphe anglais, Poitiers ne prend le roi que dans la partie anglaise, la paix blanche et la victoire française de Crécy n'ont pas lieu. Non tirés dans ces deux parties : `evt_bretigny_paix_blanche(_anglaise)`, `evt_bretigny_angleterre_a_bout`, `evt_bretigny_roi_sans_argent`, `evt_poitiers_victoire_francaise`, `evt_maupertuis_sans_bataille`, `evt_crecy_revers_anglais`, `evt_crecy_victoire_francaise`, `evt_sursaut_national`, `evt_castille_menacee`, `evt_confiscation_de_la_bretagne` (choix non pris par l'IA) et selon la partie `evt_jeanne_d_arc`, `evt_fin_d_etienne_marcel`, `evt_ordonnances_de_charles_v`.

Test `tests/campaign_life/ev_history.rs` : chaque événement a un état fabriqué où il se déclenche (`the_remaining_events_fire_in_their_staged_states`, `crecy_has_...`, `poitiers_...`, `bretigny_...`, `joan_of_arc_...`, chaînes). Le test `a_simulated_campaign_fires_the_new_history` (graine 1341, >= 20 événements F7) n'est pas modifié et passe. L'IA minimale du cœur ne livre aucune bataille en 117 ans (décompte vide) : sa partie ne tire que les lectures neutres (`a_simulated_game_draws_the_neutral_readings`).

## Écarts et points ouverts

- `battle_balance` compte toutes les batailles terrestres (rangée, assaut, sortie), pas seulement les batailles rangées : l'IA minimale ne livre aucune bataille rangée, l'IA stratégique des assauts surtout.
- Poitiers ne dépend plus de la personne de Jean II (dans les parties simulées Philippe VI règne encore en 1356) : le texte dit « le souverain ».
- Les effets d'un événement qui signe la paix (Brétigny, Crécy revers) retirent la guerre : les lectures suivantes du même tour, qui exigent `at_war`, ne se déclenchent plus ; c'est voulu mais piège pour les futurs événements.
- Le rapport de forces (`strength_ratio`) compare les hommes en armes à l'instant T, sans les alliés.
- La « pleine souveraineté » de Guyenne reste un drapeau (`guyenne_en_pleine_souverainete`) et une cession de provinces ; aucune règle de suzeraineté n'en dépend encore.
