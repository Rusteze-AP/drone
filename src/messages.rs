pub type NodeId = u64;

struct SourceRoutingHeader {
	hops: Vec<u64>
}

#[derive(Clone)]
pub struct Ack{
	fragment_index: u64,
	time_received: std::time::Instant
}

#[derive(Clone)]
pub struct Nack{
	fragment_index: u64,
	time_of_fail: std::time::Instant,
	nack_type: NackType
}

#[derive(Clone)]
pub enum NackType{
	ErrorInRouting(NodeId),
	Dropped()
}

#[derive(Clone)]
pub enum PacketType {
	MsgFragment(Fragment),
	Nack(Nack),
	Ack(Ack)
}

#[derive(Clone)]
pub struct Fragment {
	fragment_index: u64,
	total_n_fragments: u64,
	data: FragmentData
}

#[derive(Clone)]
pub struct FragmentData {
	length: u8,
	data: [u8; 80]
}

pub struct Packet {
	pub pack_type: PacketType,
	pub routing_header: SourceRoutingHeader,
	pusession_id: u64
}