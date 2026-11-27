# SignPath Foundation — application draft & setup checklist (T1-Customizer)

Status: PREPARED, NOT SUBMITTED. Submitting the application, creating the
SignPath account, and creating the API token are maintainer-only steps
(see the checklist at the bottom). Nothing in this document is a credential.

## 1. Eligibility check (against the SignPath Foundation conditions)

Source: SignPath Foundation conditions for Open Source projects,
https://signpath.org/terms.html (read 2026-11-28)

| Condition (Foundation terms) | T1-Customizer status |
| --- | --- |
| OSI-approved OSS license, no commercial dual-licensing | OK — MIT License (`LICENSE`, "Copyright (c) 2026 AppWizz Kai") |
| No proprietary / non-open-source components in signed packages | OK — the installer/zip contain only the project's own two executables, icons and the LICENSE file. (Inno Setup is a build tool; it is not a signed component.) |
| No malware / potentially unwanted programs | OK — a keyboard configuration utility; no network exfiltration, no system modification beyond writing the device and its own settings |
| Not a hacking / security-circumvention tool | OK — it configures a USB macro keyboard; it does not scan for or exploit vulnerabilities |
| Actively maintained | OK — active development, releases v1.0.0–v1.0.5 (Nov 2026) |
| Already released in the form that should be signed | OK — GitHub Releases ship exactly the artifacts to be signed (Inno Setup installer, portable zip, two standalone exes) |
| Functionality documented on the download page | OK — README (EN/JA/KO/ZH) describes the tool; releases are on GitHub Releases |
| Sign your own projects / own binaries only | OK — T1-Customizer is a fork of the maintainer's own T1-VREmoKey (same owner, drkai-lab); all signed binaries are built from this repository by its own GitHub Actions workflow |
| Builds verifiable / from GitHub-hosted CI | OK — `.github/workflows/build.yml` builds on `windows-latest` / `ubuntu-latest` (GitHub-hosted) from source; SignPath requires all jobs leading to a signing request to run on GitHub-hosted runners |
| MFA for all team members (GitHub + SignPath) | ACTION REQUIRED — maintainer must enable MFA on both accounts |
| Code signing roles (Authors / Reviewers / Approvers) | ACTION REQUIRED — solo project: maintainer holds all roles; every release signing request needs manual approval by the Approver (the maintainer) in SignPath |
| "Code signing policy" published on the project home page | ACTION REQUIRED after approval — draft text in section 4 below |
| Signed binaries carry enforced product name / version metadata | TO CONFIGURE in the SignPath artifact configurations (file metadata restrictions: product name `T1-Customizer`, product version = the release version produced by the build) |

Risks / non-blocking notes:

- Acceptance is discretionary. The Foundation states that downloadable
  executables need "a certain verifiable reputation" (signpath.org/terms.html,
  "Common misunderstandings"). T1-Customizer is a young project (first
  release 2026-11-26), so a rejection or a request for more history is
  possible; that is the Foundation's call, not a hard rule we violate.
- The certificate is issued to "SignPath Foundation", so the publisher shown
  by Windows for signed files will be SignPath Foundation, not the maintainer.
- Every release requires a manual approval step in SignPath before signing
  completes. The CI workflow waits for completion (timeout raised to 1 hour);
  a release build will pause until the maintainer approves.

## 2. Application answers (copy/paste draft)

Apply via SignPath's open-source page: signpath.org → "Apply for Free Code
Signing" (the Foundation application route linked from
https://signpath.org/terms.html; the product page is
https://about.signpath.io/product/open-source).

- Project name: T1-Customizer
- Repository URL: https://github.com/drkai-lab/T1-Customizer
- Download / release page: https://github.com/drkai-lab/T1-Customizer/releases
- License: MIT (OSI-approved) — see LICENSE in the repository root
- Maintainer: drkai-lab (AppWizz Kai), sole maintainer
- Build system: GitHub Actions (GitHub-hosted runners), workflow
  `.github/workflows/build.yml`; releases are built from `v*` tags only
- Artifacts to be signed (all Authenticode):
  - `t1-keyboard-config.exe` and `t1-keyboard-config-gui.exe`
    (the application binaries; signed before packaging)
  - `t1-customizer-<version>-setup.exe` (Inno Setup installer containing
    the signed binaries; the installer itself is signed too)
  - The portable zip is not signed as a file; it ships the same signed exes.
- Project description (EN):
  "T1-Customizer is a free, open-source configuration tool for the T1 mini
  keyboard (a small USB macro pad with 6 keys and 2 rotary knobs). It writes
  key assignments, modifier combos, media/mouse actions, knob (dial)
  behaviour, key-pair chords and LED settings to the device over USB HID,
  and can save/load button configurations as JSON files. It is the free
  edition forked from the maintainer's own T1-VREmoKey project, with the
  VRChat/OSC features and the license-activation system removed. The GUI is
  available in Japanese, English and Korean. The program works fully
  offline and does not collect or transmit user data."
- Why signing is needed:
  "Releases are distributed as unsigned downloads from GitHub Releases, so
  Windows SmartScreen shows a blue 'Windows protected your PC' warning for
  every new user. Code signing via SignPath Foundation would let us
  distribute the installer and executables with a verifiable publisher and
  remove that barrier for a free open-source tool."

## 3. What the free tier includes (per SignPath / Foundation pages)

- A free SignPath.io subscription for the OSS project and signing with a
  certificate issued to SignPath Foundation (private key stays on
  SignPath's HSM; no local signing — signing happens only through the
  CI-connected service).
- GitHub Actions integration via the official action
  `signpath/github-action-submit-signing-request@v3`
  (docs: https://docs.signpath.io/trusted-build-systems/github).
  Required inputs: `api-token`, `organization-id`, `project-slug`,
  `signing-policy-slug`, `github-artifact-id` (from a preceding
  `actions/upload-artifact` step); `wait-for-completion` defaults to true;
  `output-artifact-directory` receives the signed files;
  `artifact-configuration-slug` selects which artifact configuration
  (file set + metadata rules) applies.
- Origin verification: SignPath checks the artifact was really built by
  the linked GitHub workflow and stored as a workflow artifact first.

## 4. Draft "Code signing policy" for the README (add AFTER approval)

The Foundation terms require a section headed "Code signing policy" on the
project home page and release pages, containing the attribution line and
the team roles. Prepared text:

```markdown
## Code signing policy

Free code signing provided by [SignPath.io](https://signpath.io),
certificate by [SignPath Foundation](https://signpath.org).

- Committers and reviewers: the repository owner (drkai-lab) — all changes
  are made or reviewed by the maintainer.
- Approvers: drkai-lab (repository owner). Every release signing request
  is manually approved by the approver before SignPath signs it.
- Only binaries built by this repository's GitHub Actions workflow from
  tagged releases are submitted for signing; locally built binaries are
  never signed.

Privacy policy: This program will not transfer any information to other
networked systems unless specifically requested by the user or the person
installing or operating it.
```

## 5. Maintainer-only checklist (only the user can do these)

SignPath account / application:

1. Enable MFA on the GitHub account (required by the Foundation terms),
   and on the SignPath account once created.
2. Submit the Foundation application (section 2) from signpath.org /
   the SignPath open-source page. Any identity or organisation questions
   in the form must be answered by the maintainer personally.
3. Wait for approval e-mail from SignPath Foundation.

SignPath configuration (in the SignPath web UI, after approval):

4. Note the Organization ID (GUID) of the new organization.
5. Add the predefined Trusted Build System "GitHub.com" to the
   organization and link it to the project; install the SignPath GitHub
   App on the `drkai-lab/T1-Customizer` repository when prompted
   (needed for origin verification / audit log).
6. Create a Project for T1-Customizer → note its project slug
   (suggested: `t1-customizer`).
7. Create a Signing Policy bound to the Foundation certificate → note its
   signing-policy slug (suggested: `release-signing`). Configure it so
   signing requests require approval by the Approver.
8. Create two Artifact Configurations in the project:
   - slug `app-exes`: root element = the ZIP produced by
     `actions/upload-artifact`, containing `t1-keyboard-config.exe` and
     `t1-keyboard-config-gui.exe`, both Authenticode-signed; set file
     metadata restrictions (product name `T1-Customizer`, product version
     = build version).
   - slug `installer`: root element = the ZIP produced by
     `actions/upload-artifact`, containing the single
     `t1-customizer-<version>-setup.exe`, Authenticode-signed, same
     metadata restrictions.
9. Create a CI user / API token with submitter permission for the project
   and policy.

GitHub repository settings (Settings → Secrets and variables → Actions):

10. Secret: `SIGNPATH_API_TOKEN` = the API token from step 9.
11. Variables:
    - `SIGNPATH_ORGANIZATION_ID` = Organization ID (step 4)
    - `SIGNPATH_PROJECT_SLUG` = project slug (step 6)
    - `SIGNPATH_SIGNING_POLICY_SLUG` = signing policy slug (step 7)
    - optional: `SIGNPATH_ARTIFACT_CONFIG_EXES` (default `app-exes`),
      `SIGNPATH_ARTIFACT_CONFIG_INSTALLER` (default `installer`)
12. LAST, as the on-switch: variable `SIGNPATH_ENABLED` = `true`.
    Until this variable is exactly `true`, tag builds behave exactly as
    before (no signing steps run) — see the `signpath-ci` branch workflow.
13. Add the "Code signing policy" section (section 4) to the README /
    release notes (required by the Foundation terms).
14. Cut the next release tag as usual; when the windows job reaches the
    signing steps, approve the two signing requests in the SignPath UI
    (app exes first, installer second) so the build can finish and
    publish signed assets.
