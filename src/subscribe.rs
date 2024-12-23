#[cfg(feature = "defmt")]
use defmt::Format;
use crate::{decoder::*, encoder::*, *};
#[cfg(feature = "derive")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
pub type LimitedVec<T> = std::vec::Vec<T>;
#[cfg(not(feature = "std"))]
pub type LimitedVec<T> = heapless::Vec<T, 5>;

#[cfg(feature = "std")]
pub type LimitedString = std::string::String;
#[cfg(not(feature = "std"))]
pub type LimitedString = heapless::String<256>;

use core::str::FromStr;

/// Subscribe topic.
///
/// [Subscribe] packets contain a `Vec` of those.
///
/// [Subscribe]: struct.Subscribe.html
#[cfg_attr(feature = "defmt",derive(Format))]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
pub struct SubscribeTopic {
    pub topic_path: LimitedString,
    pub qos: QoS,
}

impl SubscribeTopic {
    pub(crate) fn from_buffer(buf: &[u8], offset: &mut usize) -> Result<Self, Error> {
        let topic_path = LimitedString::from_str(read_str(buf, offset)?).unwrap();
        let qos = QoS::from_u8(buf[*offset])?;
        *offset += 1;
        Ok(SubscribeTopic { topic_path, qos })
    }
}

/// Subscribe return value.
///
/// [Suback] packets contain a `Vec` of those.
///
/// [Suback]: struct.Subscribe.html
#[cfg_attr(feature = "defmt",derive(Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscribeReturnCodes {
    Success(QoS),
    Failure,
}

impl SubscribeReturnCodes {
    pub(crate) fn from_buffer<'a>(buf: &'a [u8], offset: &mut usize) -> Result<Self, Error> {
        let code = buf[*offset];
        *offset += 1;

        if code == 0x80 {
            Ok(SubscribeReturnCodes::Failure)
        } else {
            Ok(SubscribeReturnCodes::Success(QoS::from_u8(code)?))
        }
    }

    pub(crate) fn to_u8(&self) -> u8 {
        match *self {
            SubscribeReturnCodes::Failure => 0x80,
            SubscribeReturnCodes::Success(qos) => qos.to_u8(),
        }
    }
}

/// Subscribe packet ([MQTT 3.8]).
///
/// [MQTT 3.8]: http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html#_Toc398718063
#[cfg_attr(feature = "defmt",derive(Format))]
#[derive(Debug, Clone, PartialEq)]
pub struct Subscribe {
    pub pid: Pid,
    pub topics: LimitedVec<SubscribeTopic>,
}

/// Subsack packet ([MQTT 3.9]).
///
/// [MQTT 3.9]: http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html#_Toc398718068
#[cfg_attr(feature = "defmt",derive(Format))]
#[derive(Debug, Clone, PartialEq)]
pub struct Suback {
    pub pid: Pid,
    pub return_codes: LimitedVec<SubscribeReturnCodes>,
}

/// Unsubscribe packet ([MQTT 3.10]).
///
/// [MQTT 3.10]: http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html#_Toc398718072
#[cfg_attr(feature = "defmt",derive(Format))]
#[derive(Debug, Clone, PartialEq)]
pub struct Unsubscribe {
    pub pid: Pid,
    pub topics: LimitedVec<LimitedString>,
}

impl Subscribe {
    pub fn new(pid: Pid, topics: LimitedVec<SubscribeTopic>) -> Self {
        Subscribe { pid, topics }
    }

    pub(crate) fn from_buffer(
        remaining_len: usize,
        buf: &[u8],
        offset: &mut usize,
    ) -> Result<Self, Error> {
        let payload_end = *offset + remaining_len;
        let pid = Pid::from_buffer(buf, offset)?;

        let mut topics = LimitedVec::new();
        while *offset < payload_end {
            let _res = topics.push(SubscribeTopic::from_buffer(buf, offset)?);

            #[cfg(not(feature = "std"))]
            _res.map_err(|_| Error::InvalidLength)?;
        }

        Ok(Subscribe { pid, topics })
    }

    pub(crate) fn to_buffer(&self, buf: &mut [u8], offset: &mut usize) -> Result<usize, Error> {
        let header: u8 = 0b10000010;
        check_remaining(buf, offset, 1)?;
        write_u8(buf, offset, header)?;

        // Length: pid(2) + topic.for_each(2+len + qos(1))
        let mut length = 2;
        for topic in &self.topics {
            length += topic.topic_path.len() + 2 + 1;
        }
        let write_len = write_length(buf, offset, length)? + 1;

        // Pid
        self.pid.to_buffer(buf, offset)?;

        // Topics
        for topic in &self.topics {
            write_string(buf, offset, topic.topic_path.as_str())?;
            write_u8(buf, offset, topic.qos.to_u8())?;
        }

        Ok(write_len)
    }
}

impl Unsubscribe {
    pub fn new(pid: Pid, topics: LimitedVec<LimitedString>) -> Self {
        Unsubscribe { pid, topics }
    }

    pub(crate) fn from_buffer(
        remaining_len: usize,
        buf: &[u8],
        offset: &mut usize,
    ) -> Result<Self, Error> {
        let payload_end = *offset + remaining_len;
        let pid = Pid::from_buffer(buf, offset)?;

        let mut topics = LimitedVec::new();
        while *offset < payload_end {
            let _res = topics.push(LimitedString::from_str(read_str(buf, offset)?).unwrap());

            #[cfg(not(feature = "std"))]
            _res.map_err(|_| Error::InvalidLength)?;
        }

        Ok(Unsubscribe { pid, topics })
    }

    pub(crate) fn to_buffer(&self, buf: &mut [u8], offset: &mut usize) -> Result<usize, Error> {
        let header: u8 = 0b10100010;
        let mut length = 2;
        for topic in &self.topics {
            length += 2 + topic.len();
        }
        check_remaining(buf, offset, 1)?;
        write_u8(buf, offset, header)?;

        let write_len = write_length(buf, offset, length)? + 1;
        self.pid.to_buffer(buf, offset)?;
        for topic in &self.topics {
            write_string(buf, offset, topic)?;
        }
        Ok(write_len)
    }
}

impl Suback {
    pub fn new(pid: Pid, return_codes: LimitedVec<SubscribeReturnCodes>) -> Self {
        Suback { pid, return_codes }
    }

    pub(crate) fn from_buffer(
        remaining_len: usize,
        buf: &[u8],
        offset: &mut usize,
    ) -> Result<Self, Error> {
        let payload_end = *offset + remaining_len;
        let pid = Pid::from_buffer(buf, offset)?;

        let mut return_codes = LimitedVec::new();
        while *offset < payload_end {
            let _res = return_codes.push(SubscribeReturnCodes::from_buffer(buf, offset)?);

            #[cfg(not(feature = "std"))]
            _res.map_err(|_| Error::InvalidLength)?;
        }

        Ok(Suback { pid, return_codes })
    }

    pub(crate) fn to_buffer(&self, buf: &mut [u8], offset: &mut usize) -> Result<usize, Error> {
        let header: u8 = 0b10010000;
        let length = 2 + self.return_codes.len();
        check_remaining(buf, offset, 1)?;
        write_u8(buf, offset, header)?;

        let write_len = write_length(buf, offset, length)? + 1;
        self.pid.to_buffer(buf, offset)?;
        for rc in &self.return_codes {
            write_u8(buf, offset, rc.to_u8())?;
        }
        Ok(write_len)
    }
}
