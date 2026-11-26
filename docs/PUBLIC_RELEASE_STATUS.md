# Public release status

The source retains its historical 1.0.0 identifiers. This publication is an evaluation
snapshot, not independently verified GA. Historical engineering notes and milestone names
must not be read as current acceptance sign-off.

Publication preparation independently checked both existing PE architectures, archive CRCs,
manifest hashes and 110 synthetic headless checks per executable. Full Git history was
scanned using Gitleaks; no secret findings were reported. This is not proof of absence
of all privacy or security issues.

Open items include real GUI feature coverage, real SSH/SFTP transport integration,
physical Windows 7 testing, the full Windows 8/8.1/11 matrix, non-sparse large-file workloads,
the 24-hour endurance test, compiled-installer verification and Authenticode signing.
The only checked-in RemoteTransport implementation is a mock.

Internal reports, generated UI illustrations, local paths and build/cache artifacts were
removed from the publication history. Historical authors and commit attribution were
preserved. Compiled assets belong in Releases, not the source tree.
