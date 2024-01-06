use crate::{node_info::NodeAddress, Discv5Error, Enr};
use std::collections::HashSet;

/// The minimum number of peers to accept sessions with that have an unreachable ENR, i.e. cater
/// requests for, at a time. Benevolent peers of this type could for example be symmetrically
/// NAT:ed nodes or nodes that have recently joined the network and are still unaware of their
/// externally reachable socket, relying on their peers to discover it.
pub const MIN_SESSIONS_UNREACHABLE_ENR: usize = 1;

pub(crate) struct SessionLimiter {
    /// Keeps track of the sessions held for peers with unreachable ENRs. These could be peers yet
    /// to discover their externally reachable socket or symmetrically NAT:ed peers that,
    /// naturally, will never discover one externally reachable socket.
    sessions_unreachable_enr_tracker: HashSet<NodeAddress>,
    /// Receiver of expired sessions.
    rx_expired_sessions: futures::channel::mpsc::Receiver<NodeAddress>,
    /// The max number of sessions to peers with unreachable ENRs at a time.
    limit: usize,
}

impl SessionLimiter {
    pub fn new(
        rx_expired_sessions: futures::channel::mpsc::Receiver<NodeAddress>,
        limit: usize,
    ) -> Self {
        SessionLimiter {
            sessions_unreachable_enr_tracker: Default::default(),
            rx_expired_sessions,
            limit,
        }
    }

    /// Drains buffer of expired sessions, and untracks any which belong to unreachable ENRs.
    fn drain_expired_sessions_buffer(&mut self) {
        while let Ok(Some(session_node_address)) = self.rx_expired_sessions.try_next() {
            self.sessions_unreachable_enr_tracker
                .remove(&session_node_address);
        }
    }

    /// Checks if a session with this peer should be allowed at this given time. Called after
    /// connection establishment, before session key derivation. As a side effect this drains the
    /// expired entries buffer.
    pub fn track_sessions_unreachable_enr(
        &mut self,
        node_address: &NodeAddress,
        enr: &Enr,
    ) -> Result<(), Discv5Error> {
        self.drain_expired_sessions_buffer();

        // Peer is reachable
        if enr.udp4_socket().is_some() || enr.udp6_socket().is_some() {
            return Ok(());
        }
        // Peer is unreachable.
        if self.sessions_unreachable_enr_tracker.len() >= self.limit {
            return Err(Discv5Error::LimitSessionsUnreachableEnr);
        }

        self.sessions_unreachable_enr_tracker
            .insert(node_address.clone());
        Ok(())
    }

    /// Untracks the given session if it has an unreachable ENR.
    pub fn untrack_session(&mut self, node_address: &NodeAddress) {
        self.sessions_unreachable_enr_tracker.remove(&node_address);
    }
}
