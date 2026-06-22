use super::traits::{PeripheralHandler, RequestType, TransferResponse};
use embedded_hal_async::i2c::I2c;

pub struct I2cPeriph<I: I2c> {
    i2c: I,
    rx_buff: [u8; 8],
    _tx_buff: [u8; 8],
}

impl<I: I2c> I2cPeriph<I> {
    pub fn init(&mut self, i2c: I) -> Self {
        Self {
            i2c,
            rx_buff: [0u8; 8],
            _tx_buff: [0u8; 8],
        }
    }
}

impl<I: I2c> PeripheralHandler for I2cPeriph<I> {
    type Error = I::Error;

    async fn transfer(
        &mut self,
        data: [u8; 8],
        transfer_mode: RequestType,
        addr: u8,
    ) -> Result<TransferResponse, Self::Error> {
        match transfer_mode {
            RequestType::Read => {
                self.i2c.write_read(addr, &data[..], &mut self.rx_buff).await?;
                Ok(TransferResponse::Received(self.rx_buff))
            }
            RequestType::Write => {
                self.i2c.write(addr, &data[..]).await?;
                Ok(TransferResponse::Written)
            }
        }
    }
}
