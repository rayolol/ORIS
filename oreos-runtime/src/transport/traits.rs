use embassy_sync::channel::{Channel, Sender};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

pub type CSRM = CriticalSectionRawMutex;

const CHANNEL_SIZE: usize = 10;

pub enum RequestType {
    Write,
    Read
}

#[allow(async_fn_in_trait)]
pub trait TransportHandler<const N: usize> {
    type Error;
    async fn transfer(&mut self, data: [u8; N], transfer_mode: RequestType) -> Result<Option<[u8; N]>, Self::Error>;
}

pub enum TransferResponse {
    Written,
    Received([u8; 8]),
}

pub enum ServerError {
    PeripheralError,
    EmptyResponse,
    WriteReturn
}

pub struct PacketRequest {
    // pub sender_id: u8,
    pub transfer_mode: RequestType,
    pub payload: [u8; 8],
    pub response_sender: Option<Sender<'static, CSRM, PacketResponse, 1>>
}

pub struct PacketResponse {
    pub payload: [u8; 8]
}

#[allow(async_fn_in_trait)]
pub trait PeripheralHandler {
    type Error;
    async fn transfer(&mut self, data: [u8; 8], transfer_mode: RequestType, target: u8) -> Result<TransferResponse, Self::Error>;
}



pub struct TransportServer<W: PeripheralHandler> {
    pub peripheral: W,
    pub channel_rx: &'static Channel<CSRM, PacketRequest, CHANNEL_SIZE>,
}

impl<W: PeripheralHandler> TransportServer<W> {
    pub const fn new(periph: W, channel: &'static Channel<CSRM, PacketRequest, CHANNEL_SIZE>) -> Self {
        Self {
            peripheral: periph,
            channel_rx: channel
        }
    }
}


impl<W: PeripheralHandler> TransportServer<W> {

    pub async fn run(&mut self) -> Result<(), ServerError> {
        loop {
            let packet: PacketRequest = self.channel_rx.receive().await;

            let transfer_result = self.peripheral.transfer(packet.payload, packet.transfer_mode, 0).await;
            let transfer_result = transfer_result.map_err(|_e| ServerError::PeripheralError)?;
            match transfer_result
            {
                TransferResponse::Received(resp) => {
                    let packet_resp = PacketResponse { payload: resp };
                    if let Some(sender) = packet.response_sender {
                        sender.send(packet_resp).await;
                    } else {
                        return Err(ServerError::EmptyResponse);
                    }
                }
                TransferResponse::Written => {
                    return Ok(());
                }
            }

            return Ok(())
        }
    }
}

pub type ChannelTransport = static_cell::StaticCell<embassy_sync::channel::Channel<CSRM, PacketRequest, CHANNEL_SIZE>>;

pub struct TransportClient {
    channel_rx_slots: [Channel<CSRM, PacketResponse, 1>; 1],
    channel_tx_slots: &'static Channel<CSRM, PacketRequest, CHANNEL_SIZE>
}


impl TransportClient {
    pub fn new(channel: &'static Channel<CSRM, PacketRequest, CHANNEL_SIZE>) -> Self {
        Self {
            channel_rx_slots: [Channel::new()],
            channel_tx_slots: channel
        }
    }

    pub async fn transfer(
        &'static self,
        payload: [u8; 8],
        transfer_mode: RequestType,
    ) -> Result<Option<[u8; 8]>, ServerError> {
        let (response_sender, response_receiver) = match transfer_mode {
            RequestType::Write => (None, None),
            RequestType::Read => (
                Some(self.channel_rx_slots[0].sender()),
                Some(self.channel_rx_slots[0].receiver()),
            ),
        };

        let packet = PacketRequest {
            transfer_mode,
            payload,
            response_sender,
        };

        self.channel_tx_slots.send(packet).await;

        match response_receiver {
            Some(receiver) => {
                let mut data = [0_u8; 8];
                data.copy_from_slice(&receiver.receive().await.payload);
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }
}




// TODO: fix issues with implementation

impl TransportHandler<8> for &'static TransportClient {
    type Error = ServerError;

    async fn transfer(&mut self, payload: [u8; 8], transfer_mode: RequestType) -> Result<Option<[u8; 8]>, ServerError> {
        TransportClient::transfer(self, payload, transfer_mode).await
    }
}

// Adapter: allow using the channel-based `TransportClient` (8-byte packets)
// where drivers expect a 5-byte transport (TMC2160 SPI frames).
impl TransportHandler<5> for &'static TransportClient {
    type Error = ServerError;

    async fn transfer(&mut self, data: [u8; 5], transfer_mode: RequestType) -> Result<Option<[u8; 5]>, ServerError> {
        // Pack the 5-byte frame into an 8-byte container (prefix with the 5 bytes,
        // leave the remaining bytes as zero). The transport channel uses fixed
        // 8-byte payloads, so we forward through that path.
        let mut packet: [u8; 8] = [0u8; 8];
        packet[..5].copy_from_slice(&data);

        let res = TransportClient::transfer(self, packet, transfer_mode).await?;

        match res {
            Some(resp8) => {
                let mut resp5 = [0u8; 5];
                resp5.copy_from_slice(&resp8[..5]);
                Ok(Some(resp5))
            }
            None => Ok(None),
        }
    }
}
