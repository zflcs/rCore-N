mod net;
mod tcp;

pub use net::{init_net, virtio_net, NET_BUFFER_LEN, NET_QUEUE_SIZE};