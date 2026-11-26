# Tylogger

A native Windows log viewer and analysis project written in Rust with Win32/GDI.
Developed and maintained by **Lazy Coder**.

[![CI](https://github.com/alfisyahry/tylogger/actions/workflows/ci.yml/badge.svg)](https://github.com/alfisyahry/tylogger/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Release status

This is an early public source release. The code identifies itself as version 1.0.0,
but its previous internal GA label is **not a claim of production readiness**.
Compatibility, installer and long-running validation remain incomplete.
Use candidate downloads for evaluation with copies of non-sensitive logs.

## Overview

Tylogger is an independent project inspired by
[LogExpert](https://github.com/LogExperts/LogExpert), not an official LogExpert release.
It uses Rust and native Windows controls rather than a browser or .NET application shell.

The source includes:

- Local log reading with shared file handles, sparse line indexing and append/truncation handling.
- A native multi-document workspace, selection, bookmarks and configurable themes.
- Search/filter, columnizer, timestamp, merge and investigation modules.
- Analytics, archive, workspace/session and plugin SDK modules.
- Remote-source interfaces and synthetic transport tests.

The presence of a module or a passing unit test does not establish complete UI integration.
In particular, a production SSH/SFTP backend has not been demonstrated: the checked-in
`RemoteTransport` implementation is an in-memory mock. Do not rely on remote monitoring
or infer complete LogExpert feature parity from the engineering documents.

## Download and requirements

Candidate assets, when available, are listed under [Releases](https://github.com/alfisyahry/tylogger/releases).
Source archives are not compiled applications. See the individual release notes for asset
hashes, provenance and outstanding checks.

- Native Windows x86 and x64 targets; a desktop session is needed for the GUI.
- Static CRT build flags; no mandatory .NET or browser runtime for the main application.
- Windows 7 SP1 through Windows 11 are engineering targets, not a verified execution matrix.
- Current local checks were performed on Windows 10. Windows 7 execution and the complete
  Windows 8/8.1/11 matrix have not been independently verified for this release.
- Published evaluation binaries are unsigned. A compiled installer is not currently supplied.
- Core local-file use is offline. Optional external tools/plugins can execute processes;
  read [PRIVACY.md](PRIVACY.md) before enabling them.

## Portable use and quick start

Extract the portable archive into a writable folder and keep `portable.flag` beside
`tylogger.exe`. Open a non-sensitive log with:

```text
tylogger.exe sample.log
```

Running without a file opens synthetic demonstration data. Use the File menu for local
files. See [commands and shortcuts](docs/COMMANDS.md) and the [user guide](docs/USER_GUIDE.md).
Formatting modules cover plain text, delimited records, JSON Lines, syslog and other
structured formats; inspect the relevant parser and test coverage for your actual data.

## Build from source

Install Microsoft C++ Build Tools with a Windows SDK and Rust through rustup.
The repository pins the locally verified Rust **1.97.1** toolchain and the two MSVC targets.

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked --target x86_64-pc-windows-msvc
cargo build --release --locked --target i686-pc-windows-msvc
target\x86_64-pc-windows-msvc\release\tylogger.exe --headless-test
target\i686-pc-windows-msvc\release\tylogger.exe --headless-test
```

Preserve the compatibility flags in `.cargo/config.toml`. Hosted Windows CI checks the
build and automated suites; it does not prove execution on older Windows versions.
Build scripts and installer sources live under `scripts/` and `installer/`.

## Verification and performance

The existing x86 and x64 evaluation executables each passed 110 synthetic headless checks
during publication preparation. Their sizes were 1,010,176 and 1,113,600 bytes respectively.
These are executable file sizes, not RAM usage, installation size or performance measurements.
No general speed advantage over LogExpert is claimed.

The headless suite tests selected core behavior and creates temporary synthetic files.
It does not replace GUI interaction, real SSH/SFTP sessions, a physical large-file workload,
a 24-hour endurance run, installer lifecycle testing or an OS compatibility lab.
See [release limitations](docs/PUBLIC_RELEASE_STATUS.md) for remaining work.

## Architecture and extensions

`src/core/` contains file reading, indexing, record processing and investigation modules.
`src/gui/` contains the Win32/GDI interface. Configuration, session and workspace handling
are implemented in the source; settings may be stored in a user profile or portable folder.
The [SDK](sdk/README.md) contains schemas, protocol documentation and examples.
External executable plugins must be treated as code with access to the current user's resources.

The [architecture](docs/ARCHITECTURE.md), [design contract](docs/DESIGN.md) and
[feature matrix](docs/FEATURE_PARITY.md) retain engineering context. They describe design
and implementation intent; they are not a certificate of complete feature or security validation.
Generated UI illustrations from internal development are intentionally omitted. No mock image
is presented as a screenshot of a running application.

## LogExpert comparison

| Area | Tylogger public snapshot | LogExpert reference |
| --- | --- | --- |
| Implementation | Rust, Win32/GDI | C# project; see upstream |
| Local files / live-tail logic | Native implementation and synthetic checks | See pinned upstream source |
| Search, filtering and investigation | Modules exist; UI parity requires further review | Existing upstream feature set |
| Remote sources | Interfaces and mock transport; production backend unverified | No comparative claim made |
| Runtime | Native executable; static CRT flags | Consult upstream system requirements |
| Relative performance / complete parity | Not established | No superiority claim made |

Upstream reference: commit `a147ed316cca14f4d90a41186079a116f91ef9ee`.

## Contributing, security and roadmap

Read [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md),
[PRIVACY.md](PRIVACY.md) and the [Code of Conduct](CODE_OF_CONDUCT.md).
Near-term work includes real GUI acceptance checks, remote transport integration,
compatibility testing, an actual installer build and release-signing arrangements.
See [CHANGELOG.md](CHANGELOG.md) for development history; prior milestone labels do not
supersede the qualification of the current public release above.

## License and credits

MIT. Maintained by Lazy Coder; existing contributor authorship is preserved.
LogExpert and dependency notices are retained in [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt).
