pub mod browser;
pub mod client;
pub mod connection;
pub mod credential;
pub mod known_hosts;
pub mod multifile;
pub mod profile;
pub mod sftp_reader;
pub mod tail;

#[cfg(test)]
mod tests;

pub use browser::{SftpFileBrowser, SortColumn};
pub use client::{MockRemoteTransport, RemoteDirEntry, RemoteFileStat, RemoteTransport};
pub use connection::{
    calculate_backoff_delay, BoundedConnectionPool, CancellationToken, ConnectionState,
};
pub use credential::{protect_secret_dpapi, unprotect_secret_dpapi, zero_memory, CredentialStore};
pub use known_hosts::{HostKeyVerificationResult, KnownHostEntry, KnownHostsStore};
pub use multifile::{RemoteMultiFileScanner, RemoteRotatedFile};
pub use profile::{
    AuthMethod, ConnectionProfile, ConnectionProfileManager, HostKeySetting, ReconnectPolicy,
};
pub use sftp_reader::SftpLogReader;
pub use tail::{RemoteRotationType, SftpLiveTailEngine};
