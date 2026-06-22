use super::traits::{PeripheralHandler, RequestType, TransferResponse};
use embedded_io_async::{Read, Write};
use embassy_time::Timer;
use defmt::info;

const SENDDELAY_US: u64 = 100;


pub struct SingleWireUart<U: Write + Read> {
    uart: U,
    rx_buff: [u8; 8],
}

impl<U: Write + Read> SingleWireUart<U> {
    pub fn new(uart: U) -> Self {
        Self {
            uart,
            rx_buff: [0u8; 8],
        }
    }
}

impl<U: Write + Read> PeripheralHandler for SingleWireUart<U> {
    type Error = U::Error;

    async fn transfer(
        &mut self,
        data: [u8; 8],
        transfer_mode: RequestType,
        _addr: u8,
    ) -> Result<TransferResponse, Self::Error> {
        match transfer_mode {
            RequestType::Write => {
                self.uart.write_all(&data).await?;
                self.uart.flush().await?;
                info!("sent data from uart periph: {}", data);
                Ok(TransferResponse::Written)
            }
            RequestType::Read => {
                self.uart.write_all(&data[..4]).await?;
                self.uart.flush().await?;

                Timer::after_micros(SENDDELAY_US).await;
                info!("sent read request from uart periph: {}", data);

                self.rx_buff = [0u8; 8];
                self.uart.read(&mut self.rx_buff).await?;

                info!("got response from device {}", self.rx_buff);
                Ok(TransferResponse::Received(self.rx_buff))
            }
        }
    }
}
