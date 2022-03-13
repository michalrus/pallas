pub use crate::payloads::*;
use pallas_codec::Fragment;
use pallas_multiplexer::{Channel, Payload};
use std::fmt::{Debug, Display};
use std::sync::mpsc::Sender;

#[derive(Debug)]
pub enum MachineError<State, Msg>
where
    State: Debug,
    Msg: Debug,
{
    InvalidMsgForState(State, Msg),
}

impl<S, M> Display for MachineError<S, M>
where
    S: Debug,
    M: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MachineError::InvalidMsgForState(msg, state) => {
                write!(
                    f,
                    "received invalid message ({:?}) for current state ({:?})",
                    msg, state
                )
            }
        }
    }
}

impl<S, M> std::error::Error for MachineError<S, M>
where
    S: Debug,
    M: Debug,
{
}

#[derive(Debug)]
pub enum CodecError {
    BadLabel(u16),
    UnexpectedCbor(&'static str),
}

impl std::error::Error for CodecError {}

impl Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::BadLabel(label) => {
                write!(f, "unknown message label: {}", label)
            }
            CodecError::UnexpectedCbor(msg) => {
                write!(f, "unexpected cbor: {}", msg)
            }
        }
    }
}

pub trait MachineOutput {
    fn send_msg(&self, data: &impl Fragment) -> Result<(), Box<dyn std::error::Error>>;
}

impl MachineOutput for Sender<Payload> {
    fn send_msg(&self, data: &impl Fragment) -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = Vec::new();
        data.write_cbor(&mut payload)?;
        self.send(payload)?;

        Ok(())
    }
}

pub type Transition<T> = Result<T, Box<dyn std::error::Error>>;

pub trait Agent: Sized {
    type Message;

    fn is_done(&self) -> bool;
    fn has_agency(&self) -> bool;
    fn send_next(self, tx: &impl MachineOutput) -> Transition<Self>;
    fn receive_next(self, msg: Self::Message) -> Transition<Self>;
}

pub fn run_agent<T>(agent: T, channel: &mut Channel) -> Result<T, Box<dyn std::error::Error>>
where
    T: Agent + Debug,
    T::Message: Fragment + Debug,
{
    let Channel(tx, rx) = channel;

    let mut agent = agent;

    while !agent.is_done() {
        log::debug!("evaluating agent {:?}", agent);

        match agent.has_agency() {
            true => {
                agent = agent.send_next(tx)?;
            }
            false => {
                let mut buffer = Vec::new();

                let msg = read_until_full_msg::<T::Message>(&mut buffer, rx).unwrap();
                log::trace!("procesing inbound msg: {:?}", msg);
                agent = agent.receive_next(msg)?;
            }
        }
    }

    Ok(agent)
}
