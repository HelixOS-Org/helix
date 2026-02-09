//! BPF Core Types
//!
//! Fundamental types for BPF program and map management.

use alloc::vec::Vec;

/// BPF program identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BpfProgId(pub u32);

impl BpfProgId {
    /// Create a new BPF program ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// BPF map identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BpfMapId(pub u32);

impl BpfMapId {
    /// Create a new BPF map ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// BTF (BPF Type Format) identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BtfId(pub u32);

impl BtfId {
    /// Create a new BTF ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// BPF program type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BpfProgType {
    /// Unspecified
    Unspec,
    /// Socket filter
    SocketFilter,
    /// Kprobe/Kretprobe
    Kprobe,
    /// Traffic Control classifier
    SchedCls,
    /// Traffic Control action
    SchedAct,
    /// Tracepoint
    Tracepoint,
    /// XDP (eXpress Data Path)
    Xdp,
    /// Perf event
    PerfEvent,
    /// Cgroup SKB
    CgroupSkb,
    /// Cgroup socket
    CgroupSock,
    /// Lightweight tunnel
    LwtIn,
    /// Lightweight tunnel out
    LwtOut,
    /// Lightweight tunnel xmit
    LwtXmit,
    /// Socket operations
    SockOps,
    /// SK SKB
    SkSkb,
    /// Cgroup device
    CgroupDevice,
    /// SK msg
    SkMsg,
    /// Raw tracepoint
    RawTracepoint,
    /// Cgroup socket address
    CgroupSockAddr,
    /// LWT seg6local
    LwtSeg6local,
    /// lirc mode2
    LircMode2,
    /// SK reuseport
    SkReuseport,
    /// Flow dissector
    FlowDissector,
    /// Cgroup sysctl
    CgroupSysctl,
    /// Raw tracepoint writable
    RawTracepointWritable,
    /// Cgroup sockopt
    CgroupSockopt,
    /// Tracing
    Tracing,
    /// Struct ops
    StructOps,
    /// Extension
    Ext,
    /// LSM (Linux Security Module)
    Lsm,
    /// SK lookup
    SkLookup,
    /// Syscall
    Syscall,
}

impl BpfProgType {
    /// Get program type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unspec => "unspec",
            Self::SocketFilter => "socket_filter",
            Self::Kprobe => "kprobe",
            Self::SchedCls => "sched_cls",
            Self::SchedAct => "sched_act",
            Self::Tracepoint => "tracepoint",
            Self::Xdp => "xdp",
            Self::PerfEvent => "perf_event",
            Self::CgroupSkb => "cgroup_skb",
            Self::CgroupSock => "cgroup_sock",
            Self::LwtIn => "lwt_in",
            Self::LwtOut => "lwt_out",
            Self::LwtXmit => "lwt_xmit",
            Self::SockOps => "sock_ops",
            Self::SkSkb => "sk_skb",
            Self::CgroupDevice => "cgroup_device",
            Self::SkMsg => "sk_msg",
            Self::RawTracepoint => "raw_tracepoint",
            Self::CgroupSockAddr => "cgroup_sock_addr",
            Self::LwtSeg6local => "lwt_seg6local",
            Self::LircMode2 => "lirc_mode2",
            Self::SkReuseport => "sk_reuseport",
            Self::FlowDissector => "flow_dissector",
            Self::CgroupSysctl => "cgroup_sysctl",
            Self::RawTracepointWritable => "raw_tracepoint_writable",
            Self::CgroupSockopt => "cgroup_sockopt",
            Self::Tracing => "tracing",
            Self::StructOps => "struct_ops",
            Self::Ext => "ext",
            Self::Lsm => "lsm",
            Self::SkLookup => "sk_lookup",
            Self::Syscall => "syscall",
        }
    }

    /// Is networking related
    pub fn is_networking(&self) -> bool {
        matches!(
            self,
            Self::SocketFilter
                | Self::SchedCls
                | Self::SchedAct
                | Self::Xdp
                | Self::CgroupSkb
                | Self::CgroupSock
                | Self::LwtIn
                | Self::LwtOut
                | Self::LwtXmit
                | Self::SockOps
                | Self::SkSkb
                | Self::SkMsg
                | Self::SkReuseport
                | Self::FlowDissector
                | Self::SkLookup
        )
    }

    /// Is tracing related
    #[inline]
    pub fn is_tracing(&self) -> bool {
        matches!(
            self,
            Self::Kprobe
                | Self::Tracepoint
                | Self::PerfEvent
                | Self::RawTracepoint
                | Self::RawTracepointWritable
                | Self::Tracing
        )
    }

    /// Is cgroup related
    #[inline]
    pub fn is_cgroup(&self) -> bool {
        matches!(
            self,
            Self::CgroupSkb
                | Self::CgroupSock
                | Self::CgroupDevice
                | Self::CgroupSockAddr
                | Self::CgroupSysctl
                | Self::CgroupSockopt
        )
    }
}

/// BPF map type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BpfMapType {
    /// Unspecified
    Unspec,
    /// Hash map
    Hash,
    /// Array map
    Array,
    /// Program array
    ProgArray,
    /// Perf event array
    PerfEventArray,
    /// Per-CPU hash
    PercpuHash,
    /// Per-CPU array
    PercpuArray,
    /// Stack trace
    StackTrace,
    /// Cgroup array
    CgroupArray,
    /// LRU hash
    LruHash,
    /// LRU per-CPU hash
    LruPercpuHash,
    /// LPM trie
    LpmTrie,
    /// Array of maps
    ArrayOfMaps,
    /// Hash of maps
    HashOfMaps,
    /// Devmap
    Devmap,
    /// Sockmap
    Sockmap,
    /// Cpumap
    Cpumap,
    /// Xskmap
    Xskmap,
    /// Sockhash
    Sockhash,
    /// Cgroup storage
    CgroupStorage,
    /// Reuseport sockarray
    ReuseportSockarray,
    /// Per-CPU cgroup storage
    PercpuCgroupStorage,
    /// Queue
    Queue,
    /// Stack
    Stack,
    /// SK storage
    SkStorage,
    /// Devmap hash
    DevmapHash,
    /// Struct ops
    StructOps,
    /// Ring buffer
    Ringbuf,
    /// Inode storage
    InodeStorage,
    /// Task storage
    TaskStorage,
    /// Bloom filter
    BloomFilter,
    /// User ringbuf
    UserRingbuf,
    /// Cgrp storage
    CgrpStorage,
}

impl BpfMapType {
    /// Get map type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unspec => "unspec",
            Self::Hash => "hash",
            Self::Array => "array",
            Self::ProgArray => "prog_array",
            Self::PerfEventArray => "perf_event_array",
            Self::PercpuHash => "percpu_hash",
            Self::PercpuArray => "percpu_array",
            Self::StackTrace => "stack_trace",
            Self::CgroupArray => "cgroup_array",
            Self::LruHash => "lru_hash",
            Self::LruPercpuHash => "lru_percpu_hash",
            Self::LpmTrie => "lpm_trie",
            Self::ArrayOfMaps => "array_of_maps",
            Self::HashOfMaps => "hash_of_maps",
            Self::Devmap => "devmap",
            Self::Sockmap => "sockmap",
            Self::Cpumap => "cpumap",
            Self::Xskmap => "xskmap",
            Self::Sockhash => "sockhash",
            Self::CgroupStorage => "cgroup_storage",
            Self::ReuseportSockarray => "reuseport_sockarray",
            Self::PercpuCgroupStorage => "percpu_cgroup_storage",
            Self::Queue => "queue",
            Self::Stack => "stack",
            Self::SkStorage => "sk_storage",
            Self::DevmapHash => "devmap_hash",
            Self::StructOps => "struct_ops",
            Self::Ringbuf => "ringbuf",
            Self::InodeStorage => "inode_storage",
            Self::TaskStorage => "task_storage",
            Self::BloomFilter => "bloom_filter",
            Self::UserRingbuf => "user_ringbuf",
            Self::CgrpStorage => "cgrp_storage",
        }
    }

    /// Is per-CPU
    #[inline]
    pub fn is_percpu(&self) -> bool {
        matches!(
            self,
            Self::PercpuHash | Self::PercpuArray | Self::LruPercpuHash | Self::PercpuCgroupStorage
        )
    }

    /// Has LRU eviction
    #[inline(always)]
    pub fn has_lru(&self) -> bool {
        matches!(self, Self::LruHash | Self::LruPercpuHash)
    }
}

/// BPF attach type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfAttachType {
    /// Cgroup ingress
    CgroupInetIngress,
    /// Cgroup egress
    CgroupInetEgress,
    /// Cgroup inet socket create
    CgroupInetSockCreate,
    /// Cgroup sock ops
    CgroupSockOps,
    /// SK SKB stream parser
    SkSkbStreamParser,
    /// SK SKB stream verdict
    SkSkbStreamVerdict,
    /// Cgroup device
    CgroupDevice,
    /// SK MSG verdict
    SkMsgVerdict,
    /// Cgroup inet4 bind
    CgroupInet4Bind,
    /// Cgroup inet6 bind
    CgroupInet6Bind,
    /// Cgroup inet4 connect
    CgroupInet4Connect,
    /// Cgroup inet6 connect
    CgroupInet6Connect,
    /// Cgroup inet4 post bind
    CgroupInet4PostBind,
    /// Cgroup inet6 post bind
    CgroupInet6PostBind,
    /// Cgroup UDP4 sendmsg
    CgroupUdp4Sendmsg,
    /// Cgroup UDP6 sendmsg
    CgroupUdp6Sendmsg,
    /// Lirc mode2
    LircMode2,
    /// Flow dissector
    FlowDissector,
    /// Cgroup sysctl
    CgroupSysctl,
    /// Cgroup UDP4 recvmsg
    CgroupUdp4Recvmsg,
    /// Cgroup UDP6 recvmsg
    CgroupUdp6Recvmsg,
    /// Cgroup getsockopt
    CgroupGetsockopt,
    /// Cgroup setsockopt
    CgroupSetsockopt,
    /// Trace raw tp
    TraceRawTp,
    /// Trace fentry
    TraceFentry,
    /// Trace fexit
    TraceFexit,
    /// Modify return
    ModifyReturn,
    /// LSM MAC
    LsmMac,
    /// Trace iter
    TraceIter,
    /// Cgroup inet4 getpeername
    CgroupInet4Getpeername,
    /// Cgroup inet6 getpeername
    CgroupInet6Getpeername,
    /// Cgroup inet4 getsockname
    CgroupInet4Getsockname,
    /// Cgroup inet6 getsockname
    CgroupInet6Getsockname,
    /// XDP devmap
    XdpDevmap,
    /// Cgroup inet sock release
    CgroupInetSockRelease,
    /// XDP cpumap
    XdpCpumap,
    /// SK lookup
    SkLookup,
    /// XDP
    Xdp,
    /// SK SKB verdict
    SkSkbVerdict,
    /// SK reuseport select
    SkReuseportSelect,
    /// SK reuseport select or migrate
    SkReuseportSelectOrMigrate,
    /// Perf event
    PerfEvent,
    /// Trace kprobe multi
    TraceKprobeMulti,
    /// LSM cgroup
    LsmCgroup,
    /// Struct ops
    StructOps,
    /// Netfilter
    Netfilter,
    /// TCX ingress
    TcxIngress,
    /// TCX egress
    TcxEgress,
}
