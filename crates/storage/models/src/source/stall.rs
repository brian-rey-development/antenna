use std::time::Duration;

use ureq::Error;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, DefaultConnector, NextTimeout, Transport,
};

/// Opens the default connections and limits the wait for each read of the response.
#[derive(Debug)]
pub(crate) struct ReadTimeoutConnector {
    inner: DefaultConnector,
    read_timeout: Duration,
}

impl ReadTimeoutConnector {
    pub(crate) fn new(read_timeout: Duration) -> Self {
        Self {
            inner: DefaultConnector::default(),
            read_timeout,
        }
    }
}

impl Connector for ReadTimeoutConnector {
    type Out = Box<dyn Transport>;

    fn connect(
        &self,
        details: &ConnectionDetails<'_>,
        chained: Option<()>,
    ) -> Result<Option<Self::Out>, Error> {
        let transport = self.inner.connect(details, chained)?;
        let read_timeout = self.read_timeout;
        Ok(transport.map(|inner| -> Box<dyn Transport> {
            Box::new(ReadTimeoutTransport {
                inner,
                read_timeout,
            })
        }))
    }
}

#[derive(Debug)]
struct ReadTimeoutTransport {
    inner: Box<dyn Transport>,
    read_timeout: Duration,
}

impl Transport for ReadTimeoutTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        self.inner.buffers()
    }

    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), Error> {
        self.inner.transmit_output(amount, timeout)
    }

    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, Error> {
        let after = timeout.after.min(self.read_timeout.into());
        self.inner.await_input(NextTimeout { after, ..timeout })
    }

    fn is_open(&mut self) -> bool {
        self.inner.is_open()
    }

    fn is_tls(&self) -> bool {
        self.inner.is_tls()
    }
}
