use super::traits::{PeripheralHandler, RequestType, TransferResponse};
use embedded_hal_async::spi::SpiDevice;

pub struct SpiPeriph<S: SpiDevice, const N: usize> {
    spi: [S; N],
    rx_buf: [u8; 8],
    tx_buf: [u8; 8],
}

impl<S: SpiDevice, const N: usize> SpiPeriph<S, N> {
    pub fn init(&mut self, spi: [S; N]) -> Self {
        Self {
            spi,
            rx_buf: [0u8; 8],
            tx_buf: [0u8; 8],
        }
    }
}

impl<S: SpiDevice, const N: usize> PeripheralHandler for SpiPeriph<S, N> {
    type Error = S::Error;

    async fn transfer(
        &mut self,
        data: [u8; 8],
        transfer_mode: RequestType,
        addr: u8,
    ) -> Result<TransferResponse, Self::Error> {
        match transfer_mode {
            RequestType::Write => {
                self.spi[addr as usize].write(&data[..]).await?;
                Ok(TransferResponse::Written)
            }
            RequestType::Read => {
                self.tx_buf.copy_from_slice(&data);
                self.spi[addr as usize]
                    .transfer(&mut self.rx_buf, &mut self.tx_buf)
                    .await?;
                Ok(TransferResponse::Received(self.rx_buf))
            }
        }
    }
}
