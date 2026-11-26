# Getting started

Use non-sensitive sample logs while evaluating this release. Extract a portable package
into a writable directory and keep portable.flag beside tylogger.exe.

```text
tylogger.exe sample.log
```

Without a file argument the application creates demonstration data. Source log reading
uses shared handles so another application can continue writing. Test rotation/truncation
behavior against copies of your own log format before relying on it.
The workspace provides native menus and a log grid; keyboard shortcuts are documented
in COMMANDS.md. Module availability does not establish complete end-user feature coverage.

Settings, bookmarks and workspace/session paths can contain private information.
Review exported files before sending a bug report. Keep a backup of your configuration.
Do not configure production remote credentials: live SSH/SFTP support is not verified.
External tools and executable plugins run code with your account's permissions.

Use --headless-test only for synthetic validation. It does not certify the GUI or a
particular Windows version. Consult PUBLIC_RELEASE_STATUS.md for release limitations.
