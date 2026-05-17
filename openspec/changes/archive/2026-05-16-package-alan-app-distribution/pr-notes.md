## PR Notes

- Breaking change: `just app` and the force-kill debug app runner are removed.
- `just install` is now the local release install path. It assembles signed
  `Alan.app`, embeds `alan` and `alan-tui`, installs the app, and links the
  embedded tools into a configurable PATH directory without using `~/.alan/bin`.
- Formal signing is required. Release assembly fails closed unless
  `ALAN_DEVELOPER_ID_APPLICATION` or `ALAN_SIGNING_IDENTITY` names a Developer
  ID Application identity; ad-hoc signing is not a fallback.
- Local private env loading is centralized in `scripts/release-env.sh`.
  Explicit env vars win; otherwise the scripts parse allowlisted variables from
  `ALAN_RELEASE_ENV_FILE` or release-specific local env files such as
  `.env.release.local`, `.env.local`, and `.env`, then `~/.alan/release.env`,
  including `ALAN_DEVELOPER_ID_APPLICATION` and Apple ID app-specific-password
  notarization settings.
- Homebrew distribution is cask-first: `brew install --cask alan` installs
  `Alan.app` and links `Contents/Resources/bin/alan` plus
  `Contents/Resources/bin/alan-tui`.
- Direct `.app` installs can use the macOS **Tools > Install Command Line
  Tools...** action to create PATH-visible symlinks, while refusing Homebrew
  prefixes, existing Homebrew-managed links in other prefixes, non-alan-owned
  targets, and `~/.alan/bin`.
- Public release is one-command after `.env` is configured: `just release-check`
  validates signing/notarization and creates or refreshes the notary keychain
  profile; `just release` builds, signs, notarizes, staples, and archives.
- `.env.example` documents the canonical local release variables; real `.env`
  remains ignored.
