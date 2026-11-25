pub mod analytics;
pub mod anomaly;
pub mod archive;
pub mod bookmark;
pub mod columnizer;
pub mod compat;
pub mod compare;
pub mod correlation;
pub mod custom_columnizer;
pub mod domain;
pub mod export;
pub mod filter;
pub mod filter_tab;
pub mod highlight;
pub mod inspector;
pub mod legacy;
pub mod memory;
pub mod merge;
pub mod multifile;
pub mod multiline;
pub mod pattern;
pub mod plugin;
pub mod reader;
pub mod remote;
pub mod search;
pub mod session;
pub mod source;
pub mod stats;
pub mod stream;
pub mod sync;
pub mod time;
pub mod tools;
pub mod trigger;
pub mod workspace;

pub use analytics::{
    AggregateField, AggregationResult, AggregationType, AnalyticsEngine, AnalyticsPreset,
    AnalyticsQuery, BucketInterval, BucketStats, CountMinSketch, EvaluationMode, FilterCondition,
    HyperLogLog, MetricRow, PresetManager, QueryOperator, SpaceSavingTopK, SparseTimeBucketer,
    TimeRangeFilter,
};
pub use anomaly::{AnomalyPoint, RollingBaselineDetector};
pub use compare::{ComparisonEngine, DatasetComparisonSummary, KeyComparison};
pub use domain::{
    CanonicalIp, EndpointAnalytics, HttpStatusAnalytics, HttpStatusDistribution, IpAnalytics,
    LatencyAnalytics, LatencyQuantiles, NormalizedEndpoint, SecurityObservation,
    SecurityPresetAnalyzer, TrustedProxyPolicy,
};
pub use export::{OfflineReportGenerator, PrivacyFilter, PrivacyMaskingOptions};
pub use pattern::{DynamicTokenizer, PatternGrouper, PatternTemplate};

pub use bookmark::{Bookmark, BookmarkStatus, GlobalBookmarkManager};
pub use columnizer::{Columnizer, ColumnizerRegistry};
pub use correlation::{
    CorrelationCandidate, CorrelationEngine, CorrelationRule, CorrelationTargetRecord, MatchReason,
};
pub use custom_columnizer::{CustomColumnizer, CustomParserType};
pub use export::{ExportFormat, StreamingExporter};
pub use filter::{
    EmptyColumnPolicy, FilterCriteria, FilterEngine, FilterMode, FilterPreset, FilterPresetManager,
};
pub use filter_tab::FilteredDocumentSource;
pub use highlight::{HighlightGroup, HighlightRule};
pub use inspector::{InspectedRecord, InspectorField, LogInspector};
pub use memory::{format_safe_preview_line, GlobalMemoryBudget};
pub use merge::{
    BoundedKWayMerger, LiveLogMergeEngine, LiveMergeMetrics, MergeCandidate, MergedLogSource,
    RecordProvenance, SourceAlias,
};
pub use multifile::MultiFileSource;
pub use multiline::{LogicalRecord, MultilineEngine};
pub use plugin::{
    DeclarativeColumnizer, ExecutablePluginHost, PluginCategory, PluginIpcMessage, PluginManifest,
    PluginRegistry, PluginTrustState,
};
pub use reader::{NonLockingLogReader, UpdateEvent};
pub use search::{SearchEngine, SearchOptions, SearchResult};
pub use session::{CrashRecoveryTracker, MissingSourceAction, SessionDocument, SessionState};
pub use source::{allocate_source_id, LogSource, MockSource, SingleFileSource, SourceLineRef};
pub use stats::{EventSeverity, EventStatisticsCollector, EventTimelineModel, TimeBucketDuration};
pub use sync::{
    SplitViewOrientation, SplitViewState, SyncGroup, SyncLoopGuard, SyncManager, SyncMember,
    SyncMemberStatus, SyncOriginToken,
};
pub use time::{
    CalibrationResult, DeltaCalculator, DeltaUnit, LocatorResult, MatchMode, ParseStatus,
    SparseTimestampCheckpoint, SparseTimestampIndex, TimePrecision, TimeSpreadBucket,
    TimeSpreadModel, TimeshiftEngine, TimestampLocator, TimestampModel, TimestampParserRegistry,
};
pub use tools::{
    CapturedOutputSource, ExternalToolConfig, ProcessJobContainer, SafeProcessLauncher,
    ToolPlaceholderContext,
};
pub use trigger::{
    ActionHistoryEntry, BoundedActionHistory, BoundedActionQueue, GlobalTriggerDeduplicator,
    KeywordTriggerRule, QueuedAction, TriggerAction, TriggerEngine, TriggerExecutionKey,
};
pub use compat::{
    enforce_safe_dll_search_order, fill_secure_random, verify_delay_load_runtime_safety,
    Win32SyncEvent,
};
pub use workspace::{Document, Workspace};

pub use archive::{
    check_zip_bomb, decompress_gzip, inflate_deflate, validate_safe_zip_entry_path, GzipLogReader,
    TempFileGuard, ZipArchiveReader, ZipMember,
};
pub use legacy::{
    LegacyExternalToolDef, LegacyFilterPreset, LegacyHighlightRule, LegacyImportSummary,
    LegacySessionImporter, SettingsMigrationWizard,
};
pub use plugin::{DataSourceIpcMessage, MockDataSourcePlugin};
pub use remote::{
    calculate_backoff_delay, protect_secret_dpapi, unprotect_secret_dpapi, zero_memory, AuthMethod,
    BoundedConnectionPool, CancellationToken, ConnectionProfile, ConnectionProfileManager,
    ConnectionState, CredentialStore, HostKeySetting, HostKeyVerificationResult, KnownHostEntry,
    KnownHostsStore, MockRemoteTransport, ReconnectPolicy, RemoteDirEntry, RemoteFileStat,
    RemoteMultiFileScanner, RemoteRotatedFile, RemoteRotationType, RemoteTransport,
    SftpFileBrowser, SftpLiveTailEngine, SftpLogReader, SortColumn,
};
pub use source::{SourceCapabilities, SourceIdentity, SourceStatus, SourceType};
pub use stream::{
    validate_safe_argument, BoundedStreamStorage, DockerLogParser, DockerStreamRecord,
    DockerStreamType, JournaldJsonParser, JournaldRecord, StreamTemplate,
};
