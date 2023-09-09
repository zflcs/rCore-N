use numeric_enum_macro::numeric_enum;

use crate::SyscallResult;

pub trait SyscallNET {

    /// make current task listen to the socket_fd
    fn listen(port: u16) -> SyscallResult {
        Ok(0)
    }

    /// make current task accept to the socket_fd
    fn accept(port_index: usize) -> SyscallResult {
        Ok(0)
    }
}


numeric_enum! {
    #[repr(usize)]
    #[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
    #[allow(non_camel_case_types)]
    pub enum ProtoFamily {
        AF_INET = 0,
        AF_INET6 = 1,
        AF_LOCAL = 2,
        AF_ROUTE = 3
    }
}

numeric_enum! {
    #[repr(usize)]
    #[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
    #[allow(non_camel_case_types)]
    pub enum SocketType {
        SOCK_STREAM = 0,
        SOCK_DGRAM = 1,
        SOCK_RAW = 2,
        SOCK_PACKET = 3,
        SOCK_SEQPACKET = 4,
    }
}

numeric_enum! {
    #[repr(usize)]
    #[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
    #[allow(non_camel_case_types)]
    pub enum Protocol {
        IPPROTO_TCP = 0,
        IPPTOTO_UDP = 1,
        IPPROTO_SCTP = 2,
        IPPROTO_TIPC = 3,
    }
}