# Critique gameplay / IA (nuit du 22/10/2026)

Rôle : programmeur gameplay / IA. Lecture ciblée de `core/crates/{ai,sim-campaign,sim-battle}` (grep, extraits). Aucun test ni clippy lancé, aucun code modifié : les constats sont à confirmer par un test avant correctif.

## Constats

### 1. Ordre de jeu des factions IA fixe (avantage du premier joueur)
- **Problème** : chaque saison, les IA jouent dans l'ordre alphabétique des identifiants. La première réagit à un état non encore modifié, les suivantes subissent ses coups (sièges, batailles, traités).
- **Preuve** : `sim-campaign/src/turn.rs:49-64` (« Each AI faction plays in id order », `BTreeMap` de `state.rs:1062`).
- **Gravité** : majeur (biais d'équilibre systématique, invisible dans les moyennes des seed bots).
- **Correctif** : ordre déterministe mais tournant, p. ex. permutation dérivée de `hash(seed, tour)` ; ajouter une métrique « victoires par rang de jeu » aux seed bots.
- **Effort** : S

### 2. Le planificateur travaille sur un état périmé, les ordres refusés sont jetés sans trace
- **Problème** : `planner(self, …)` renvoie tous les ordres de la saison d'un coup, puis ils sont appliqués un à un. Un ordre rendu invalide par le précédent (armée déjà partie, or dépensé, cible morte) est ignoré (`let _ =`). L'IA ne sait pas qu'elle a échoué, personne ne mesure le taux de rejet.
- **Preuve** : `sim-campaign/src/turn.rs:94-97` ; `sim-campaign/src/ai_replay.rs:205,212` (`let _ = self.apply_order(...)`) ; `ai/src/campaign.rs:552` (`planned_spending` estime la dépense sans refléter les refus).
- **Gravité** : majeur (comportements d'IA gaspillés, difficiles à diagnostiquer).
- **Correctif** : compter les rejets par type d'ordre (compteur dans le rapport de tour, oracle seed bots avec seuil) ; à terme, planifier par lots de 1 à 3 ordres avec re-lecture de l'état.
- **Effort** : S pour la mesure, L pour le replanning.

### 3. Fonctions transcendantes en f64 dans la simulation : déterminisme multi-plateformes non garanti
- **Problème** : la bataille utilise ~86 appels `sin/cos/powf/exp/atan2` et ~2300 occurrences de f32/f64 ; les résultats de la libm peuvent différer entre macOS, Linux et Windows (port Windows en cours, ADR 0087/0153). Les replays (`ep13_replay`), les sauvegardes et `rng.rs` (« sur toutes les plateformes ») promettent plus que ce que le code garantit. Aucune dépendance `libm` détectée.
- **Preuve** : `grep -rn "sin(\|cos(\|powf\|exp(\|atan2" sim-battle/src` (86 lignes) ; promesse dans `sim-battle/src/rng.rs:3-4` ; aucune `libm` dans les `Cargo.toml` du cœur.
- **Gravité** : majeur (replays et seeds divergents entre PC du joueur et CI ; bug irreproductible).
- **Correctif** : test d'empreinte (hash de l'état après N ticks d'une bataille fixe) exécuté sur CI macOS + Linux + Windows ; si divergence, passer par la crate `libm` pour les fonctions concernées.
- **Effort** : M

### 4. Bataille non bornée en temps réel côté IA campagne : aucune limite de calcul
- **Problème** : `play_ai_turn` mesure le temps (`Instant::now`) mais ne s'en sert que pour des statistiques. Aucun plafond ni mode dégradé si le planificateur explose (grosse campagne tardive, 8+ factions). Le pool rayon réserve 64 Mo de pile par worker pour compenser une récursion ou des allocations profondes.
- **Preuve** : `sim-campaign/src/turn.rs:65-67` ; `ai/src/parallel.rs:55` (`WORKER_STACK_BYTES = 64 MiB`).
- **Gravité** : mineur (pas de bug constaté, risque de fin de partie lente).
- **Correctif** : test de régression sur la durée du tour à T=200 (budget en nombre d'opérations, pas en secondes, pour rester déterministe) ; documenter pourquoi 64 Mo.
- **Effort** : S

### 5. Soustractions non signées sans garde sur des invariants implicites
- **Problème** : `candidate.deadline_turn - candidate.issued_turn` (u32) fait paniquer en debug ou boucler en release si une donnée de mission est incohérente (échéance < émission), y compris dans le tri de l'IA. Idem `points.len() - 1` sur un chemin vide.
- **Preuve** : `sim-campaign/src/missions.rs:298` et `:351` ; `sim-campaign/src/path_plan.rs:120,127,140` ; `sim-campaign/src/siege.rs:916`.
- **Gravité** : mineur (données actuelles valides, mais le schéma ne l'interdit pas).
- **Correctif** : `saturating_sub` + règle de schéma/validateur de données (`deadline > issued`) ; `debug_assert!` sur les plans de chemin non vides.
- **Effort** : S

### 6. Caches globaux et thread-local dans le cœur de bataille
- **Problème** : `LAYOUT_CACHE` est un `thread_local` de `HashMap` borné à 512 entrées, vidé selon une politique d'éviction dépendant de l'ordre d'accès ; les caches de `data-model` (`cover.rs`, `navgrid.rs`) sont des `static Mutex<HashMap<PathBuf,…>>` jamais invalidés. Si une valeur mise en cache influence la simulation (et pas seulement le rendu), l'état dépend de l'historique des appels, donc des threads (rayon) et des tests précédents.
- **Preuve** : `sim-battle/src/unit.rs:104-110,891-896` ; `data-model/src/cover.rs:167-169` ; `data-model/src/navgrid.rs:350-351`.
- **Gravité** : mineur (probablement sans effet sur le résultat, non prouvé).
- **Correctif** : test « cache froid vs cache chaud » sur une bataille complète (hash d'état identique) ; sinon, passer les caches en entrée explicite de la simulation.
- **Effort** : S

### 7. Table d'importance des alertes en `HashMap<String,u32>`
- **Problème** : tri d'affichage dépendant d'une `HashMap` ; sans effet tant qu'on interroge par clé, mais toute itération future (liste, sérialisation, export) sera non déterministe. Les clés sont des chaînes libres : une faute dans le JSON retombe silencieusement à 0 (« a kind missing here sorts last »).
- **Preuve** : `sim-battle/src/alerts.rs:82-93`.
- **Gravité** : mineur.
- **Correctif** : `BTreeMap` et test de complétude : chaque `AlertKind` doit figurer dans `battle_alerts.json`.
- **Effort** : S

### 8. Constantes d'équilibre de l'IA codées en dur (hors `data/`)
- **Problème** : la règle du projet est « jamais de données codées en dur », or l'IA contient `SUBSIDY_*` (2 saisons, diviseur 3, ratio 3, minimum 100), `VALUE_WEIGHT = 1e-3`, `MAX_SENT_PER_TURN = 4`, `PLAN_PERIOD = 6`, `AMBUSH_CANDIDATES = 4`. Les seed bots ne peuvent pas les balayer, le rééquilibrage de la refonte (chantier 4, IA de campagne) obligera à recompiler.
- **Preuve** : `ai/src/support.rs:10-17` ; `ai/src/doctrine.rs:17` ; `ai/src/campaign/char_moves.rs:15` ; `ai/src/diplomacy_eval.rs:21` ; `ai/src/stances.rs:37`.
- **Gravité** : majeur pour la suite (bloque le travail d'équilibrage), mineur aujourd'hui.
- **Correctif** : fichier `data/rules/ai_tuning.json` + schéma, chargé via `bundled_rules!` ; valeurs actuelles reprises à l'identique (test de non-régression sur les golden des seed bots).
- **Effort** : M

### 9. Pas de difficulté ni de profil d'IA mesurables côté campagne
- **Problème** : seul `JUDGEMENT` (bruit d'estimation d'un siège) et quelques jets salés traitent de la difficulté. Aucun levier global (agressivité, prudence budgétaire) : les 6 « bots » des seed bots sont des scripts extérieurs, pas des profils de l'IA du jeu. Résultat : les révoltes par partie varient de 0 (diplomate) à 4,3 (agresseur) sans qu'on sache laquelle correspond à l'IA livrée.
- **Preuve** : `ai/src/salts.rs:17-18` ; `docs/wip/refonte-systemes.md` § 1.A (mesures par bot).
- **Gravité** : majeur (la cible 3-15 / 4-10 des révoltes ne peut pas être évaluée sur le comportement réel de l'IA).
- **Correctif** : un seed bot « IA du jeu » (`plan_turn` pour tous) comme référence unique des bandes cibles ; unifier les bandes 3-15 / 4-10 sur cette référence.
- **Effort** : M

### 10. Compteur de saisons consécutives pour les révoltes : cas limites et effet de seuil
- **Problème** : révolte après 5 saisons consécutives de moyenne > 75, compteur remis à zéro à la moindre interruption et après chaque révolte. Une oscillation 74/76 empêche toute révolte ; une ville peut basculer aux rebelles (> 90) sans armée ni bataille, donc sans réponse jouable du joueur. Le bruit de mesure (±0,8) est du même ordre que l'écart à la cible.
- **Preuve** : `data/rules/population.json` ; `sim-campaign/src/population.rs` ; `docs/wip/refonte-systemes.md` § 1 (constat, lot B).
- **Gravité** : majeur (déjà identifié, correctif planifié mais non fait).
- **Correctif** : lot B de la refonte (jauge de tension cumulée avec décroissance) avec test de propriété : une oscillation autour du seuil finit par déclencher une révolte, une oscillation bien en dessous jamais.
- **Effort** : M

### 11. Équilibre de la mêlée tenu par des impacts répétés, mécaniques neutres désactivées
- **Problème** : élan, immunité au choc et déclin de charge sont codés mais neutres car l'équilibre repose sur ~7 000 impacts par bataille ; la fatigue ne monte pas en mêlée (repos ≈ 2,7× plus rapide). Plusieurs tests d'équilibre sont des fenêtres étroites (`Crécy [14,20)`, `Poitiers [11,19)`) : tout changement de règle les brise, d'où la tentation de les élargir.
- **Preuve** : `docs/wip/refonte-systemes.md` § 2 ; `sim-battle/tests/combat/tw_balance.rs:109` (`#[ignore]`) ; `sim-battle/tests/ai_tactics/contact.rs:22` et `rx_batsim.rs:126` (`#[ignore]`).
- **Gravité** : majeur (dette d'équilibre, trois tests de comportement d'IA ignorés).
- **Correctif** : lot C1 (mesure du budget d'impacts) en premier ; réactiver ou supprimer explicitement les trois tests `#[ignore]`, avec une raison écrite (convention `#[ignore = "..."]` déjà utilisée ailleurs).
- **Effort** : M

### 12. Fichier `state.rs` et modules campagne très gros (dette de lisibilité)
- **Problème** : 20+ modules de 1 000 à 1 700 lignes (`dynasty.rs` 1665, `siege.rs` 1606, `state.rs` 1544, `negotiation.rs` 1480, `battle_auto.rs` 1461, `feudal.rs` 1430) ; 22 `#[allow]` dont 19 `too_many_arguments` : signe de contexte non regroupé (`data`, `state`, `events`, `faction` passés séparément). Cela complique l'ajout de règles transversales (ordre public, tour d'IA).
- **Preuve** : `wc -l core/crates/sim-campaign/src/*.rs` ; `grep -rn too_many_arguments core/crates` (19).
- **Gravité** : mineur.
- **Correctif** : structure `TurnCtx { data, events, faction }` introduite module par module à l'occasion des lots, sans refonte dédiée.
- **Effort** : L (en continu)

### 13. `unreachable!` dans les résolutions d'effets et d'ordres
- **Problème** : plusieurs branches `unreachable!("handled by …")` reposent sur un tri préalable par le code appelant ; l'ajout d'une variante d'`EventEffect` ou d'`Order` provoque un panic en jeu plutôt qu'une erreur de compilation. Les données d'événements étant modifiables (JSON), un effet mal classé suffit.
- **Preuve** : `sim-campaign/src/effects.rs:84,497` ; `sim-campaign/src/orders/mod.rs:164,171` ; `sim-campaign/src/ransom.rs:432` ; `sim-battle/src/sim/commands.rs:94`.
- **Gravité** : mineur.
- **Correctif** : scinder les enums (ex. `EventEffect::{Province, Character, Other}`) ou retourner `Err(UnsupportedEffect)` ; test qui parcourt toutes les variantes.
- **Effort** : S à M

## Top 3

1. **Constat 2** : mesurer le rejet silencieux des ordres IA (S) ; c'est le prérequis pour juger l'IA de campagne avant la refonte.
2. **Constat 3** : test d'empreinte multi-plateformes de la bataille (M) ; le déterminisme est la promesse centrale (replays, seeds, saves) et le port Windows la met en jeu.
3. **Constat 1 puis 8** : ordre de jeu tournant des IA (S), puis sortie des constantes d'IA vers `data/` (M) ; les deux conditionnent un rééquilibrage fiable.
