use wg_internal::network::{SourceRoutingHeader, NodeId};
use wg_internal::packet::{Fragment, FragmentData, Packet, PacketType};

pub trait RustezePacket {
    fn new(
        pack_type: PacketType,
        routing_header: Box<impl RustezeSourceRoutingHeader>,
        session_id: u64,
    ) -> Self;
    fn get_pack_type(&self) -> &PacketType;
    fn get_routing_header(&self) -> Box<impl RustezeSourceRoutingHeader>;
    fn get_session_id(&self) -> u64;
}

impl RustezePacket for Packet {
    fn new(
        pack_type: PacketType,
        routing_header: Box<impl RustezeSourceRoutingHeader>,
        session_id: u64,
    ) -> Self {
        Self {
            pack_type,
            routing_header: routing_header.to_source_routing_header(),
            session_id,
        }
    }

    fn get_pack_type(&self) -> &PacketType {
        &self.pack_type
    }

    fn get_routing_header(&self) -> Box<impl RustezeSourceRoutingHeader> {
        Box::new(SourceRoutingHeader {
            hop_index: self.routing_header.hop_index,
            hops: self.routing_header.hops.clone(),
        })
    }

    fn get_session_id(&self) -> u64 {
        self.session_id
    }
}

pub trait RustezeFragment {
    fn default() -> Self;
    fn new(fragment_index: u64, total_n_fragments: u64, data: FragmentData) -> Self;
}

impl RustezeFragment for Fragment {
    fn default() -> Self {
        Self {
            fragment_index: 0,
            total_n_fragments: 0,
            data: FragmentData {
                length: 0,
                data: [0; 80],
            },
        }
    }

    fn new(fragment_index: u64, total_n_fragments: u64, data: FragmentData) -> Fragment {
        Fragment {
            fragment_index,
            total_n_fragments,
            data,
        }
    }
}

pub trait RustezeSourceRoutingHeader {
    fn default() -> Self;
    fn new(hops: Vec<NodeId>, hop_index: usize) -> Self;
    fn to_source_routing_header(&self) -> SourceRoutingHeader;
    fn get_current_hop(&self) -> Option<NodeId>;
    fn increment_index(&mut self);
    fn decrement_index(&mut self);
    fn get_next_hop(&self) -> Option<NodeId>;
    fn get_previous_hop(&self) -> Option<NodeId>;
    
}

impl RustezeSourceRoutingHeader for SourceRoutingHeader {
    fn default() -> Self {
        Self {
            hops: vec![],
            hop_index: 0,
        }
    }
    
    fn new(hops: Vec<NodeId>, hop_index: usize) -> Self {
        Self { hops, hop_index }
    }
    
    fn to_source_routing_header(&self) -> Self {
        Self {
            hop_index: self.hop_index,
            hops: self.hops.clone(),
        }
    }
    
    fn get_current_hop(&self) -> Option<NodeId> {
        match self.hops.get(self.hop_index) {
            None => None,
            Some(current) => Some(*current),
        }
    }

    fn increment_index(&mut self) {
        self.hop_index += 1;
    }

    fn decrement_index(&mut self) {
        self.hop_index -= 1;
    }

    fn get_next_hop(&self) -> Option<NodeId> {        
        match self.hops.get(self.hop_index + 1) {
            None => None,
            Some(current) => Some(*current),
        }
    }

    fn get_previous_hop(&self) -> Option<NodeId> {
        match self.hops.get(self.hop_index - 1) {
            None => None,
            Some(current) => Some(*current),
        }
    }
}
