pub struct NetDriver {}

impl smoltcp::phy::Device for NetDriver {
    type RxToken = RxToken;
    type TxToken = TxToken;

    fn receive(&mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        None
    }

    fn transmit(&mut self) -> Option<Self::TxToken> {
        None
    }
}
