#![no_std]

use core::fmt::{self, Debug, Display};
use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;
use embedded_io_async::{ErrorType, Read, ReadReady, Write, WriteReady};

/// Asynchronous driver for MAX485 RS-485 transceivers. Requires a serial port,
/// a RE/DE pin, and a delay provider.
pub struct Max485<RIDO, REDE, DELAY>
where
    RIDO: Read + Write,
    REDE: OutputPin,
{
    serial: RIDO,
    pin: REDE,
    delay: DELAY,
}

impl<RIDO, REDE, DELAY> Max485<RIDO, REDE, DELAY>
where
    RIDO: Read + Write,
    REDE: OutputPin,
{
    pub fn new(serial: RIDO, pin: REDE, delay: DELAY) -> Self {
        Self { serial, pin, delay }
    }

    pub fn take_peripherals(self) -> (RIDO, REDE) {
        (self.serial, self.pin)
    }

    /// Provide a configuration function to be applied to the underlying serial port.
    pub fn reconfig_port<F>(&mut self, config: F)
    where
        F: Fn(&mut RIDO),
    {
        config(&mut self.serial);
    }
}

impl<RIDO, REDE, DELAY> ErrorType for Max485<RIDO, REDE, DELAY>
where
    RIDO: Read + Write,
    REDE: OutputPin,
{
    type Error = crate::Error<RIDO::Error, REDE::Error>;
}

impl<RIDO, REDE, DELAY> Write for Max485<RIDO, REDE, DELAY>
where
    RIDO: Read + Write,
    REDE: OutputPin,
    DELAY: DelayNs,
{
    async fn write(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        self.pin.set_high().map_err(Error::Pin)?;

        let n = self.serial.write(bytes).await.map_err(Error::Serial)?;
        self.serial.flush().await.map_err(Error::Serial)?;
        self.delay.delay_us(50).await;

        self.pin.set_low().map_err(Error::Pin)?;

        Ok(n)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.serial.flush().await.map_err(Error::Serial)
    }
}

impl<RIDO, REDE, DELAY> Read for Max485<RIDO, REDE, DELAY>
where
    RIDO: Read + Write,
    REDE: OutputPin,
{
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.pin.set_low().map_err(Error::Pin)?;
        self.serial.read(buf).await.map_err(Error::Serial)
    }
}

impl<RIDO, REDE, DELAY> ReadReady for Max485<RIDO, REDE, DELAY>
where
    RIDO: Read + Write + ReadReady,
    REDE: OutputPin,
{
    fn read_ready(&mut self) -> Result<bool, Self::Error> {
        self.serial.read_ready().map_err(Error::Serial)
    }
}

impl<RIDO, REDE, DELAY> WriteReady for Max485<RIDO, REDE, DELAY>
where
    RIDO: Read + Write + WriteReady,
    REDE: OutputPin,
{
    fn write_ready(&mut self) -> Result<bool, Self::Error> {
        self.serial.write_ready().map_err(Error::Serial)
    }
}

/// Custom Error type
#[derive(Debug)]
pub enum Error<S, P> {
    Serial(S),
    Pin(P),
}

impl<S, P> Display for Error<S, P>
where
    S: Display,
    P: Debug, // embedded_hal::digital::Error only implements debug
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Serial(s) => write!(f, "serial error: {s}"),
            Error::Pin(p) => write!(f, "pin error: {p:?}"),
        }
    }
}

impl<S, P> core::error::Error for Error<S, P>
where
    S: core::error::Error,
    P: Debug,
{
}

impl<S, P> embedded_io_async::Error for Error<S, P>
where
    S: embedded_io_async::Error,
    P: Debug,
{
    fn kind(&self) -> embedded_io_async::ErrorKind {
        match self {
            Error::Serial(s) => s.kind(),
            Error::Pin(_) => embedded_io_async::ErrorKind::Other,
        }
    }
}
