# ADR 0174 — Formations historiques en données et reformation progressive

État : accepté (lot RJ-a, 2026-10-03). Révise ADR 0095 (« pas de nouveau type de formation »).

## Contexte

Retours du joueur (03/10, `docs/wip/rj-retours-joueur.md`) :
- le bouton « Formation » cyclait quatre formes codées en dur (`enum Formation` de `sim-battle`, cycle dans `battle_input.gd`), sans noms historiques ni explication ;
- la mise en formation était instantanée : `BattleSim::apply` affectait `formation` et les places étaient recalculées au pas suivant, les figurines se téléportaient pendant que le temps tournait ;
- ni « Formation » ni « Tir à volonté » ne montraient s'ils étaient actifs.

ADR 0095 avait exclu tout nouveau type de formation (largeur au glisser seulement). Le joueur demande désormais plusieurs formations historiques avec infobulle et une reformation visible.

## Décision

1. **Formations en données.** `data/rules/unit_formations.json` (schéma `unit_formations_rules.schema.json`) liste les formations : clé, nom historique, libellé court, contexte pour l'infobulle, catégories et montés/à pied permis, géométrie (forme parmi `line`, `column`, `square`, `wedge`, `herse` ; rangs par classe, files, facteurs d'espacement), modificateurs (allure, charge, dégâts subis en mêlée et au tir, tir infligé, moral perdu, poussée exercée et encaissée, coups des cavaliers, rangs combattants), drapeaux (`all_round`, `braced`, `road_march`, `width_adjustable`), rôle pour l'IA (`default`, `march`, `anti_cavalry`, `charge`) et durée de reformation. L'ordre du fichier est celui du menu et du cycle de la touche T.
2. **`Formation` devient un indice dans cette table**, écrit en JSON par sa clé (commandes, scénarios, cartes historiques, replays : les clés `line`, `column`, `square`, `wedge` d'avant restent valides). Le code ne nomme plus aucune formation : chaque effet lit la table (`Unit::formation_speed`, `melee_taken_factor`, `braced`, `all_round`…). Seules les formes géométriques restent du code. Le facteur de poussée du carré passe de `battle_push.json` (`square_factor`) à la formation (`push_resistance`).
3. **Huit formations** (sources et réserves : `docs/research/rj-formations.md`) : Rangés en bataille (ligne), Coin, En ordre de marche (colonne), Schiltron — les quatre d'avant, renommées, mêmes effets — et En haie, Bataille serrée, Herse, En conroi. L'infobulle dit quand un usage est débattu (herse, coin). Écartées : haie de pieux (déjà l'ordre « pieux »), ordre lâche (déjà le mode escarmouche).
4. **Reformation progressive dans le cœur.** Un ordre de formation met le régiment en `Unit::reform` (`from`, largeur quittée, `elapsed`, `duration`). Durée = `reform_s` × (effectif / 100)^0,5 bornée à [0,6 ; 1,8] × (1 − 0,06 × expérience), au moins ×0,6. Chaque homme marche de sa place dans l'ancienne formation à sa place dans la nouvelle, au pas de marche (relevé pour arriver à temps) ; les règles (emprise, contact) utilisent déjà la nouvelle formation. Pendant la reformation : allure ×0,5 (sauf à la charge : les cavaliers serrent les rangs au galop, sans quoi le coin pris en chargeant cassait toute charge d'IA), dégâts subis en mêlée ×1,25, au tir ×1,1, moral perdu ×1,2 (bloc `reform`). Redemander la formation quittée fait demi-tour sur place ; pendant le déploiement et la mise en place des scénarios, le changement reste instantané. `state_digest` hache la reformation quand il y en a une.
5. **IA.** `formation_ai` choisit par rôle la première formation permise. Comme chaque changement coûte désormais une reformation, l'IA laisse finir une reformation (sauf schiltron devant la cavalerie) et ne reprend l'ordre de marche ou le coin que 20 s après le dernier changement ; elle ne forme le coin qu'à 120-250 m de sa cible et loin (60 m) de toute haie ou fossé, et le garde tant qu'elle charge. Le détour des cavaliers autour d'une haie passe 25 m (au lieu de 10) du côté de la cible : à 10 m ils longeaient la haie et se faisaient renvoyer au point de détour à chaque pas d'IA (le test B6 ne passait que grâce au coin instantané, plus profond).
6. **Interface.** Le bouton « Formation » ouvre un menu (`battle_formation_menu.gd`) : une ligne par formation, grisée si aucun régiment choisi ne peut la prendre, dorée si c'est celle de toute la sélection, infobulle en sections (`ib:formation:<clé>` : nom, troupes permises, contexte historique, effets chiffrés tirés des données, reformation). La touche T fait défiler les formations permises. Les boutons « Formation » et « Tir à volonté » ont un état on/off doré comme les modes ; « Formation » montre le libellé court commun et une barre de reformation. Pont : `unit_formations()`, `formation_reform_rules()`, et dans `get_units` `formation_name`, `formation_short`, `formations`, `reforming`, `reform_progress`, `reform_from`.

## Conséquences

- Une nouvelle formation s'ajoute en données, sauf nouvelle forme géométrique.
- Les issues des batailles d'IA changent (reformations plus lentes) : empreintes chiffrées des tests recalculées et documentées dans les tests.
- Un glisser-droit garde une formation en ligne réglable (`width_adjustable`) ; une autre forme revient à la ligne ordinaire, instantanément (comme avant).
- L'interpolation entre deux pas de simulation (lot RJ-b) s'applique aux poses de reformation sans changement.
- Points ouverts : l'IA n'emploie pas encore les nouvelles formations (bataille serrée en défense, herse pour les archers) ; libellés à relire par l'historien (`docs/research/rj-formations.md`).
