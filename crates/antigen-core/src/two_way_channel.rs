use crossbeam_channel::{Sender, Receiver, unbounded, bounded};
/// One half of a two-way cross-thread communication channel
pub struct TwoWayChannel<TX, RX> {
    pub tx: Sender<TX>,
    pub rx: Receiver<RX>,
}

impl<TX, RX> TwoWayChannel<TX, RX> {
    pub fn unbounded() -> (TwoWayChannel<TX, RX>, TwoWayChannel<RX, TX>) {
        let (tx0, rx0) = unbounded::<TX>();
        let (tx1, rx1) = unbounded::<RX>();
        (
            TwoWayChannel { tx: tx0, rx: rx1 },
            TwoWayChannel { tx: tx1, rx: rx0 },
        )
    }

    pub fn bounded(cap: usize) -> (TwoWayChannel<TX, RX>, TwoWayChannel<RX, TX>) {
        let (tx0, rx0) = bounded::<TX>(cap);
        let (tx1, rx1) = bounded::<RX>(cap);
        (
            TwoWayChannel { tx: tx0, rx: rx1 },
            TwoWayChannel { tx: tx1, rx: rx0 },
        )
    }
}

