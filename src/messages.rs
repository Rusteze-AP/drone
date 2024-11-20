pub type NodeId = u64;

#[derive(Clone)]
pub struct SourceRoutingHeader {
    hops: Vec<NodeId>,
    hop_index: usize,
}

#[derive(Clone)]
pub struct Ack {
    fragment_index: u64,
    time_received: std::time::Instant,
}

#[derive(Clone)]
pub struct Nack {
    fragment_index: u64,
    time_of_fail: std::time::Instant,
    nack_type: NackType,
}

#[derive(Clone)]
pub enum NackType {
    ErrorInRouting(NodeId),
    Dropped(),
}

#[derive(Clone)]
pub enum PacketType {
    MsgFragment(Fragment),
    Nack(Nack),
    Ack(Ack),
}

#[derive(Clone)]
pub struct Fragment {
    fragment_index: u64,
    total_n_fragments: u64,
    data: FragmentData,
}

#[derive(Clone)]
pub struct FragmentData {
    length: u8,
    data: [u8; 80],
}

pub struct Packet {
	pub pack_type: PacketType,
	pub routing_header: SourceRoutingHeader,
	pub session_id: u64,
}

impl Packet {
    pub fn new(
        pack_type: PacketType,
        routing_header: SourceRoutingHeader,
        session_id: u64,
    ) -> Packet {
        Packet {
            pack_type,
            routing_header,
            session_id,
        }
    }
    
    pub fn get_pack_type(&self)->&PacketType{
        &self.pack_type
    }
    
    pub fn get_routing_header(&self)->SourceRoutingHeader{
        self.routing_header.clone()
    }
    
    pub fn get_session_id(&self)->u64{
        self.session_id
    }
}

impl Fragment {
    pub fn default() -> Fragment {
        Fragment {
            fragment_index: 0,
            total_n_fragments: 0,
            data: FragmentData::new(0, [0; 80]),
        }
    }
    pub fn new(fragment_index: u64, total_n_fragments: u64, data: FragmentData) -> Fragment {
        Fragment {
            fragment_index,
            total_n_fragments,
            data,
        }
    }
}

impl FragmentData {
    pub fn new(length: u8, data: [u8; 80]) -> FragmentData {
        FragmentData { length, data }
    }
}

impl Nack {
    pub fn default() -> Nack {
        Nack {
            fragment_index: 0,
            time_of_fail: std::time::Instant::now(),
            nack_type: NackType::Dropped(),
        }
    }
    pub fn new(fragment_index: u64, time_of_fail: std::time::Instant, nack_type: NackType) -> Nack {
        Nack {
            fragment_index,
            time_of_fail,
            nack_type,
        }
    }
}

impl Ack {
    pub fn default() -> Ack {
        Ack {
            fragment_index: 0,
            time_received: std::time::Instant::now(),
        }
    }
    pub fn new(fragment_index: u64, time_received: std::time::Instant) -> Ack {
        Ack {
            fragment_index,
            time_received,
        }
    }
}

impl SourceRoutingHeader {
    pub fn default() -> SourceRoutingHeader {
        SourceRoutingHeader { hops: vec![], hop_index: 0 }
    }
    pub fn new(hops: Vec<u64>, hop_index: usize) -> SourceRoutingHeader {
        SourceRoutingHeader { hops , hop_index}
    }
    
    pub fn get_current_hop(&self)->Option<NodeId>{
        match self.hops.get(self.hop_index){
            None => None,
            Some(current) => Some(*current)
        }
    }
    
    pub fn increment_index(&mut self){
        self.hop_index += 1;
    }
    
    pub fn decrement_index(&mut self){
        self.hop_index -= 1;
    }

    pub fn get_next_hop(&self) -> Option<NodeId> {
        match self.hops.get(self.hop_index + 1){
            None => None,
            Some(current) => Some(*current)
        }
    }
    
    pub fn get_previous_hop(&self) -> Option<NodeId> {
        match self.hops.get(self.hop_index - 1){
            None => None,
            Some(current) => Some(*current)
        }
    }
}
