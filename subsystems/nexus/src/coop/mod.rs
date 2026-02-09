//! # Kernel-App Cooperation Protocol — Year 4 SYMBIOSIS (Q3 2029)
//!
//! Bidirectional communication protocol between kernel and userland
//! applications. Applications can give hints to the kernel, and the kernel
//! can send advisories back to applications.
//!
//! ## Key Innovations
//!
//! - **Bidirectional Hints**: Apps → Kernel hints, Kernel → App advisories
//! - **Resource Negotiation**: Dynamic resource contracts between kernel and apps
//! - **Cooperative Scheduling**: App and kernel collaborate on scheduling decisions
//! - **Feedback Loop**: Continuous improvement from cooperation telemetry
//!
//! ## Submodules
//!
//! - `protocol`: Core protocol definitions and message types
//! - `hints`: Bidirectional hint system (app→kernel, kernel→app)
//! - `negotiate`: Resource negotiation and contract management
//! - `feedback`: Cooperation telemetry and feedback loops

#![allow(dead_code)]

extern crate alloc;

pub mod advisory;
pub mod arbitrate;
pub mod barrier;
pub mod budget;
pub mod channel;
pub mod compliance;
pub mod consensus;
pub mod contract;
pub mod deadline;
pub mod events;
pub mod exchange;
pub mod fairshare;
pub mod feedback;
pub mod gossip;
pub mod handshake;
pub mod hints;
pub mod learning;
pub mod negotiate;
pub mod pledge;
pub mod protocol;
pub mod quota_coop;
pub mod registry;
pub mod reputation;
pub mod rewards;
pub mod session;
pub mod timeline;
pub mod trust;
pub mod witness;
// Round 4
pub mod auction;
pub mod checkpoint;
pub mod coalition;
pub mod delegation;
pub mod donation;
pub mod lease;
pub mod mediation;
pub mod throttle_coop;
pub mod voting;
// Round 5
pub mod discovery;
pub mod election;
pub mod escrow;
pub mod healthcheck;
pub mod migration_coop;
pub mod notification;
pub mod priority_coop;
pub mod ratelimit_coop;
pub mod reservation;
pub mod snapshot_coop;
// Round 6
pub mod backpressure;
pub mod cap_exchange;
pub mod dep_tracker;
pub mod fairness_monitor;
pub mod governance;
pub mod intent;
pub mod sla;
pub mod watchdog_proto;
// Round 7
pub mod attestation;
pub mod bandwidth;
pub mod capability_proto;
pub mod embargo;
pub mod fence_proto;
pub mod partition;
pub mod quorum;
pub mod service_mesh;
pub mod token_ring;
// Round 8
pub mod dlm;
pub mod event_bus;
pub mod group_sched;
pub mod health_monitor;
pub mod leader_election;
pub mod load_shed;
pub mod pi_protocol;
pub mod rate_limiter;
pub mod service_registry;
pub mod snapshot_sync;
pub mod work_steal;
// Round 9
pub mod broadcast_mgr;
pub mod clock_sync;
pub mod conflict_resolver;
pub mod membership_mgr;
pub mod quorum_tracker;
pub mod raft_engine;
pub mod saga_coord;
pub mod state_transfer;
pub mod task_graph;
pub mod txn_log;

// Round 10
pub mod causal_order;
pub mod circuit_breaker;
pub mod consensus_log;
pub mod consistent_hash;
pub mod failure_detector;
pub mod flow_control;
pub mod merkle_tree;
pub mod priority_queue;
pub mod slot_alloc;
pub mod svc_discovery;
pub mod versioned_state;

// Round 11
pub mod bloom_filter;
pub mod crdt_engine;
pub mod epoch_mgr;
pub mod id_gen;
pub mod log_replicator;
pub mod retry_policy;
pub mod ring_buffer;
pub mod sharding;
pub mod vector_clock;
pub mod write_ahead_log;

// Round 12
pub mod broadcast;
pub mod coop_alloc;
pub mod epoch_barrier;
pub mod fair_lock;
pub mod hazard_ptr;
pub mod mpsc_queue;
pub mod seqlock;
pub mod skip_list;

// Round 13
pub mod adaptive_lock;
pub mod async_queue;
pub mod batch_sync;
pub mod consensus_mgr;
pub mod counter_set;
pub mod event_sink;
pub mod futex_mgr;
pub mod lease_mgr;
pub mod prio_arbiter;
pub mod task_steal;
pub mod throttle_gate;

// Round 14
pub mod credit_flow;
pub mod fair_share;
pub mod gossip_proto;
pub mod heartbeat_mgr;
pub mod join_handle;
pub mod merge_sort;
pub mod permit_pool;
pub mod rendezvous;
pub mod split_lock;

// Round 15
pub mod barrier_pool;
pub mod claim_mgr;
pub mod condvar_mgr;
pub mod deadline_sched;
pub mod dependency_graph;
pub mod quorum_mgr;
pub mod work_stealing;

// Round 16
pub mod broadcast_chan;
pub mod latch_mgr;
pub mod mpmc_queue;
pub mod once_cell;
pub mod park_mgr;
pub mod priority_inherit;
pub mod seq_lock;
pub mod ticket_lock;
pub mod wait_group;
// Round 17
pub mod async_barrier;
pub mod epoch_gc;
pub mod fair_sched;
pub mod lock_free_list;
pub mod phase_barrier;
pub mod rcu_sync;
// Round 18
pub mod backoff;
pub mod bounded_queue;
pub mod countdown;
pub mod futex;
pub mod latch;
// Round 19
pub mod async_oneshot;
pub mod bitlock;
pub mod convoy;
pub mod fair_mutex;
pub mod intrusive_list;
pub mod mpmc_channel;
pub mod park;
pub mod spin_barrier;
// Round 20
pub mod read_indicator;
pub mod rwlock;
pub mod sequence_lock;
// Round 21
pub mod clh_lock;
pub mod lockdep;
pub mod mcs_lock;
pub mod rcu_reader;
pub mod spinlock;
// Round 22
pub mod dual_queue;
pub mod elim_stack;
pub mod exchanger;
pub mod flat_combine;
pub mod michael_scott_queue;
pub mod skiplist;
pub mod treiber_stack;
pub mod wait_free;
// Round 23
pub mod tcp_coop;
pub mod udp_coop;
pub mod route_coop;
pub mod arp_coop;
pub mod firewall_coop;
pub mod socket_coop;
pub mod netdev_coop;
pub mod qos_coop;
pub mod packet_coop;
pub mod dns_coop;
pub mod netns_coop;
// Round 24
pub mod bio_coop;
pub mod blkdev_coop;
pub mod dentry_coop;
pub mod devmapper_coop;
pub mod flock_coop;
pub mod inode_coop;
pub mod iosched_coop;
pub mod mount_coop;
pub mod raid_coop;
pub mod superblock_coop;
pub mod vfs_coop;
// Round 25 — Security cooperation modules
pub mod apparmor_coop;
pub mod audit_coop;
pub mod capability_coop;
pub mod credential_coop;
pub mod crypto_coop;
pub mod integrity_coop;
pub mod keyring_coop;
pub mod landlock_coop;
pub mod lsm_coop;
pub mod seccomp_coop;
pub mod selinux_coop;

// Round 26 — IPC/signal cooperative coordination
pub mod eventfd_coop;
pub mod futex_coop;
pub mod mqueue_coop;
pub mod msgqueue_coop;
pub mod notify_coop;
pub mod pipe_coop;
pub mod semaphore_coop;
pub mod shm_coop;
pub mod sighand_coop;
pub mod signal_coop;
pub mod timerfd_coop;

// Round 27 — Networking/socket cooperative coordination
pub mod backlog_coop;
pub mod buffer_pool_coop;
pub mod congestion_coop;
pub mod connection_coop;
pub mod epoll_coop;
pub mod netfilter_coop;
pub mod routing_coop;
pub mod zerocopy_coop;

// Round 28 — Filesystem/VFS cooperation modules
pub mod extent_coop;
pub mod journal_coop;
pub mod page_cache_coop;
pub mod writeback_coop;

// Round 29 — Process/thread cooperation modules
pub mod clone_coop;
pub mod exec_coop;
pub mod exit_coop;
pub mod fork_coop;
pub mod nice_coop;
pub mod pgid_coop;
pub mod pid_coop;
pub mod prctl_coop;
pub mod session_coop;
pub mod thread_coop;
pub mod wait_coop;

// Re-export core types
// Re-export new module types
pub use advisory::{Advisory, AdvisoryCategory, AdvisoryEngine, AdvisoryType, AdvisoryUrgency};
// Round 9 re-exports
    AntiEntropyV2Stats, ConvergenceMetric, CoopAntiEntropyV2, Divergence, MerkleNode,
    PeerEntropyState, RepairMode, RepairTask, RepairTaskState,
};
// Round 3 re-exports
pub use arbitrate::{
    CoopArbitrationManager, CoopArbitrationStats, Dispute, DisputeCategory, DisputeEvidence,
    DisputeSeverity, DisputeState, EvidenceType, Resolution, ResolutionType,
    ResourceAdjustment as ArbitrationAdjustment,
};
// Round 7 re-exports
pub use attestation::{
    AttestationQuote, AttestationState, CoopAttestationProtocol, CoopAttestationStats, PcrRegister,
    ProcessAttestation,
};
// Round 4 re-exports
pub use auction::{
    Auction, AuctionBudget, AuctionResource, AuctionState, AuctionType, Bid, CoopAuctionManager,
    CoopAuctionStats,
};
// Round 6 re-exports
pub use backpressure::{
    BackpressureLevel, BackpressureSignalMsg, CoopBackpressureProtocol, CoopBackpressureStats,
    FlowControlState, PressureAction, PressureChain, PressureSource,
};
pub use bandwidth::{
    BandwidthBucket, BandwidthResource, BandwidthTransfer, CoopBandwidthAllocator,
    CoopBandwidthStats, ProcessBandwidth,
};
// Round 2 re-exports
pub use barrier::{
    BarrierConfig, BarrierInstance, BarrierManagerStats, BarrierState, BarrierSummary, BarrierType,
    CoopBarrierManager,
};
pub use broadcast_mgr::{
    BcastGroup, BcastMessage, BcastMsgState, BcastSequencer, BcastStats, BroadcastReliability,
    CoopBroadcastMgr, DeliveryQueue,
};
pub use budget::{
    BudgetForecast, BudgetGroup, BudgetLoan, BudgetManagerStats, BudgetResource, BudgetState,
    CoopBudgetManager, ProcessBudget, ResourceBudgetEntry,
};
pub use cap_exchange::{
    CapExState, CapRight, CapScope, CapToken, CapTransfer, CoopCapExchange, CoopCapExchangeStats,
};
pub use capability_proto::{
    CapObjectType, CapRight as CapProtoRight, CapabilityToken, CoopCapProtocol,
    CoopCapProtocolStats, ProcessCapTable,
};
pub use channel::{
    Channel, ChannelDirection, ChannelError, ChannelId, ChannelManager, ChannelMessage,
    ChannelPriority, ChannelState, ChannelStats, InlinePayload, MessagePayload,
};
pub use checkpoint::{
    CheckpointSchedule, CheckpointState, CheckpointType, CoopCheckpointManager,
    CoopCheckpointStats, CoordinatedCheckpoint, ParticipantState, ProcessCheckpoint,
};
pub use clock_sync::{
    ClockSample, ClockSyncStats, CoopClockSync, HlcTimestamp, PeerClockState, SkewAlert,
};
pub use coalition::{
    Coalition, CoalitionMember, CoalitionPurpose, CoalitionState, CoopCoalitionManager,
    CoopCoalitionStats, MemberRole,
};
pub use compliance::{
    ComplianceLevel, ComplianceMonitor, ComplianceResult, GracePeriod, PenaltyAction,
    PenaltySchedule, SlaBound, SlaMetric, Violation,
};
pub use conflict_resolver::{
    Conflict, ConflictParty, ConflictResource, ConflictSeverity, ConflictState,
    CoopConflictResolver, ResolutionStrategy, ResolverStats,
};
pub use consensus::{
    ConsensusAlgorithm, CoopConsensusManager, CoopConsensusStats, Proposal, ProposalState,
    ProposalType, VoteRecord, VoteType,
};
    ConsensusProto, CoopConsensusV2, CoopConsensusV2Stats, LogEntry, RaftNode, RaftState,
    TwoPhaseState, TwoPhaseTransaction,
};
    AppendRequest, ConsensusNode, ConsensusRole, CoopConsensusV3, CoopConsensusV3Stats,
    LogEntry as ConsensusV3LogEntry, VoteRequest, VoteResponse,
};
pub use contract::{
    BreachParty, BreachSeverity, Contract as CoopContract, ContractBreach, ContractManagerStats,
    ContractState as CoopContractState, ContractTerm, ContractType, CoopContractManager,
    NegotiationOffer, NegotiationResponse, TermMeasurement, TermType, TermUnit,
};
pub use deadline::{
    AdmissionRejection, AdmissionResult, CoopDeadlineManager, DeadlineClass, DeadlineManagerStats,
    DeadlineMiss, DeadlineTask, DeadlineUrgency, MissCause, ReclaimableSlack,
};
pub use delegation::{
    CoopDelegationManager, CoopDelegationStats, Delegation, DelegationChain, DelegationConstraint,
    DelegationState, DelegationType,
};
pub use dep_tracker::{
    CoopDepEdge, CoopDepGraph, CoopDepState, CoopDepStrength, CoopDepTracker, CoopDepTrackerStats,
    CoopDepType,
};
// Round 5 re-exports
pub use discovery::{
    CoopDiscoveryManager, CoopDiscoveryStats, DiscoveryQuery, DiscoveryResult, LoadBalanceStrategy,
    ServiceInstance, ServiceState as CoopServiceState, ServiceType,
};
// Round 8 re-exports
pub use dlm::{
    CoopDlm, DlmLockRequest, DlmLockState, DlmLockType, DlmResource, LockCompat, WaitForEdge,
};
pub use donation::{
    CoopDonationManager, CoopDonationStats, DonationPriority, DonationPriorityState,
    DonationReason, PriorityCeiling, PriorityDonation,
};
pub use election::{
    CoopElectionManager, CoopElectionStats, Election, ElectionAlgorithm, ElectionNode,
    ElectionState, NodeRole,
};
pub use embargo::{
    CoopEmbargoEngine, CoopEmbargoStats, Embargo, EmbargoCheckResult, EmbargoRequest,
    EmbargoResponse, EmbargoTarget, EmbargoType,
};
pub use escrow::{
    CoopEscrowManager, CoopEscrowStats, DisputeResolution as EscrowDisputeResolution,
    EscrowCondition, EscrowContract, EscrowResource, EscrowResourceType, EscrowState,
};
pub use event_bus::{
    BusEvent, CoopEventBus, DeadLetter, DeliveryStatus, EventBusPriority, EventTopic,
    Subscriber as EventBusSubscriber, SubscriberFilter,
};
pub use events::{CoopEvent, CoopEventBus, EventCategory, EventFilter, EventType, SubscriptionId};
pub use exchange::{
    CoopExchangeManager, CoopExchangeStats, ExchangeOrder, ExchangeResource, MarketStats,
    OrderBook, OrderSide, OrderState, Trade,
};
pub use fairness_monitor::{
    CoopFairnessMonitor, CoopFairnessStats, FairnessResource, FairnessVerdict, ResourceFairness,
    StarvationAlert,
};
pub use fairshare::{
    CoopFairShareScheduler, FairShareEntity, FairShareStats, FairnessMetrics, ShareWeight,
    VirtualTime,
};
pub use feedback::{CoopFeedback, CoopMetrics, FeedbackCollector, FeedbackType};
pub use fence_proto::{
    BarrierGroup as FenceBarrierGroup, CoopFenceProtoStats, CoopFenceProtocol, EpochState,
    FenceType, GracePeriod as FenceGracePeriod, ThreadEpochInfo,
};
pub use gossip::{
    CoopGossipManager, GossipConfig, GossipMessage, GossipMessageType, GossipNode, GossipStats,
    NodeHealth, VectorClock,
};
pub use governance::{
    ConditionOp, CoopGovernanceEngine, CoopGovernanceStats, PolicyAction, PolicyCondition,
    PolicyPriority, PolicyRule, PolicyScope, PolicySet, TenantBoundary,
};
pub use group_sched::{
    CoopGroupSched, GroupMember, GroupMemberState, GroupSchedPolicyV2,
    SchedGroupV2 as CoopSchedGroup,
};
pub use handshake::{
    Capability, CapabilitySet, CoopHandshakeManager, HandshakeManagerStats, HandshakeSession,
    HandshakeState, HelloMessage, HelloResponse, PerfHintType, PerformanceHint,
    ProtocolVersion as HandshakeProtocolVersion,
};
pub use health_monitor::{
    CascadeDetector, CoopHealthMonitor, FailureDomain, HealthCheck as CoopHealthCheck,
    HealthStatus as CoopHealthStatus, Heartbeat as CoopHeartbeat, MonitoredComponent,
    RecoveryActionCoop,
};
pub use healthcheck::{
    CascadeDetector, CascadeEvent, CoopHealthCheckManager, CoopHealthCheckStats, CoopHealthStatus,
    ProbeConfig, ProbeResult, ProbeState, ProbeType, ProcessHealthState,
};
pub use hints::{AppHint, AppHintType, HintBus, KernelAdvisory, KernelAdvisoryType};
pub use intent::{
    CoopIntentEngine, CoopIntentStats, IntentCategory, IntentConflict, IntentDeclaration,
    IntentPriority, IntentRequirement, IntentState,
};
pub use leader_election::{
    CoopLeaderElection, Election, ElectionNode, ElectionRole, ElectionState, LeaderLease,
    VoteResponse,
};
pub use learning::{
    CoopLearningEngine, Feature, FeatureVector, LearningConfig, LearningStats, QTable,
    RewardComponent, RewardSignal, SchedulingAction as LearningAction,
};
pub use lease::{
    CoopLeaseManager, CoopLeaseStats, Lease, LeasePriority, LeaseRequest, LeaseResource, LeaseState,
};
pub use load_shed::{
    ClassPolicy, CoopLoadShedder, RequestClass, ShedDecision, ShedLevel, SubsystemShedState,
};
pub use mediation::{
    Conflict, ConflictResource, ConflictSeverity, CoopMediationManager, CoopMediationStats,
    FairnessRecord, MediationPolicy, ResolutionState, ResolutionStrategy,
};
pub use membership_mgr::{
    CoopMembershipMgr, JoinRequest, MemberDesc, MemberRole, MemberStatus, MembershipEvent,
    MembershipEventKind, MembershipStats, MembershipView,
};
pub use migration_coop::{
    CoopMigrationCoordinator, CoopMigrationReason, CoopMigrationState, CoopMigrationStats,
    MigrationPlan, MigrationTarget as CoopMigrationTarget, StateItemType, StateTransferItem,
};
pub use negotiate::{
    Contract, ContractId, ContractState, NegotiationEngine, ResourceDemand, ResourceOffer,
};
pub use notification::{
    CoopNotificationManager, CoopNotificationStats, DeliveryGuarantee, Notification,
    NotificationPriority, NotificationState, Subscription, SubscriptionFilter, Topic,
};
pub use partition::{
    CoopPartitionManager, CoopPartitionStats, Partition, PartitionBorrow, PartitionOp,
    PartitionResource,
};
pub use pi_protocol::{
    BoostReasonCoop, CoopPiProtocol, CoopPiProtocolStats, PiProtocol, PiResource,
    PriorityBoost as CoopPriorityBoost, TaskPriorityState,
};
pub use pledge::{
    CoopPledge, CoopPledgeManager, CoopPledgeStats, PledgeDirection, PledgeReliability,
    PledgeResource, PledgeState,
};
pub use priority_coop::{
    ActiveBoost, CoopBoostReason, CoopPriorityClass, CoopPriorityEngine, CoopPriorityStats,
    NegotiationAction, NegotiationRequest, NegotiationResult, ProcessPriority,
};
pub use protocol::{
    CoopCapability, CoopMessage, CoopMessageType, CoopSession, CoopSessionState, ProtocolVersion,
};
pub use quorum::{
    CoopQuorumProtocol, CoopQuorumStats, Proposal as QuorumProposal, QuorumGroup, QuorumMember,
    QuorumPolicy, QuorumState, Vote as QuorumVote, VoteValue as QuorumVoteValue,
};
pub use quorum_tracker::{
    Ballot, CoopQuorumTracker, JointQuorum, QuorumPolicy, QuorumStats, QuorumVote, VoteValue,
};
pub use quota_coop::{
    CoopQuotaManager, CoopQuotaPool, CoopQuotaResource, CoopQuotaState, CoopQuotaStats, QuotaEntry,
    QuotaLoan,
};
pub use raft_engine::{CoopRaftEngine, CoopRaftStats, RaftLogEntry, RaftNode, RaftPeer, RaftRole};
pub use rate_limiter::{
    ConsumerRateState, CoopRateLimiter, RateDecision, RateLimitAlgorithm, SlidingWindowCounter,
    TokenBucket,
};
pub use ratelimit_coop::{
    CoopRateLimitManager, CoopRateLimitStats, CoopSlidingWindow,
    CoopTokenBucket as RateLimitTokenBucket, ProcessRateState, RateLimitAlgorithm, RateLimitConfig,
    RateLimitDecision, RateLimitGroup, RateLimitScope,
};
pub use registry::{
    CapabilityCategory, CapabilityRegistry, CapabilityStatus, RegisteredCapability,
    ServiceDescriptor, ServiceDirectory,
};
pub use reputation::{
    CoopReputationManager, CoopReputationStats, DimensionScore, ProcessReputation,
    ReputationDimension, ReputationLevel,
};
pub use reservation::{
    CoopReservationManager, CoopReservationStats, GrantedAllocation, ReservableResource,
    Reservation, ReservationPriority, ReservationState as CoopReservationState, ResourceCapacity,
    ResourceRequest,
};
    CoopResourcePoolV2, CoopResourcePoolV2Stats, PoolObject, PoolObjectState, PoolSlab, PoolTier,
};
pub use rewards::{
    CoopLevel, PenaltyReason, RewardConfig, RewardEngine, RewardReason, ScoreEvent, ScoreSnapshot,
};
pub use saga_coord::{
    CoopSagaCoord, RecoveryMode, Saga, SagaLogEntry, SagaLogEvent, SagaStats, SagaStatus, SagaStep,
    StepStatus,
};
pub use service_mesh::{
    CircuitState, CoopServiceMesh, CoopServiceMeshStats, LoadBalanceStrategy as MeshLoadBalance,
    Service, ServiceInstance as MeshServiceInstance, ServiceState as MeshServiceState,
};
pub use service_registry::{
    CoopServiceRegistry, DependencyStatus, LookupResult, ServiceCapability, ServiceEndpoint,
    ServiceEvent, ServiceEventType, ServiceHealth,
};
pub use session::{
    Session, SessionCapabilities, SessionGroup, SessionId, SessionManager, SessionState,
};
pub use sla::{
    CoopSlaEngine, CoopSlaStats, ErrorBudget, SlaContract, SlaTier, SloDefinition, SloMetric,
    SloStatus,
};
pub use snapshot_coop::{
    CoopSnapshot, CoopSnapshotManager, CoopSnapshotStats, CoordinatedSnapshot, DiffType,
    SnapshotDataType, SnapshotDiff, SnapshotEntry, SnapshotState as CoopSnapshotState,
};
pub use snapshot_sync::{
    ChannelRecording, ChannelTracker, ComponentSnapshot, ConsistentSnapshot, CoopSnapshotSync,
    RecordedMessage, SnapshotDiff, SnapshotStateCoop, SnapshotType,
};
pub use state_transfer::{
    CoopStateTransfer, SnapshotDesc, TransferChunk, TransferMode, TransferSession, TransferState,
    TransferStats,
};
pub use task_graph::{
    CoopTaskGraph, CriticalPathSegment, ExecLevel, FailurePolicy, GraphStats, GraphTask,
    GraphTaskPriority, GraphTaskStatus,
};
    AdaptiveSampler, CoopTelemetryV2, CoopTelemetryV2Stats, MetricAggregation, MetricKind,
    MetricPoint, SpanStatus, TraceContext, TraceSpan,
};
pub use throttle_coop::{
    BackpressureSignal, CoopThrottleManager, CoopThrottleStats, CoopTokenBucket,
    ProcessThrottleState, ThrottleConfig, ThrottleResource, ThrottleState,
};
pub use timeline::{
    CoopTimelineManager, CoopTimelineStats, ReservationState as TimelineReservationState,
    ReservationType, ResourceTimeline, TimeSlot, TimelineConflict, TimelineReservation,
    TimelineResource,
};
pub use token_ring::{
    CoopTokenRing, CoopTokenRingStats, RingNode, RingNodeState, Token, TokenState,
};
pub use trust::{TrustDimension, TrustEvidence, TrustLevel, TrustManager, TrustSnapshot};
pub use txn_log::{
    CoopTxnLog, GroupCommitBatch, LogRecord, LogRecordType, LogSegment, RecoveryAction,
    RecoveryActionType, TxnCheckpoint, TxnLogStats, TxnMeta, TxnState,
};
pub use voting::{
    Ballot, BallotState, CoopVotingManager, CoopVotingStats, VoteType as CoopVoteType, VoteValue,
};
pub use watchdog_proto::{
    CoopWatchdogProtoStats, CoopWatchdogProtocol, EscalationLevel, LivenessState, PhiDetector,
    WatchdogRecovery, WatchedProcess,
};
pub use witness::{
    AgreementRecord, Attestation, AttestationType, CoopWitnessManager, CoopWitnessStats, Witness,
    WitnessRole, WitnessStatus,
};
pub use work_steal::{
    CoopWorkStealer, StealStrategyCoop, StealWorker, WorkItem, WorkerDeque, WorkerStateCoop,
    WorkerStats as StealWorkerStats,
};

// Round 10 re-exports
pub use causal_order::{
    CausalBarrier, CausalEvent, CausalOrderStats, CoopCausalOrder, DeliveryOrder,
    NodeCausalState, VectorClock,
};
pub use circuit_breaker::{
    Bulkhead, CircuitBreaker, CircuitBreakerStats, CircuitConfig, CircuitState,
    CoopCircuitBreaker, FailureType as CircuitFailureType, WindowEntry,
};
pub use consensus_log::{
    CoopConsensusLog, ConsensusLogStats, LogEntry, LogEntryState, LogEntryType,
    ReplicationProgress, SnapshotMeta as ConsensusSnapshotMeta,
};
pub use consistent_hash::{
    ConsistentHashStats, CoopConsistentHash, ItemPlacement, RebalanceEvent,
    RebalanceReason, RingNode, VirtualNode,
};
pub use failure_detector::{
    CoopFailureDetector, DetectionMethod, FailureDetectorStats,
    HealthAssessment, HeartbeatWindow, NodeDetectorState, PartitionDetector,
};
pub use flow_control::{
    CongestionSignal, CongestionWindow, CoopFlowControl, CreditCounter,
    FlowChannel, FlowControlStats, FlowState,
};
    CoopGossipV2, DigestEntry, GossipMsgType, GossipPeer, GossipV2Stats,
    LivenessState, Rumor,
};
    CoopLeaseMgrV2, LeaseEvent, LeaseEventType, LeaseMgrStats,
    LeaseRecord, LeaseRequest, LeaseState as LeaseMgrState, LeaseType as LeaseMgrType,
};
pub use merkle_tree::{
    CoopMerkleTree, DiffType as MerkleDiffType, MerkleDiff, MerkleNode as MerkleTreeNode,
    MerkleProof, MerkleTreeStats, ProofStep,
};
pub use priority_queue::{
    CoopPriorityQueue, NodePartition, PriorityBucket, PriorityQueueStats,
    QueueItem, QueueItemState, TaskUrgency,
};
pub use slot_alloc::{
    CoopSlotAllocator, NodeAllocation, SlotAllocStats, SlotFrame,
    SlotPriority, SlotState, TimeSlot,
};
pub use svc_discovery::{
    CoopServiceDiscovery, LbStrategy, ServiceDepEdge, ServiceDescriptor,
    ServiceDiscoveryStats, ServiceEndpoint, ServiceState as SvcState,
};
pub use versioned_state::{
    CoopVersionedState, MvccStats, MvccTransaction, MvccTxnState,
    Snapshot as MvccSnapshot, VersionChain, VersionRecord, VersionVisibility,
};

// Round 11 re-exports
pub use bloom_filter::{
    BloomFilter, BloomFilterStats, CoopBloomFilter, CountingBloomFilter,
    ScalableBloomFilter,
};
pub use crdt_engine::{
    CoopCrdtEngine, CrdtEngineStats, CrdtType, GCounter, GSet, LWWRegister,
    ORSet, PNCounter,
};
pub use epoch_mgr::{
    AdvanceResult, CleanupType, CoopEpochMgr, DeferredCleanup, EpochMgrStats,
    EpochParticipant, ParticipantState as EpochParticipantState,
};
    ArrivalWindow, CoopHeartbeatV2, HeartbeatStatus, HeartbeatV2Stats,
    PeerHeartbeat,
};
pub use id_gen::{
    CoopIdGen, IdFormat, IdGenStats, NamespaceGenerator, NodeRegistration as IdNodeRegistration,
    SnowflakeId,
};
pub use log_replicator::{
    CompactionPolicy, CoopLogReplicator, FollowerInfo, FollowerState,
    LogEntry as ReplicatorLogEntry, LogEntryKind as ReplicatorLogEntryKind,
    LogReplicatorStats, SnapshotMeta as ReplicatorSnapshotMeta,
};
pub use retry_policy::{
    BackoffStrategy, CircuitBreaker as RetryCircuitBreaker, CircuitState as RetryCircuitState,
    CoopRetryPolicy, RetryOutcome, RetryPolicy, RetryPolicyStats, RetryState,
    StormDetector,
};
pub use ring_buffer::{
    Consumer as RingConsumer, CoopRingBuffer, OverflowPolicy, Producer as RingProducer,
    RingBufferStats, RingChannel, WatermarkEvent,
};
pub use sharding::{
    CoopSharding, HotShardConfig, Shard, ShardMigration, ShardNode, ShardState,
    ShardStrategy, ShardingStats, VirtualNode as ShardVirtualNode,
};
pub use vector_clock::{
    CausalEvent as VClockCausalEvent, CausalHistory, CausalOrder, CoopVectorClock,
    VectorClock as VClockImpl, VectorClockStats, VersionVector,
};
pub use write_ahead_log::{
    ActiveTransaction, CoopWriteAheadLog, WalCheckpoint, WalEntry,
    WalEntryType, WalSegment, WalStats,
};

// Round 12 re-exports
    ArrivalState, BarrierInstance as BarrierV2Instance, BarrierParticipant,
    BarrierType as BarrierV2Type, BarrierV2Stats, CoopBarrierV2,
};
pub use broadcast::{
    BroadcastMsg, BroadcastPriority, BroadcastStats, BroadcastTopic,
    CoopBroadcast, DeliveryGuarantee, Subscriber as BroadcastSubscriber,
};
    ChannelInstance, ChannelMsg, ChannelReceiver, ChannelSender,
    ChannelState as ChannelV2State, ChannelType as ChannelV2Type,
    ChannelV2Stats, CoopChannelV2,
};
pub use coop_alloc::{
    AllocClass, ClassBucket, CoopAlloc, CoopAllocRecord, CoopAllocStats,
    CoopPool, PoolPressure, SubsystemBudget,
};
pub use epoch_barrier::{
    CoopEpochBarrier, DeferredDrop, Epoch as EpochVal, EpochBarrierStats,
    EpochState as EpochBarrierState, ThreadEpoch,
};
pub use fair_lock::{
    CoopFairLock, FairLock, FairLockStats, FairnessPolicy,
    HoldType, LockHolder as FairLockHolder, LockWaiter as FairLockWaiter,
    WaiterState as FairWaiterState,
};
pub use hazard_ptr::{
    CoopHazardPtr, HazardDomain, HazardPtrStats, HazardSlot,
    HazardState, RetiredNode, ThreadHazardCtx,
};
pub use mpsc_queue::{
    Consumer as MpscConsumer, CoopMpscQueue, MpscInstance,
    MpscStats, MsgPriority, OverflowAction, Producer as MpscProducer,
    QueueMsg,
};
    CoopRcuV2, CpuRcuState, GracePeriod as RcuGracePeriod,
    GracePeriodState, RcuCallback, RcuFlavor, RcuV2Stats, SrcuDomain,
};
pub use seqlock::{
    CoopSeqlock, ReadOutcome, Seqcount, Seqlock,
    SeqlockState, SeqlockStats,
};
pub use skip_list::{
    CoopSkipList, SkipList, SkipListStats, SkipNode,
};

// Round 13 re-exports
pub use adaptive_lock::{
    AdaptiveLockInstance, AdaptiveLockState, AdaptiveLockStats,
    ContentionLevel, CoopAdaptiveLock, LockStrategy, SpinParams,
};
pub use async_queue::{
    AsyncQueueInstance, AsyncQueueStats, CoopAsyncQueue,
    ItemState as AsyncItemState, QueueItem as AsyncQueueItem,
    QueueOrder, QueuePriority as AsyncQueuePriority,
};
pub use batch_sync::{
    BatchGroup, BatchOpType, BatchSyncStats, CoopBatchSync,
    ParticipantState as BatchParticipantState, BatchParticipant,
};
pub use consensus_mgr::{
    ConsensusAlgorithm as ConsMgrAlgorithm, ConsensusMgrStats,
    CoopConsensusMgr, Proposal as ConsMgrProposal,
    ProposalState as ConsMgrProposalState, Vote as ConsMgrVote,
    VoterRecord,
};
pub use counter_set::{
    CoopCounterSet, Counter, CounterSet, CounterSetStats,
    CounterType, OverflowPolicy as CounterOverflowPolicy, PerCpuCounter,
};
pub use event_sink::{
    CoopEventSink, EventFilter as SinkEventFilter,
    EventSeverity as SinkEventSeverity, EventCategory as SinkEventCategory,
    EventSinkInstance, EventSinkStats, SinkEvent, SinkSubscriber,
};
pub use futex_mgr::{
    CoopFutexMgr, FutexBucket, FutexMgrStats, FutexOp,
    FutexWaitState, FutexWaiter,
};
pub use lease_mgr::{
    CoopLeaseMgr, LeaseHolder, LeaseMgrStats,
    LeaseRequest as LeaseMgrRequest, LeaseState as LeaseMgrV2State,
    LeaseType as LeaseMgrV2Type, ResourceLease, ResourceLeaseState,
};
pub use prio_arbiter::{
    ArbitrationPolicy, ArbitrationRequest, ArbitrationResult,
    Contender, CoopPrioArbiter, PrioArbiterStats, ResourceArbContext,
};
pub use task_steal::{
    CoopTaskSteal, StealPolicy, StealPriority, StealResult, StealTask,
    StealTaskState, TaskStealStats, WorkQueue,
};
pub use throttle_gate::{
    CoopThrottleGate, GateState, SlidingWindowState,
    ThrottleAlgorithm, ThrottleGateInstance, ThrottleGateStats,
    TokenBucketState as ThrottleTokenBucket,
};

// Round 14 re-exports
    BackpressureV2Stats, BpEvent, BpLevel, BpSource, BpStrategy,
    CoopBackpressureV2, FlowState as BpFlowState,
};
pub use credit_flow::{
    CoopCreditFlow, CreditEndpoint, CreditFlowState, CreditFlowStats,
    CreditGrant, CreditType,
};
pub use fair_share::{
    AllocationResult, CoopFairShare, FairShareStats, ResourceDim,
    ShareEntity, ShareEntityState, ShareType,
};
pub use gossip_proto::{
    CoopGossipProto, GossipMessage as GossipProtoMessage,
    GossipMsgType, GossipNode as GossipProtoNode,
    GossipNodeState, GossipProtoStats, GossipStyle,
};
pub use heartbeat_mgr::{
    CoopHeartbeatMgr, DetectorType, HeartbeatMgrStats,
    HeartbeatRecord, HeartbeatState, MonitoredNode,
};
pub use join_handle::{
    CoopJoinHandle, JoinGroup, JoinHandle, JoinHandleStats,
    JoinResult, JoinState,
};
    CoopLoadShedV2, LoadLevel, LoadShedV2Stats, LoadShedder,
    ShedDecision as ShedV2Decision, ShedPolicy, ShedRequest, ShedTier,
};
pub use merge_sort::{
    CoopMergeSort, MergeSortStats, MergeTask, SortOrder, SortSession,
    SortState,
};
pub use permit_pool::{
    CoopPermitPool, Permit, PermitPool, PermitPoolStats, PermitState,
    PermitType, PoolWaiter,
};
pub use rendezvous::{
    CoopRendezvous, ExchangeMode, RendezvousParticipant, RendezvousPoint,
    RendezvousState, RendezvousStats,
};
pub use split_lock::{
    AlignmentIssue, CoopSplitLock, SplitLockAction, SplitLockEvent,
    SplitLockPolicy, SplitLockStats, ThreadSplitLockState,
};
// Round 15 re-exports
pub use barrier_pool::{
    BarrierInstance as PoolBarrierInstance, BarrierParticipant,
    BarrierPoolStats, BarrierState as PoolBarrierState,
    BarrierType as PoolBarrierType, CoopBarrierPool,
};
pub use claim_mgr::{
    Claim, ClaimMgrStats, ClaimState, ClaimType, CoopClaimMgr,
};
pub use condvar_mgr::{
    CondVar, CondWaiter, CondWaitResult, CondvarMgrStats, CoopCondvarMgr,
};
pub use deadline_sched::{
    CoopDeadlineSched, DeadlineSchedStats, DlParams, DlTask, DlTaskState,
};
pub use dependency_graph::{
    CoopDependencyGraph, CycleInfo, DepEdge, DepEdgeType, DepGraphStats,
    DepNode, DepNodeType,
};
    CoopEpochMgrV2, EpochV2, EpochV2MgrStats, EpochV2Thread, GarbageEntry,
};
    CoopHazardPtrV2, HazardPointerV2, HazardPtrV2Stats, HpState,
    HpThreadV2, RetiredNodeV2,
};
pub use quorum_mgr::{
    CoopQuorumMgr, QuorumMember, QuorumMgrStats, QuorumProposal,
    QuorumType, VoteResult,
};
    CoopRingBufferV2, RingBufferV2Instance, RingBufferV2Stats,
    RingEntry, RingStateV2,
};
    CoopSemaphoreV2, SemTypeV2, SemWaiter, SemaphoreV2, SemaphoreV2Stats,
};
pub use work_stealing::{
    CoopWorkStealing, WorkStealingStats, WorkerQueue, WsTask,
    WsTaskPriority, WsTaskState,
};

// Round 16 re-exports
pub use broadcast_chan::{
    BroadcastChannel, BroadcastChanStats, CoopBroadcastChan,
    BroadcastMsg as BroadcastChanMsg,
    BroadcastState as BroadcastChanState,
    BroadcastSubscriber as BroadcastChanSubscriber,
};
    CoopFairLockV2, FairLockV2, FairLockV2Stats, McsNode, McsNodeState,
};
    CoopJoinHandleV2, JoinHandleV2, JoinHandleV2Stats,
    JoinResultV2, JoinStateV2,
};
pub use latch_mgr::{
    CoopLatchMgr, CountdownLatch, LatchMgrStats, LatchState,
};
pub use mpmc_queue::{
    CoopMpmcQueue, MpmcEntry, MpmcQueue, MpmcQueueStats, MpmcState,
};
pub use once_cell::{
    CoopOnceCell, OnceCell, OnceCellState, OnceCellStats,
};
pub use park_mgr::{
    CoopParkMgr, ParkMgrStats, ParkState, ParkedThread,
};
pub use priority_inherit::{
    CoopPriorityInherit, PiChainEntry, PiMutex,
    PriorityInheritStats, ThreadPriority,
};
pub use seq_lock::{
    CoopSeqLock, SeqLock, SeqLockStats,
};
pub use ticket_lock::{
    CoopTicketLock, TicketLock, TicketLockStats,
};
pub use wait_group::{
    CoopWaitGroup, WaitGroup, WaitGroupState, WaitGroupStats,
};
// Round 17 re-exports
pub use async_barrier::{
    AsyncBarrier, AsyncBarrierPhase, AsyncBarrierStats,
    AsyncWaiter, CoopAsyncBarrier,
};
    ConsensusV2State, ConsensusV2Stats, CoopConsensusMgrV2,
    ProposalV2, VoteV2,
};
pub use epoch_gc::{
    CoopEpochGc, DeferredItem, EpochCollector, EpochGcStats,
    EpochState as EpochGcState, ThreadEpochRecord,
};
pub use fair_sched::{
    CoopFairSched, FairSchedClass, FairSchedStats, FairTask,
};
    CoopHazardPtrV3, HazardDomainV3, HazardPtrV3Stats,
    HazardSlot as HazardSlotV3, HazardSlotState, RetiredObjV3,
};
pub use lock_free_list::{
    CoopLockFreeList, LfNode, LfNodeState, LockFreeList,
    LockFreeListStats,
};
pub use phase_barrier::{
    CoopPhaseBarrier, PhaseBarrier, PhaseBarrierStats,
    PhaseParticipant, PhaseState,
};
    CoopQuorumMgrV2, MemberStatusV2, QuorumConfigV2,
    QuorumMemberV2, QuorumMgrV2Stats, QuorumTypeV2,
};
pub use rcu_sync::{
    CoopRcuSync, GracePeriodTracker, RcuReader, RcuReaderState,
    RcuSyncCallback as RcuSyncCb, RcuSyncStats,
};
    CoopRendezvousV2, RendezvousChannelV2, RendezvousEndpointV2,
    RendezvousStateV2, RendezvousV2Stats,
};
    CoopWorkStealingV2, StealPriority as StealPriorityV2,
    StealTask as StealTaskV2, WorkStealingV2Stats,
    WorkerDeque as WorkerDequeV2,
};
// Round 18 re-exports
    AdaptiveLockEntryV2, AdaptiveLockV2Stats, ContentionLevelV2,
    CoopAdaptiveLockV2, LockStrategyV2,
};
pub use backoff::{
    BackoffState, BackoffStats, BackoffStrategy as CoopBackoffStrategy,
    CoopBackoff,
};
pub use bounded_queue::{
    BoundedQueue, BoundedQueueState, BoundedQueueStats,
    CoopBoundedQueue, QueueItem as BoundedQueueItem,
};
pub use countdown::{
    CoopCountdown, CountdownLatch as CoopCountdownLatch,
    CountdownState, CountdownStats,
};
    BusEventV2, BusSubscriberV2, CoopEventBusV2,
    EventBusPriority as EventBusV2Priority, EventBusV2Stats,
};
pub use futex::{
    CoopFutex, CoopFutexBucket, CoopFutexOp, CoopFutexStats,
    CoopFutexWaiter,
};
pub use latch::{
    CoopLatch, Latch, LatchState as CoopLatchState,
    LatchStats,
};
    CoopPriorityQueueV2, PriorityItemV2, PriorityLevelV2,
    PriorityQueueV2, PriorityQueueV2Stats,
};
    CoopRingBufferV3, RingBufferV3, RingBufferV3Stats,
    RingEntryV3, RingSlotStateV3,
};
    CoopSeqlockV2, SeqlockV2, SeqlockV2State, SeqlockV2Stats,
};
    CoopTicketLockV2, TicketLockV2, TicketLockV2State,
    TicketLockV2Stats,
};
// Round 19 re-exports
pub use async_oneshot::{
    AsyncOneshot, AsyncOneshotStats, CoopAsyncOneshot,
    OneshotState,
};
pub use bitlock::{
    Bitlock, BitlockArray, BitlockState, BitlockStats,
    CoopBitlock,
};
pub use convoy::{
    CoopConvoy, ConvoyEntry, ConvoySeverity, ConvoyStats,
};
pub use fair_mutex::{
    CoopFairMutex, FairMutex, FairMutexState,
    FairMutexStats, FairMutexWaiter,
};
pub use intrusive_list::{
    CoopIntrusiveList, IntrusiveList, IntrusiveListStats,
    IntrusiveNode, IntrusiveNodeState,
};
pub use mpmc_channel::{
    CoopMpmcChannel, MpmcChannel, MpmcChannelStats,
    MpmcMsg, MpmcState as MpmcChannelState,
};
pub use park::{
    CoopPark, ParkState as CoopParkState, ParkStats,
    ParkedThread as CoopParkedThread,
};
    CoopRendezvousV3, RendezvousV3Participant,
    RendezvousV3Point, RendezvousV3State, RendezvousV3Stats,
};
    CoopSemaphoreV3, SemV3Waiter, SemaphoreV3,
    SemaphoreV3State, SemaphoreV3Stats,
};
pub use spin_barrier::{
    BarrierPhase, CoopSpinBarrier, SpinBarrier,
    SpinBarrierStats,
};
    CoopWaitQueueV2, WaitQueueV2, WaitQueueV2Stats,
    WqV2Entry, WqV2EntryState,
};
// Round 20 re-exports
    CondvarV2Instance, CondvarV2Result, CondvarV2Stats,
    CondvarV2Waiter, CoopCondvarV2,
};
    CoopEventBusV3, EventBusV3Stats, EventV3Delivery,
    EventV3Message, EventV3Priority, EventV3Subscriber,
    EventV3Topic,
};
    CoopFutexOp, CoopFutexV2, CoopFutexV2Instance,
    CoopFutexV2Stats, CoopFutexWaiter,
};
    CoopLatchV2, LatchV2Instance, LatchV2State,
    LatchV2Stats,
};
    CoopPhaseBarrierV2, PhaseBarrierV2Instance,
    PhaseBarrierV2Stats, PhaseV2Participant, PhaseV2State,
};
    CoopPriorityLevel, CoopPriorityV2, PriorityBoostReason,
    PriorityV2CoopStats, PriorityV2Task,
};
pub use read_indicator::{
    CoopReadIndicator, ReadIndicatorInstance,
    ReadIndicatorSlot, ReadIndicatorState, ReadIndicatorStats,
};
pub use rwlock::{
    CoopRwLock, RwLockFairness, RwLockInstance,
    RwLockState, RwLockStats, RwLockWaiter,
};
pub use sequence_lock::{
    CoopSeqlock, SeqlockInstance, SeqlockState,
    SeqlockStats,
};
    CoopTicketLockV3, TicketLockV3Instance,
    TicketLockV3Stats, TicketV3State,
};
    CoopTokenRingV2, TokenRingV2Instance,
    TokenRingV2Stats, TokenV2Participant, TokenV2State,
};
// Round 21 re-exports
pub use clh_lock::{
    ClhLockInstance, ClhLockStats, ClhNode,
    ClhNodeState, CoopClhLock,
};
    CondvarV3Instance, CondvarV3MorphTarget,
    CondvarV3Stats, CondvarV3WaitResult,
    CondvarV3Waiter, CoopCondvarV3,
};
    CoopFutexV3, FutexV3Instance, FutexV3Op,
    FutexV3Result, FutexV3Stats, FutexV3Waiter,
};
pub use lockdep::{
    CoopLockdep, LockdepClass, LockdepHoldStack,
    LockdepNode, LockdepStats, LockdepViolation,
    LockdepViolationKind,
};
pub use mcs_lock::{
    CoopMcsLock, McsLockInstance, McsLockStats,
    McsNode as McsLockNode, McsNodeState as McsLockNodeState,
};
pub use rcu_reader::{
    CoopRcuReader, RcuCallback as RcuReaderCallback,
    RcuCpuState, RcuFlavor as RcuReaderFlavor,
    RcuReaderState as RcuReaderCpuState, RcuReaderStats,
};
    CoopRwLockV2, RwLockV2Fairness, RwLockV2Instance,
    RwLockV2State, RwLockV2Stats,
};
    CoopSemaphoreV4, SemaphoreV4Instance,
    SemaphoreV4Priority, SemaphoreV4Result,
    SemaphoreV4Stats, SemaphoreV4Waiter,
};
    CoopSeqlockV3, SeqlockV3Instance, SeqlockV3ReaderStats,
    SeqlockV3State, SeqlockV3Stats, SeqlockV3Variant,
};
pub use spinlock::{
    CoopSpinlock, SpinBackoff, SpinlockInstance,
    SpinlockState, SpinlockStats,
};
    CoopTicketLockV4, TicketV4BackoffMode,
    TicketV4Instance, TicketV4State, TicketV4Stats,
};
// Round 22 re-exports
pub use dual_queue::{
    CoopDualQueue, DualQueueNodeState, DualQueueNodeType,
    DualQueueState, DualQueueStats, DualQueueNode,
};
pub use elim_stack::{
    CoopElimStack, ElimArrayConfig, ElimOpType,
    ElimSlot, ElimSlotState, ElimStackStats,
};
    CoopEpochMgrV3, EpochV3Garbage, EpochV3GcPolicy,
    EpochV3State, EpochV3Stats, EpochV3ThreadState,
};
pub use exchanger::{
    CoopExchanger, ExchangeResult, ExchangeSlot,
    ExchangeSlotState, ExchangerArena, ExchangerStats,
};
pub use flat_combine::{
    CoopFlatCombine, FlatCombineOpType, FlatCombineRequest,
    FlatCombineRound, FlatCombineSlot, FlatCombineState,
    FlatCombineStats,
};
    CoopHazardPtrV4, HazardV4Domain, HazardV4Pointer,
    HazardV4RetiredNode, HazardV4State, HazardV4Stats,
    HazardV4ThreadState,
};
pub use michael_scott_queue::{
    CoopMichaelScottQueue, MsQueueNode, MsQueueOpResult,
    MsQueueState, MsQueueStats,
};
pub use skiplist::{
    CoopSkiplist, SkiplistConfig, SkiplistLevel,
    SkiplistNode, SkiplistOp, SkiplistStats,
};
pub use treiber_stack::{
    CoopTreiberStack, TreiberNode, TreiberOpResult,
    TreiberStackState, TreiberStackStats,
};
pub use wait_free::{
    CoopWaitFree, WaitFreeAnnouncement, WaitFreeOpType,
    WaitFreeProgress, WaitFreeRegister, WaitFreeStats,
    WaitFreeThreadState,
};
    CoopWorkStealV2, WorkStealV2Deque, WorkStealV2Policy,
    WorkStealV2Stats, WorkStealV2Task, WorkStealV2TaskState,
};
// Round 23 re-exports
pub use tcp_coop::{
    CoopTcp, CoopTcpCongestion, CoopTcpConnection,
    CoopTcpState, CoopTcpStats, SharedCwndState,
};
pub use udp_coop::{
    CoopUdp, CoopUdpShareMode, CoopUdpSocket,
    CoopUdpState, CoopUdpStats, SharedUdpPort,
};
pub use route_coop::{
    CoopRoute, CoopRouteEntry, CoopRouteProto,
    CoopRouteScope, CoopRouteStats, SharedRouteTable,
};
pub use arp_coop::{
    CoopArp, CoopArpEntry, CoopArpState,
    CoopArpStats, SharedArpCache,
};
pub use firewall_coop::{
    CoopFirewall, CoopFwAction, CoopFwChain,
    CoopFwMatch, CoopFwRule, CoopFwStats, SharedRuleSet,
};
pub use socket_coop::{
    CoopLbMode, CoopSocket, CoopSocketGroup,
    CoopSocketInstance, CoopSocketState, CoopSocketStats,
    CoopSocketType,
};
pub use netdev_coop::{
    CoopNetdev, CoopNetdevInstance, CoopNetdevState,
    CoopNetdevStats, CoopNetdevType, SharedNetQueue,
};
pub use qos_coop::{
    BwAllocation, CoopQos, CoopQosClass,
    CoopQosSched, CoopQosStats, CoopTokenBucket as QosTokenBucket,
    SharedQosPolicy,
};
pub use packet_coop::{
    CoopPacket, CoopPktProto, CoopPktStats,
    PktBufState, SharedBufPool, SharedPktBuf,
};
pub use dns_coop::{
    CoopDns, CoopDnsCacheState, CoopDnsStats,
    CoopDnsType, DnsQueryTracker, SharedDnsEntry,
};
pub use netns_coop::{
    CoopNetNamespace, CoopNetns, CoopNetnsState,
    CoopNetnsStats, CoopVethPair, CoopVethState,
};
pub use bio_coop::{
    CoopBio, CoopBioRequest, CoopBioState, CoopBioStats, CoopBioType,
};
pub use blkdev_coop::{
    CoopBlkdev, CoopBlkdevInstance, CoopBlkdevState, CoopBlkdevStats, CoopBwAlloc,
};
pub use dentry_coop::{
    CoopDentry, CoopDentryEntry, CoopDentryState, CoopDentryStats,
};
pub use devmapper_coop::{
    CoopDevMapper, CoopDmDevice, CoopDmState, CoopDmStats, CoopDmTarget, CoopThinPool,
};
pub use flock_coop::{
    CoopFileLock, CoopFlock, CoopFlockStats, CoopLockFairness, CoopLockState, CoopLockType,
};
pub use inode_coop::{
    CoopInode, CoopInodeEntry, CoopInodeState, CoopInodeStats,
};
pub use iosched_coop::{
    CoopIoRequest, CoopIoPrio, CoopIoSched, CoopIoSchedStats,
};
pub use mount_coop::{
    CoopMount, CoopMountProp, CoopMountState, CoopMountStats, SharedMountPoint,
};
pub use raid_coop::{
    CoopRaid, CoopRaidLevel, CoopRaidState, CoopRaidStats, CoopRebuildTask,
};
pub use superblock_coop::{
    CoopSbState, CoopSuperblock, CoopSuperblockEntry, CoopSuperblockStats,
};
pub use vfs_coop::{
    CoopVfs, CoopVfsOp, CoopVfsState, CoopVfsStats, SharedPathEntry,
};
// Re-exports from Round 25 — Security cooperation
pub use apparmor_coop::{AppArmorCoopEvent, AppArmorCoopRecord, AppArmorCoopStats, CoopAppArmor};
pub use audit_coop::{AuditCoopEvent, AuditCoopRecord, AuditCoopStats, CoopAudit};
pub use capability_coop::{CapCoopEvent, CapCoopRecord, CapCoopStats, CoopCapability};
pub use credential_coop::{CoopCredential, CredCoopEvent, CredCoopRecord, CredCoopStats};
pub use crypto_coop::{CoopCrypto, CryptoCoopEvent, CryptoCoopMode, CryptoCoopRecord, CryptoCoopStats};
pub use integrity_coop::{CoopIntegrity, IntegrityCoopEvent, IntegrityCoopRecord, IntegrityCoopStats};
pub use keyring_coop::{CoopKeyring, KeyringCoopEvent, KeyringCoopRecord, KeyringCoopScope, KeyringCoopStats};
pub use landlock_coop::{CoopLandlock, LandlockCoopEvent, LandlockCoopRecord, LandlockCoopStats};
pub use lsm_coop::{CoopLsm, LsmCoopEvent, LsmCoopPolicy, LsmCoopRecord, LsmCoopStats};
pub use seccomp_coop::{CoopSeccomp, SeccompCoopEvent, SeccompCoopRecord, SeccompCoopStats, SeccompCoopStrategy};
pub use selinux_coop::{CoopSelinux, SelinuxCoopEvent, SelinuxCoopRecord, SelinuxCoopStats};

// Round 26 re-exports — IPC/signal cooperative coordination
pub use eventfd_coop::{CoopEventfd, EventfdCoopEvent, EventfdCoopRecord, EventfdCoopStats};
pub use futex_coop::{CoopFutex as CoopFutexIpc, FutexCoopEvent, FutexCoopRecord, FutexCoopStats};
pub use mqueue_coop::{CoopMqueue, MqueueCoopEvent, MqueueCoopRecord, MqueueCoopStats};
pub use msgqueue_coop::{CoopMsgqueue, MsgqueueCoopEvent, MsgqueueCoopRecord, MsgqueueCoopStats};
pub use notify_coop::{CoopNotify, NotifyCoopEvent, NotifyCoopRecord, NotifyCoopStats};
pub use pipe_coop::{CoopPipe, PipeCoopEvent, PipeCoopRecord, PipeCoopStats};
pub use semaphore_coop::{CoopSemaphore, SemCoopEvent, SemCoopRecord, SemCoopStats};
pub use shm_coop::{CoopShm, ShmCoopEvent, ShmCoopRecord, ShmCoopStats};
pub use sighand_coop::{CoopSighand, SighandCoopEvent, SighandCoopRecord, SighandCoopStats};
pub use signal_coop::{CoopSignal, SignalCoopEvent, SignalCoopRecord, SignalCoopStats};
pub use timerfd_coop::{CoopTimerfd, TimerfdCoopEvent, TimerfdCoopRecord, TimerfdCoopStats};

// Round 27 re-exports — Networking/socket cooperative coordination
pub use backlog_coop::{BacklogCoopEvent, BacklogCoopRecord, BacklogCoopStats, CoopBacklog};
pub use buffer_pool_coop::{BufferPoolCoopEvent, BufferPoolCoopRecord, BufferPoolCoopStats, CoopBufferPool};
pub use congestion_coop::{CongestionCoopEvent, CongestionCoopRecord, CongestionCoopStats, CoopCongestion};
pub use connection_coop::{ConnectionCoopEvent, ConnectionCoopRecord, ConnectionCoopStats, CoopConnection};
pub use epoll_coop::{CoopEpoll, EpollCoopEvent, EpollCoopRecord, EpollCoopStats};
pub use netfilter_coop::{CoopNetfilter, NetfilterCoopEvent, NetfilterCoopRecord, NetfilterCoopStats};
pub use routing_coop::{CoopRouting, RoutingCoopEvent, RoutingCoopRecord, RoutingCoopStats};
pub use zerocopy_coop::{CoopZerocopy, ZerocopyCoopEvent, ZerocopyCoopRecord, ZerocopyCoopStats};

// Round 28 re-exports
pub use extent_coop::{CoopExtentEntry, CoopExtentManager, CoopExtentStats, CoopExtentType};
pub use journal_coop::{CoopJournalManager, CoopJournalStats, CoopJournalTx, CoopJournalTxType};
pub use page_cache_coop::{CoopEvictionPolicy, CoopPageCacheManager, CoopPageCacheStats, CoopPageEntry, CoopPageState};
pub use writeback_coop::{CoopWritebackEntry, CoopWritebackManager, CoopWritebackReason, CoopWritebackStats};

// Re-exports from Round 29 — Process/thread cooperation
pub use clone_coop::{CoopCloneManager, CoopCloneResult, CoopCloneSharingPolicy, CoopCloneStats};
pub use exec_coop::{CoopExecManager, CoopExecPhase, CoopExecRecord, CoopExecStats};
pub use exit_coop::{CoopExitManager, CoopExitPhase, CoopExitRecord, CoopExitStats};
pub use fork_coop::{CoopForkManager, CoopForkRecord, CoopForkStats, CoopForkStrategy};
pub use nice_coop::{CoopNiceEntry, CoopNiceManager, CoopNiceStats, CoopSchedClass};
pub use pgid_coop::{CoopPgidEntry, CoopPgidManager, CoopPgidState, CoopPgidStats};
pub use pid_coop::{CoopPidManager, CoopPidMapping, CoopPidNsEntry, CoopPidNsLevel, CoopPidStats};
pub use prctl_coop::{CoopPrctlEntry, CoopPrctlManager, CoopPrctlStats, CoopPrctlType};
pub use session_coop::{CoopSessionEntry, CoopSessionManager, CoopSessionState, CoopSessionStats};
pub use thread_coop::{CoopThreadGroup, CoopThreadLevel, CoopThreadManager, CoopThreadStats};
pub use wait_coop::{CoopWaitEntry, CoopWaitManager, CoopWaitMode, CoopWaitStats};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hint_creation() {
        let hint = AppHint::new(42, AppHintType::ComputeIntensive { duration_ms: 5000 });
        assert_eq!(hint.pid, 42);
        assert!(matches!(
            hint.hint_type,
            AppHintType::ComputeIntensive { .. }
        ));
    }

    #[test]
    fn test_advisory_creation() {
        let advisory = KernelAdvisory::new(42, KernelAdvisoryType::MemoryPressure {
            level: PressureLevel::Medium,
            recommended_release_bytes: 50 * 1024 * 1024,
        });
        assert_eq!(advisory.pid, 42);
    }

    #[test]
    fn test_hint_bus() {
        let mut bus = HintBus::new(100);

        bus.send_hint(AppHint::new(42, AppHintType::LatencySensitive {
            thread_id: 1,
        }));
        bus.send_hint(AppHint::new(43, AppHintType::IoHeavy {
            expected_bytes: 1024 * 1024,
        }));

        let hints = bus.drain_hints();
        assert_eq!(hints.len(), 2);
    }

    #[test]
    fn test_negotiation() {
        let mut engine = NegotiationEngine::new();

        let demand = ResourceDemand {
            pid: 42,
            cpu_shares: Some(200),
            memory_bytes: Some(256 * 1024 * 1024),
            io_bandwidth_bps: None,
            net_bandwidth_bps: None,
            priority: 5,
            duration_ms: Some(60_000),
        };

        let offer = engine.evaluate_demand(&demand);
        assert!(offer.is_some());
        let offer = offer.unwrap();

        let contract = engine.accept_offer(demand, offer);
        assert_eq!(contract.state, ContractState::Active);
    }

    #[test]
    fn test_feedback_collector() {
        let mut collector = FeedbackCollector::new(1000);

        collector.record(CoopFeedback {
            pid: 42,
            feedback_type: FeedbackType::HintAccuracy { accuracy: 0.92 },
            timestamp: 1000,
        });

        let metrics = collector.compute_metrics();
        assert!(metrics.avg_hint_accuracy > 0.0);
    }

    // Needed for the advisory test
    use hints::PressureLevel;
}
pub mod anti_entropy;
pub mod heartbeat;
pub mod rcu;
pub mod resource_pool;
pub mod semaphore;
pub mod telemetry;
pub mod wait_queue;
pub mod dist_lock;
// R30 — Memory Management
pub mod mmap_coop;
pub mod shmem_coop;
pub mod hugepage_coop;
pub mod mprotect_coop;
pub mod mremap_coop;
pub mod msync_coop;
pub mod munmap_coop;
pub mod vma_coop;
pub mod page_fault_coop;
pub mod oom_coop;
pub mod swap_coop;
pub mod mlock_coop;
// Consciousness — Cooperation Self-Awareness
pub mod conscious;
