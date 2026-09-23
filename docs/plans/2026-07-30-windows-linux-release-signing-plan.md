# Signature des releases Windows et Linux

Le workflow `.github/workflows/release.yml` exige les signatures Windows, Linux et
Tauri avant de créer une release **en brouillon**. Axel conserve la publication
manuelle. Cette procédure n'autorise pas à déclencher une release pendant l'usine.

## Trois protections complémentaires

- Windows : Authenticode via Azure Artifact Signing, avec horodatage RFC 3161.
  Le hook Tauri signe le binaire applicatif puis les installateurs avant la création
  des archives updater. SignTool et PowerShell vérifient les signatures.
- Linux : signatures OpenPGP détachées ASCII `.asc` sur les fichiers finaux `.deb`,
  `.rpm`, `.AppImage` et `.AppImage.tar.gz`. La vérification utilise une clé publique
  et une empreinte primaire configurées indépendamment des fichiers téléchargés.
- Updater : les `.sig` Tauri restent produits avec la clé existante et intégrés à
  `latest.json`. Ils ne remplacent ni Authenticode ni OpenPGP. Leur validation
  cryptographique reste celle du plugin updater Tauri.

Le contrat `createUpdaterArtifacts: "v1Compatible"`, les noms des plateformes,
la clé publique et l'endpoint Tauri ne changent pas. NSIS reste le canal updater
Windows ; les archives `.msi.zip` ne sont pas distribuées. macOS conserve sa
signature Developer ID, la notarisation et l'agrafage des DMG.

## Configuration préalable par Axel

La présence réelle de ces paramètres dans GitHub n'a pas été consultée. Aucune
opération de compte ou de facturation ne fait partie de l'implémentation.

| Type GitHub | Nom | Rôle |
| --- | --- | --- |
| Variable | `AZURE_CLIENT_ID` | Application ou identité utilisée par OIDC |
| Variable | `AZURE_TENANT_ID` | Tenant Azure |
| Variable | `AZURE_SUBSCRIPTION_ID` | Abonnement Azure |
| Variable | `AZURE_SIGNING_ENDPOINT` | Endpoint de la région du compte Artifact Signing |
| Variable | `AZURE_SIGNING_ACCOUNT` | Nom du compte Artifact Signing |
| Variable | `AZURE_SIGNING_CERTIFICATE_PROFILE` | Profil de certificat Public Trust |
| Variable | `WINDOWS_SIGNING_SUBJECT` | Sujet X.509 complet et exact de l'éditeur attendu |
| Secret | `LINUX_SIGNING_PRIVATE_KEY` | Clé privée OpenPGP armurée, protégée par passphrase |
| Secret | `LINUX_SIGNING_PASSPHRASE` | Passphrase non vide de cette clé |
| Variable | `LINUX_SIGNING_PUBLIC_KEY` | Clé publique OpenPGP armurée de release |
| Variable | `LINUX_SIGNING_KEY_FINGERPRINT` | Empreinte primaire complète, hexadécimale |
| Secret existant | `TAURI_SIGNING_PRIVATE_KEY` | Clé de l'updater ; absence bloquante |
| Secret existant | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Mot de passe Tauri, optionnel selon la clé |

Les secrets Apple existants restent requis : `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`,
`APPLE_PASSWORD`, et `APPLE_TEAM_ID` selon la configuration existante.

Dans Azure, préparer un compte Artifact Signing avec identité validée et profil
Public Trust. Configurer une fédération OIDC correspondant aux tags autorisés de
`Tontoon7/skillreg-local`, et le rôle de signature limité au profil. Le sujet
attendu provient du certificat du profil, pas d'une chaîne inventée. Une rotation
des certificats Azure ne nécessite pas de changement d'empreinte dans le dépôt :
le contrôle porte sur la chaîne de confiance et le sujet exact.

Le workflow accorde `contents: write` uniquement au job `publish` et
`id-token: write` au job de build. Seule la ligne Windows de la matrice ouvre une
session Azure. Les métadonnées Dlib excluent toutes les méthodes documentées
autres qu'Azure CLI, qui utilise la session OIDC. L'action Azure nettoie la session
en fin de job ; la configuration temporaire Windows est également supprimée,
y compris après échec.

## Chaîne de contrôle

1. Exécuter les tests d'inventaire sur chaque runner, les tests GPG sur Linux et
   les tests PowerShell sur Windows, avant les secrets de signature. Contrôler
   les variables publiques, la cohérence du tag et des versions applicatives.
2. Installer sous `RUNNER_TEMP` les paquets Microsoft
   `Microsoft.Windows.SDK.BuildTools` **10.0.26100.9169** et
   `Microsoft.ArtifactSigning.Client` **1.0.128**, avec .NET 8 x64.
   Les nouvelles actions de préparation et de connexion sont épinglées à un SHA.
3. Générer un overlay Tauri Windows contenant `signCommand` sous forme d'objet
   `cmd` / `args`. Les chemins absolus et `%1` restent des arguments séparés,
   même avec des espaces. Aucun changement de configuration des builds locaux.
4. Le wrapper Windows signe en SHA-256, horodate avec
   `http://timestamp.acs.microsoft.com`, puis exige le code zéro de
   `signtool verify /pa /all /tw`, un statut Authenticode `Valid`, le sujet attendu
   et un certificat d'horodatage. Un avertissement SignTool est bloquant.
5. Sélectionner exactement les familles attendues dans le répertoire `bundle`
   du target construit, sans rechercher des exécutables dans tout `target/`.
   Les signatures Tauri doivent exister, être non vides et accompagner l'archive.
6. Sous Linux, charger les secrets uniquement dans l'étape de signature.
   Importer la clé privée dans un trousseau temporaire privé, contrôler son
   empreinte, signer les quatre fichiers puis les vérifier dans un second
   trousseau public. Contrôler `VALIDSIG` et l'empreinte primaire, y compris pour
   une sous-clé, et refuser une primaire ou un signataire expiré/révoqué. Une ancienne
   sous-clé expirée inutilisée ne bloque pas une nouvelle sous-clé valide. Distribuer la clé publique
   exportée sous `skillreg-linux-signing-key.asc`. Les agents GPG et trousseaux
   sont nettoyés même après erreur ; le trousseau personnel n'est jamais utilisé.
7. Sous Windows, revérifier le binaire applicatif, les installateurs EXE/MSI et
   l'unique EXE de l'archive NSIS. Son SHA-256 doit égaler celui du NSIS autonome.
8. Après les vérifications natives, `record` produit `verified-files.json` :
   plateforme, commit, version, identité attendue, tailles et SHA-256. L'upload
   échoue si aucun fichier n'existe. Cet inventaire relie les contrôles aux octets
   transférés ; ce n'est pas une signature cryptographique indépendante.
9. `publish`, dépendant du succès de toute la matrice, exige les quatre inventaires
   du même run. `prepare` refuse toute plateforme absente, identité incohérente,
   modification, collision, lien symbolique, chemin sortant ou fichier inattendu.
   Il prépare uniquement les téléchargements autorisés et un `latest.json` complet,
   puis contrôle aussi tailles et SHA-256 des copies finales avant d'écrire la liste
   de publication.
10. Vérifier à nouveau OpenPGP sur les fichiers reçus avec la clé et l'empreinte
    des variables GitHub. Seulement après cette étape, créer le brouillon avec la
    liste explicite des assets préparés. Aucun glob n'alimente la commande GitHub.

| Plateforme | Inventaire transféré obligatoire |
| --- | --- |
| `darwin-aarch64`, `darwin-x86_64` | Chacune : DMG, `.app.tar.gz`, `.app.tar.gz.sig` |
| `linux-x86_64` | `.deb`, `.rpm`, `.AppImage`, `.AppImage.tar.gz`, leurs quatre `.asc`, `.AppImage.tar.gz.sig`, clé publique |
| `windows-x86_64` | NSIS `.exe`, MSI `.msi`, `.nsis.zip`, `.nsis.zip.sig` |

Les inventaires et `.sig` ne sont pas des assets publics : les signatures Tauri
sont lues dans `latest.json`. Les `.asc` Linux et la clé publique sont distribuées.
Les archives macOS gardent les noms publics `SkillReg_aarch64.app.tar.gz` et
`SkillReg_x64.app.tar.gz`, sans modification de leurs octets.

## Vérification par l'utilisateur

Windows : consulter les signatures numériques dans les propriétés de l'EXE/MSI,
ou exécuter depuis un terminal Windows équipé du SDK :

```powershell
signtool verify /pa /all /tw .\SkillReg_installer.exe
Get-AuthenticodeSignature -LiteralPath .\SkillReg_installer.exe |
  Format-List Status, SignerCertificate, TimeStamperCertificate
```

Comparer le sujet de l'éditeur à celui annoncé pour SkillReg. Une signature
valide ne garantit pas l'absence de tout avertissement SmartScreen.

Linux : télécharger le fichier, son `.asc` et la clé publique depuis la release.
Comparer l'empreinte complète à celle annoncée dans les notes de release du
dépôt GitHub de confiance, avant l'import. Le trousseau suivant est isolé :

```bash
verification_home=$(mktemp -d)
chmod 700 "$verification_home"
gpg --homedir "$verification_home" --show-keys --with-fingerprint skillreg-linux-signing-key.asc
# Compare the complete primary fingerprint before continuing.
gpg --homedir "$verification_home" --import skillreg-linux-signing-key.asc
gpg --homedir "$verification_home" --verify SkillReg.AppImage.asc SkillReg.AppImage
gpgconf --homedir "$verification_home" --kill gpg-agent
rm -rf "$verification_home"
```

Adapter le nom au téléchargement réel ; répéter pour `.deb`, `.rpm` ou l'archive
`.AppImage.tar.gz`. Ces signatures détachées ne sont pas automatiquement vérifiées
par apt, rpm ou l'updater SkillReg. La confiance dans la clé vient de la comparaison
indépendante de l'empreinte, pas seulement du message « Good signature ».

## Validation et recette

```bash
pnpm format:check && pnpm build
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_release_artifacts.py' -v
bash scripts/test-linux-signing.sh
pwsh -NoProfile -File scripts/test-windows-signing.ps1
bash scripts/check-release-notarization.sh
actionlint .github/workflows/release.yml
node --test --experimental-strip-types tests/*.test.ts
```

Python teste les inventaires et transferts sur fichiers temporaires, sans prétendre
valider une signature réelle. Les tests GPG utilisent des clés éphémères et des
trousseaux dédiés. Les tests Windows distinguent doubles locaux et refus natif
d'un fichier non signé par SignTool. Leur indisponibilité locale ne doit jamais
être transformée en succès : ils sont obligatoires sur les runners correspondants.

La recette Azure réelle reste à effectuer lors d'une release autorisée et
configurée : vérifier les assets, installer NSIS et MSI sur une machine Windows
de test et contrôler la signature du binaire installé ; vérifier les téléchargements
Linux depuis un trousseau neuf ; tester la mise à jour depuis la version précédente
sur les quatre plateformes ; confirmer la notarisation macOS. Aucun compte Azure,
secret de release ou téléchargement de production n'est utilisé par les tests isolés.

La branche parallèle `feat/activation-reset` propose `createUpdaterArtifacts: true`.
Une fusion ultérieure doit réconcilier ses nouveaux formats avec cet inventaire ;
ne pas résoudre le conflit en supprimant les contrôles ou une plateforme.

## Références

- [Intégration SignTool et authentification Microsoft](https://learn.microsoft.com/en-us/azure/artifact-signing/how-to-signing-integrations)
- [Client Microsoft Artifact Signing 1.0.128](https://www.nuget.org/packages/Microsoft.ArtifactSigning.Client/1.0.128)
- [SDK BuildTools 10.0.26100.9169](https://www.nuget.org/packages/Microsoft.Windows.SDK.BuildTools/10.0.26100.9169)
- [Commandes et codes de retour SignTool](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool)
- [Hook de signature Windows Tauri](https://v2.tauri.app/distribute/sign/windows/)
- [Contrat updater Tauri](https://v2.tauri.app/plugin/updater/)
- [Fédération OIDC GitHub vers Azure](https://docs.github.com/en/actions/how-tos/secure-your-work/security-harden-deployments/oidc-in-azure)
- [Commandes opérationnelles GnuPG](https://www.gnupg.org/documentation/manuals/gnupg/Operational-GPG-Commands.html)
