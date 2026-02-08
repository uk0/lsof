use std::fmt;

#[derive(Debug, Clone)]
pub struct NetworkInfo {
    pub protocol: Protocol,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: TcpState,
    pub pid: Option<u32>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    Tcp,
    Tcp6,
    Udp,
    Udp6,
    Unix,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "TCP"),
            Protocol::Tcp6 => write!(f, "TCP6"),
            Protocol::Udp => write!(f, "UDP"),
            Protocol::Udp6 => write!(f, "UDP6"),
            Protocol::Unix => write!(f, "unix"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TcpState {
    Listen,
    Established,
    CloseWait,
    TimeWait,
    SynSent,
    SynRecv,
    FinWait1,
    FinWait2,
    Closing,
    LastAck,
    Closed,
    Unknown(String),
}

impl fmt::Display for TcpState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TcpState::Listen => write!(f, "LISTEN"),
            TcpState::Established => write!(f, "ESTABLISHED"),
            TcpState::CloseWait => write!(f, "CLOSE_WAIT"),
            TcpState::TimeWait => write!(f, "TIME_WAIT"),
            TcpState::SynSent => write!(f, "SYN_SENT"),
            TcpState::SynRecv => write!(f, "SYN_RECV"),
            TcpState::FinWait1 => write!(f, "FIN_WAIT1"),
            TcpState::FinWait2 => write!(f, "FIN_WAIT2"),
            TcpState::Closing => write!(f, "CLOSING"),
            TcpState::LastAck => write!(f, "LAST_ACK"),
            TcpState::Closed => write!(f, "CLOSED"),
            TcpState::Unknown(s) => write!(f, "{}", s),
        }
    }
}
