#[derive(Debug, Clone, Copy)]
pub enum MessageSource {
    Kernel = -1,
    VMWare = 1,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageType {
    NotifyField { notications: usize },
}
