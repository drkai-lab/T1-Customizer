# Microsoft Store distribution (the free SmartScreen fix)

Goal: stop the "Windows protected your PC" (SmartScreen) prompt for
T1-Customizer **without paying for a certificate**. SignPath Foundation
rejected the application (not enough public trust signals yet), and there
is no free publicly-trusted code-signing certificate for direct downloads
(a self-signed certificate behaves exactly like no signature for
SmartScreen). The one free route that removes the prompt entirely is the
**Microsoft Store**: Store-distributed apps are re-signed by Microsoft
and are never subject to SmartScreen download warnings.

This document records the verified facts, what is already prepared in
this repo (branch `store-msix`), and the steps only the maintainer can do.

## Verified facts (Microsoft sources, checked 2026-11-28)

### Registration is free for individuals, including from Japan

- The new onboarding flow at **storedeveloper.microsoft.com** waives the
  old $19 individual registration fee entirely; no credit card is needed.
  The flow is: sign in with a Microsoft account → begin identity
  verification → capture a government-issued ID and a selfie on mobile →
  complete the profile → Partner Center dashboard.
  ([Steps to open a developer account](https://docs.microsoft.com/en-gb/windows/uwp/publish/account-types-locations-and-fees),
  [Free developer registration for individual developers](https://learn.microsoft.com/en-us/windows/apps/publish/whats-new-individual-developer))
- The free flow is "globally available in nearly 200 markets", and ID
  verification is "supported in all markets worldwide"; Japan is a
  Microsoft Store market. So a Japan-based individual can register free.
  ([Windows Developer Blog, 2025-09-10](https://blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/))
- Identity verification (government ID + selfie) is inherently personal:
  only the maintainer can do it. It cannot be delegated.

### Store-installed apps never show the SmartScreen prompt

- Microsoft's own developer guidance: "The simplest way to avoid
  SmartScreen warnings is to publish through the Microsoft Store.
  Store-distributed apps are signed by a Microsoft certificate and are
  never subject to SmartScreen download warnings."
  ([SmartScreen reputation for Windows app developers](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation))
- For MSIX submissions the Store re-signs the package after
  certification; the developer never buys or manages a certificate, and
  the comparison table rates the Store (MSIX) route "No warnings".
  ([Code signing options for Windows app developers](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options))
- This also covers Windows 11 **Smart App Control**, which can block
  unsigned executables outright: a Store-signed package carries
  Microsoft's signature and full reputation.
- Why the other Microsoft option does not help here: Azure Artifact
  Signing (formerly Trusted Signing) is limited to individuals in the
  USA and Canada only (organizations in more countries, but this project
  publishes as an individual in Japan), and even then reputation builds
  gradually. ([same code-signing options page](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options))

### Packaging model: full-trust MSIX

- The app ships as a **packaged classic Win32 (full-trust) MSIX**:
  `EntryPoint="Windows.FullTrustApplication"` plus the restricted
  capability `runFullTrust`. The processes run as the user, outside the
  AppContainer sandbox, with the same rights as the unpackaged build.
  ([Generating MSIX package components / manual conversion](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion))
- **USB HID access works unchanged.** The app talks to the keyboard
  through the `hidapi` crate, i.e. the classic Win32 HID/SetupAPI DLLs.
  Full-trust processes can use Win32 device APIs exactly like the
  unpackaged build. The WinRT `Windows.Devices.HumanInterfaceDevice`
  API would require `DeviceCapability` manifest entries; this app does
  not use it, so **no device capabilities are declared**.
- **Process model works unchanged.** The resident monitor is not a
  separate binary: the GUI relaunches its own executable with
  `--monitor` (`resident.rs: start_monitor_process`, via
  `current_exe`), and the console executable hands GUI launches off to
  the GUI executable. Both executables live in the package root, so all
  self-spawning stays inside the package. The tray icon, the pair-chord
  keyboard hook, and `SendInput` synthesis are ordinary Win32 and work
  under full trust. The app installs **no Windows service, no driver,
  and no registry autostart entry**, so there is nothing for MSIX
  virtualization to break.
- **Settings keep working, in a different physical place.**
  `settings.json` lives in `%APPDATA%\t1-keyboard` and state files
  (pid/lock, fallback log) in `%APPDATA%\t1-keyboard\state`
  (`rust/src/paths.rs`). For a packaged app, Windows redirects AppData
  writes into the package's private per-user data container
  (`%LOCALAPPDATA%\Packages\<PackageFamilyName>\LocalCache\Roaming\...`).
  Settings persist and behave the same from the app's point of view, but
  the Store build and the Inno Setup build do **not** share settings,
  and the files are not where an unpackaged user would look for them.

### ⚠️ The one user-visible behavior change: `monitor.log` location

`paths::log_file()` prefers `monitor.log` **next to the executable**
(the maintainer's explicit request, 2026-11-27) and probes the exe
folder's writability with a throwaway create-file test
(`dir_is_writable`), falling back to the state dir when read-only.

In an MSIX install the executable lives under
`C:\Program Files\WindowsApps\<package>\`, which denies writes, so the
probe fails and the log **silently falls back to the state dir**
(redirected into the package container, see above). Logging keeps
working and the pair-chord/preset diagnostics are all still recorded —
but "the log is next to the exe" does **not** hold for the Store build.
When asking a Store user for their `monitor.log`, point them at the
state dir inside the package container, or have them set `T1_STATE_DIR`.
A future improvement (not done here) would be to detect package
identity in `log_file()` and surface the real log path in the GUI.

### Store policy notes

- Hardware-configuration utilities are a normal Store category (vendor
  keyboard/mouse tools ship there); nothing in Store policy bans a
  utility that configures the user's own USB device. The app is free,
  has no in-app purchases, no ads, and no network features.
- `runFullTrust` is a **restricted capability**: Partner Center shows a
  warning on package upload (normal for every packaged desktop app, not
  a rejection) and requires a written justification in the submission's
  **Submission options** step. A human reviewer reads it; genuine
  desktop tools are routinely approved, but it can add time.
  Ready-to-paste justification:

  > T1-Customizer is a full-trust Win32 desktop application packaged as
  > MSIX. runFullTrust is required to launch its bundled executables: it
  > communicates with a USB HID macro keyboard through native Win32 HID
  > APIs, runs a resident monitor process (the same executable
  > relaunched with --monitor) that uses a low-level keyboard hook and
  > SendInput to implement two-key chord assignments and dial volume
  > control, and shows a tray icon. It installs no services or drivers,
  > requests no administrator elevation, and accesses no data beyond
  > its own settings files.

- The submission also needs: a **privacy policy URL** (certification
  commonly asks full-trust apps for one — a one-paragraph page stating
  the app runs entirely on-device and collects/transmits no data is
  enough), the **IARC age-rating questionnaire**, listing text, and
  screenshots. The 300×300+ Store logo can come from `icons/app-512.png`.

## What is prepared (branch `store-msix`)

- `packaging/msix/Package.appxmanifest` — full-trust manifest for both
  executables. The GUI exe is the Start entry point; the console exe
  carries a `t1-keyboard-config.exe` execution alias so the CLI
  (`--monitor` / `--watch` / `--selftest` / `--version`) keeps working
  from a terminal. Identity values are clearly marked
  `REPLACE-WITH-PARTNER-CENTER-*` placeholders. Only `runFullTrust` is
  declared. Languages: ja-JP, en-US, ko-KR.
- `packaging/msix/Assets/` — StoreLogo (50×50), Square150x150Logo,
  Square44x44Logo, Wide310x150Logo, generated from `icons/app-1024.png`.
- `windows/build-msix.ps1` — builds the release exes exactly like
  `build-release.ps1` (or `-SkipBuild` to reuse `dist-win`), reads the
  version from the executable, stages manifest + assets + exes, and
  packs an **unsigned** `dist-win\t1-customizer-<ver>.msix` with the
  Windows SDK's `makeappx.exe`. It warns (but still builds) while the
  Partner Center placeholders are in the manifest.
- The existing Inno Setup release flow is untouched; the two channels
  are built and shipped independently.

## Steps only the maintainer can do (in order)

1. **Register** at <https://storedeveloper.microsoft.com> → "Get started
   for free" → **Individual developer** → sign in with your Microsoft
   account → complete the **government-ID + selfie verification** on
   your phone → finish the profile. (Free; personal; ~minutes.)
2. **Reserve the app name**: Partner Center → Apps and games → New
   product → **MSIX or PWA app** → reserve `T1-Customizer` (if the exact
   name is taken, reserve a close variant; the listing display name can
   still read "T1-Customizer" if policy allows).
3. **Copy the product identity**: Partner Center → your app → Product
   management → **Product identity**. Replace the three placeholders in
   `packaging/msix/Package.appxmanifest` with the shown
   `Package/Identity/Name`, `Package/Identity/Publisher`, and publisher
   display name. Commit that change (it is account-specific but not a
   secret).
4. **Build the package** on a Windows machine with the Windows 10/11
   SDK installed: `pwsh windows/build-msix.ps1`. **Do not sign** the
   resulting `.msix` — a pre-signed package mismatches the Store
   identity; the Store signs it itself.
5. **Create the submission** in Partner Center: upload the `.msix`
   (Packages), set Properties (category: Utilities & tools), add the
   privacy policy URL, complete the IARC age rating, write the Store
   listing (description, screenshots, logo), and in **Submission
   options** paste the `runFullTrust` justification above.
6. **Submit for certification** and answer any certification notes.
   Review typically takes hours to a few days; approval is not
   guaranteed. After approval the Store signs and publishes the package;
   users installing from the Store never see SmartScreen.
7. For each **update**: bump the app version as usual, rebuild the
   `.msix`, and submit a new submission with the higher version.

## Honest limitations

- **Only the Store channel is prompt-free.** The GitHub Releases
  installer and portable zip stay unsigned and will keep triggering
  SmartScreen on every release (unsigned files build reputation per
  file hash, starting from zero each version). The README should point
  users who dislike the prompt at the Store listing once it exists.
- **Review takes time and can say no.** Certification (including the
  `runFullTrust` justification) is a human/automated review on every
  submission; a rejection means fixing the notes and resubmitting.
- **The unsigned `.msix` cannot be sideload-tested as-is.** Installing
  it before submission requires a self-signed certificate whose subject
  matches the manifest Publisher (plus Developer Mode / trusted cert).
  The practical pre-submission test is a Partner Center package flight
  or a throwaway local signing with the Publisher temporarily set to
  the test certificate's subject.
- **Behavior differences of the Store build** (see above): `monitor.log`
  lands in the redirected state dir instead of next to the exe, and
  settings live in the package's private container, separate from the
  Inno Setup build's settings.
- The publisher name users see will be the Partner Center **publisher
  display name** — register it as "AppWizz Kai" so the Store listing
  matches the name shown in the license and the app.
