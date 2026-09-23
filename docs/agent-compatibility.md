# Matrice de compatibilité des agents

Dernière vérification : 2026-07-29.

Ce document décide quels bindings SkillReg peut créer. Une ligne non validée reste désactivée
dans l'adaptateur correspondant : la présence d'un agent ne suffit pas à revendiquer ses dossiers.
La détection est en lecture seule et la création des liens appartient au gestionnaire de bindings.

## Contrat commun

- Le contenu canonique reste sous
  `~/.skillreg/skills/<consumer-org>/<source-org>/<skill-name>/content`.
- Un seul lien portant le nom du skill est créé dans le dossier utilisateur préféré de chaque
  agent validé.
- Les dossiers candidats historiques sont scannés pour signaler une installation existante, mais
  ne sont ni déplacés, ni remplacés, ni revendiqués automatiquement.
- Aucun adaptateur ne déclare de signal d'usage fiable. `usage_capability` vaut
  `unavailable` pour Claude, Codex et Cursor.
- Un lien cassé n'est jamais interprété comme une autorisation de supprimer sa cible. SkillReg
  vérifie le manifest, le type du lien et sa cible exacte avant toute réparation ou suppression.
- Un adaptateur non supporté ou en erreur ne bloque pas la détection ni la réconciliation des
  autres agents.

## Cycle de vie géré

- Une installation desktop ne choisit aucun agent : les adaptateurs détectés et activés sur la
  plateforme reçoivent chacun un binding vers la même copie canonique.
- À chaque ouverture du dashboard, SkillReg réconcilie silencieusement les bindings. Un agent
  compatible installé après une skill reçoit ainsi son lien sans réinstallation ni choix
  supplémentaire.
- Installer, mettre à jour, réparer, désinstaller et changer d'organisation utilisent le même
  verrou de mutation et le manifest v2.
- La réparation recrée uniquement un lien manquant ou cassé dont la propriété et la cible sont
  prouvées. Un conflit physique ou un lien vers une autre cible reste intact.
- La désinstallation pré-vérifie tous les bindings. Si un seul lien n'est plus possédé sans
  ambiguïté, aucun contenu canonique n'est retiré.
- Changer d'organisation planifie les retraits et créations avant mutation. L'ancienne
  organisation reste en cache, sans bindings actifs, et peut être réactivée sans téléchargement si
  son hash est valide.
- Le CLI utilise les chemins projet historiques pour les workflows avancés. Au scope utilisateur,
  il lit le manifest v2 en lecture seule et ne concurrence jamais le gestionnaire Rust.

Les champs d'observabilité présents dans le manifest sont des capacités, pas des mesures d'usage.
Le lot A n'enregistre ni compteur, ni date de dernière utilisation, ni suggestion de nettoyage.

## Chemins et détection

| Agent | Signal de détection en lecture seule | Dossier utilisateur préféré | Dossiers encore reconnus | Politique de compatibilité |
| --- | --- | --- | --- | --- |
| Claude Code | `claude --version` | `~/.claude/skills` | Aucun chemin utilisateur legacy distinct | Les liens gérés nécessitent Claude Code 2.1.203 ou plus récent. Une version antérieure est `unsupported`. |
| Codex | `codex --version` | `~/.agents/skills` | `~/.codex/skills` | Le chemin neutre documenté remplace progressivement le chemin historique SkillReg. Une installation legacy est seulement signalée. |
| Cursor Agent | `cursor agent --version` | `~/.cursor/skills` | `~/.agents/skills` est un emplacement officiel alternatif | Le chemin natif Cursor évite qu'un binding Cursor et un binding Codex revendiquent le même lien dans `~/.agents/skills`. |

Les chemins préférés sont construits exclusivement depuis le home explicite passé à
l'adaptateur. Ils ne dépendent jamais du répertoire courant. Si le binaire est absent, l'état est
`not_detected`, même lorsqu'un ancien dossier de skill existe ; ce dernier est renvoyé séparément
comme installation non gérée.

## Sources officielles

### Claude Code

La documentation officielle :

- définit `~/.claude/skills/<skill-name>/SKILL.md` comme emplacement personnel ;
- indique qu'à partir de Claude Code 2.1.203 une entrée `<skill-name>` peut être un symlink vers
  un dossier et que Claude suit ce lien ;
- indique que les modifications sont suivies à chaud, sauf quand le dossier de skills racine
  n'existait pas au démarrage, auquel cas un redémarrage est requis.

Source : [Extend Claude with skills](https://code.claude.com/docs/en/skills).

### Codex

La documentation officielle :

- définit `$HOME/.agents/skills` comme emplacement utilisateur ;
- indique explicitement que Codex suit les dossiers de skills symlinkés ;
- annonce une détection automatique des nouveaux skills, avec redémarrage recommandé si un skill
  n'apparaît pas.

Source : [Build skills](https://developers.openai.com/codex/skills/).

### Cursor

La documentation officielle définit `.agents/skills`, `.cursor/skills`,
`~/.agents/skills` et `~/.cursor/skills`, et reconnaît aussi les dossiers historiques Claude et
Codex. Elle indique que les skills sont découverts au démarrage, mais ne promet pas le support des
symlinks ni le rechargement à chaud. Ces deux points sont donc décidés uniquement à partir des
tests runtime ci-dessous.

Source : [Agent Skills](https://cursor.com/docs/skills.md).

## Validation runtime par plateforme

| OS | Agent et version | Type testé | Résultat | Redémarrage | Binding activé |
| --- | --- | --- | --- | --- | --- |
| macOS | Claude Code 2.1.220 | Symlink de dossier dans `.claude/skills` | Le skill sentinelle est découvert et renvoie `SKILLREG_RUNTIME_PROBE_OK`. | Non pour un dossier racine déjà présent ; oui par prudence si SkillReg doit créer ce dossier après le démarrage. | Oui |
| macOS | Codex CLI 0.145.0 | Symlink de dossier dans `.agents/skills` | Le skill sentinelle est découvert et renvoie `SKILLREG_RUNTIME_PROBE_OK`. | L'adaptateur conserve `requires_restart_after_binding=true`, conformément au fallback documenté. | Oui |
| macOS | Cursor Agent 2026.07.23-e383d2b | Symlink de dossier dans `.cursor/skills` | Le skill sentinelle est découvert et renvoie `SKILLREG_RUNTIME_PROBE_OK`. | Oui par prudence : seul le chargement au démarrage a été validé. | Oui |
| Linux | Claude Code | Non exécuté sur un hôte Linux | Emplacement documenté, comportement du lien non vérifié dans SkillReg. | Inconnu | Non |
| Linux | Codex | Non exécuté sur un hôte Linux | Emplacement documenté, comportement du lien non vérifié dans SkillReg. | Inconnu | Non |
| Linux | Cursor Agent | Non exécuté sur un hôte Linux | Emplacement documenté, comportement du lien non vérifié dans SkillReg. | Inconnu | Non |
| Windows | Claude Code | Junction non testée dans l'agent | Le format de binding Windows choisi ci-dessous reste à valider. | Inconnu | Non |
| Windows | Codex | Junction non testée dans l'agent | Le format de binding Windows choisi ci-dessous reste à valider. | Inconnu | Non |
| Windows | Cursor Agent | Junction non testée dans l'agent | Le format de binding Windows choisi ci-dessous reste à valider. | Inconnu | Non |

Le test macOS a été exécuté le 2026-07-29 avec
`src-tauri/tests/fixtures/agents/runtime-probe/SKILL.md`. Chaque agent a été lancé depuis un
répertoire temporaire contenant uniquement un lien vers cette fixture. Aucun dossier personnel de
skills n'a été modifié. Les sessions Claude et Codex ont été non persistantes ; le répertoire
temporaire et ses trois liens ont été supprimés après vérification.

Le comportement runtime d'un lien cassé n'est pas revendiqué par SkillReg : il n'est ni documenté
uniformément ni nécessaire au contrat produit. Le gestionnaire détecte le lien cassé avant
l'agent, le marque `missing` et ne le répare que si la preuve de propriété du manifest correspond.

## Décision Windows

Trois options ont été comparées avant l'implémentation :

| Option | Avantage | Limite pour un produit non technique | Décision |
| --- | --- | --- | --- |
| `std::os::windows::fs::symlink_dir` | API standard, aucun paquet supplémentaire | La création peut exiger le Developer Mode, le privilège `SeCreateSymbolicLinkPrivilege` ou une exécution administrateur. | Rejetée comme chemin par défaut |
| API reparse point Win32 directe | Contrôle complet des junctions | Code bas niveau sensible pour créer, inspecter et supprimer le bon reparse tag ; surface de sécurité et de maintenance disproportionnée. | Rejetée |
| Crate ciblée `junction` 2.x | API minimale `create`, `delete`, `exists`, `get_target`, création sans élévation sur NTFS et suppression du seul reparse point | Windows/NTFS uniquement ; une junction doit rester sur un volume compatible et être testée dans chaque agent. | Retenue, sous `cfg(target_os = "windows")` |

Sources :

- [Rust `symlink_dir`](https://doc.rust-lang.org/std/os/windows/fs/fn.symlink_dir.html)
- [Microsoft `CreateSymbolicLinkW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw)
- [Microsoft reparse point operations](https://learn.microsoft.com/en-us/windows/win32/fileio/reparse-point-operations)
- [Crate `junction` 2.0.0](https://docs.rs/junction/latest/junction/)

Il n'existe aucun fallback vers une copie physique. Tant que les trois tests runtime Windows ne
sont pas renseignés dans cette matrice, les adaptateurs retournent `unsupported` sur Windows,
même si le gestionnaire de liens sait créer et vérifier une junction.

## Critères pour activer une nouvelle ligne

1. Utiliser un profil temporaire et la fixture sentinelle.
2. Vérifier le chemin préféré et un lien pointant vers un dossier hors du dossier agent.
3. Vérifier la découverte après démarrage et, séparément, le comportement à chaud.
4. Vérifier qu'un lien cassé n'entraîne ni crash ni suppression de contenu.
5. Relever la version exacte de l'agent, l'OS, le système de fichiers et le type de lien.
6. Ajouter une fixture d'usage distincte avant toute capability `partial` ou `exact`.
7. Mettre à jour le test `supports_managed_links` avant d'activer la plateforme.
