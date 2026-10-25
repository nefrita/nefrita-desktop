use std::collections::VecDeque;

use crate::pin_server::{PinServer, PinServerError, PinServerPayload, PinServerRequest};

pub(crate) struct ScriptedPinServer {
    scripted_exchanges: VecDeque<Result<PinServerPayload, PinServerError>>,
    received_requests: Vec<PinServerRequest>,
}

impl ScriptedPinServer {
    pub(crate) fn answering(
        scripted_exchanges: Vec<Result<PinServerPayload, PinServerError>>,
    ) -> Self {
        Self {
            scripted_exchanges: scripted_exchanges.into(),
            received_requests: Vec::new(),
        }
    }

    pub(crate) fn received_requests(&self) -> &[PinServerRequest] {
        &self.received_requests
    }
}

impl PinServer for ScriptedPinServer {
    fn exchange(
        &mut self,
        pin_server_request: &PinServerRequest,
    ) -> Result<PinServerPayload, PinServerError> {
        self.received_requests.push(pin_server_request.clone());
        self.scripted_exchanges
            .pop_front()
            .expect("the PIN server should not be asked more times than scripted")
    }
}
