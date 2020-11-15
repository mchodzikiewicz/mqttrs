use crate::*;
use bytes::BytesMut;
use proptest::{bool, collection::vec, num::*, prelude::*};
use core::convert::TryFrom;

// Proptest strategies to generate packet elements
prop_compose! {
    fn stg_topic()(topic in "[a-z0-9/]{1,100}") -> String {
        topic
    }
}
prop_compose! {
    fn stg_qos()(qos in 0u8..=2) -> QoS {
        QoS::from_u8(qos).unwrap()
    }
}
prop_compose! {
    fn stg_pid()(pid in 1..core::u16::MAX) -> Pid {
        Pid::try_from(pid).unwrap()
    }
}
prop_compose! {
    fn stg_subtopic()(topic_path in stg_topic(), qos in stg_qos()) -> SubscribeTopic {
        SubscribeTopic { topic_path, qos }
    }
}
prop_compose! {
    fn stg_subretcode()(success in bool::ANY, qos in stg_qos()) -> SubscribeReturnCodes {
        if success {
            SubscribeReturnCodes::Success(qos)
        } else {
            SubscribeReturnCodes::Failure
        }
    }
}
prop_compose! {
    fn stg_optstr()(opt in bool::ANY, s in ".{0,200}") -> Option<String> {
        if opt { Some(s) } else { None }
    }
}

// Proptest strategies to generate packets
prop_compose! {
    fn stg_connect()(keep_alive in u16::ANY,
                     client_id in ".{0,100}",
                     clean_session in bool::ANY,
                     username in stg_optstr(),
                     password in stg_optstr()) -> Packet {
        Packet::Connect(Connect { protocol: Protocol::MQTT311,
                                  keep_alive,
                                  client_id,
                                  clean_session,
                                  last_will: None,
                                  username,
                                  password: password.map(|p| p.as_bytes().to_vec()) })
    }
}
prop_compose! {
    fn stg_connack()(session_present in bool::ANY, code in 0u8..6) -> Packet {
        Packet::Connack(Connack { session_present,
                                  code: ConnectReturnCode::from_u8(code).unwrap() })
    }
}
prop_compose! {
    fn stg_publish()(dup in bool::ANY,
                     qos in stg_qos(),
                     pid in stg_pid(),
                     retain in bool::ANY,
                     topic_name in stg_topic(),
                     payload in vec(0u8..255u8, 1..300)) -> Packet {
        Packet::Publish(Publish{dup,
                                qospid: match qos {
                                    QoS::AtMostOnce => QosPid::AtMostOnce,
                                    QoS::AtLeastOnce => QosPid::AtLeastOnce(pid),
                                    QoS::ExactlyOnce => QosPid::ExactlyOnce(pid),
                                },
                                retain,
                                topic_name,
                                payload})
    }
}
prop_compose! {
    fn stg_puback()(pid in stg_pid()) -> Packet {
        Packet::Puback(pid)
    }
}
prop_compose! {
    fn stg_pubrec()(pid in stg_pid()) -> Packet {
        Packet::Pubrec(pid)
    }
}
prop_compose! {
    fn stg_pubrel()(pid in stg_pid()) -> Packet {
        Packet::Pubrel(pid)
    }
}
prop_compose! {
    fn stg_pubcomp()(pid in stg_pid()) -> Packet {
        Packet::Pubcomp(pid)
    }
}
prop_compose! {
    fn stg_subscribe()(pid in stg_pid(), topics in vec(stg_subtopic(), 0..20)) -> Packet {
        Packet::Subscribe(Subscribe{pid: pid, topics})
    }
}
prop_compose! {
    fn stg_suback()(pid in stg_pid(), return_codes in vec(stg_subretcode(), 0..300)) -> Packet {
        Packet::Suback(Suback{pid: pid, return_codes})
    }
}
prop_compose! {
    fn stg_unsubscribe()(pid in stg_pid(), topics in vec(stg_topic(), 0..20)) -> Packet {
        Packet::Unsubscribe(Unsubscribe{pid:pid, topics})
    }
}
prop_compose! {
    fn stg_unsuback()(pid in stg_pid()) -> Packet {
        Packet::Unsuback(pid)
    }
}
prop_compose! {
    fn stg_pingreq()(_ in bool::ANY) -> Packet {
        Packet::Pingreq
    }
}
prop_compose! {
    fn stg_pingresp()(_ in bool::ANY) -> Packet {
        Packet::Pingresp
    }
}
prop_compose! {
    fn stg_disconnect()(_ in bool::ANY) -> Packet {
        Packet::Disconnect
    }
}

/// Each call to this macro creates a unit test for a particular packet type.
macro_rules! impl_proptests {
    ($name:ident, $stg:ident) => {
        proptest! {
            /// Encodes packet generated by $stg and checks that `decode()`ing it yields the
            /// original packet back.
            #[test]
            fn $name(pkt in $stg()) {
                // Encode the packet
                let mut buf = BytesMut::with_capacity(10240);
                let res = encode(&pkt, &mut buf);
                prop_assert!(res.is_ok(), "encode({:?}) -> {:?}", pkt, res);
                prop_assert!(buf.len() >= 2, "buffer too small: {:?}", buf); //PING is 2 bytes
                prop_assert!(buf[0] >> 4 > 0 && buf[0] >> 4 < 16, "bad packet type {:?}", buf);

                // Check that decoding returns the original
                let encoded = buf.clone();
                let decoded = decode(&mut buf);
                let ok = match &decoded {
                    Ok(Some(p)) if *p == pkt => true,
                    _other => false,
                };
                prop_assert!(ok, "decode({:#x?}) -> {:?}", encoded.as_ref(), decoded);
                prop_assert!(buf.is_empty(), "Buffer not empty: {:?}", buf);

                // Check that decoding a partial packet returns Ok(None)
                let decoded = decode(&mut encoded.clone().split_off(encoded.len() - 1)).unwrap();
                prop_assert!(decoded.is_none(), "partial decode {:?} -> {:?}", encoded, decoded);

                // TODO: The next part can't fail anymore because ByteMut 0.5 grows as
                // needed. However, we want to restore support for non-growable buffers eventually
                // (especially for no-std), so I'm keeping this code around until decode() is
                // modified to accept other buffer types.

                // Check that encoding into a small buffer fails cleanly
                // buf.clear();
                // buf.split_off(encoded.len());
                // prop_assert!(encoded.len() == buf.remaining_mut() && buf.is_empty(),
                //             "Wrong buffer init1 {}/{}/{}", encoded.len(), buf.remaining_mut(), buf.is_empty());
                // prop_assert!(encode(&pkt, &mut buf).is_ok(), "exact buffer capacity {}", buf.capacity());
                // for l in (0..encoded.len()).rev() {
                //    buf.clear();
                //    buf.split_to(1);
                //    prop_assert!(l == buf.remaining_mut() && buf.is_empty(),
                //                 "Wrong buffer init2 {}/{}/{}", l, buf.remaining_mut(), buf.is_empty());
                //    prop_assert_eq!(Err(Error::WriteZero), encode(&pkt, &mut buf),
                //                    "small buffer capacity {}/{}", buf.capacity(), encoded.len());
                // }
            }
        }
    };
}
impl_proptests! {connect,     stg_connect}
impl_proptests! {connack,     stg_connack}
impl_proptests! {publish,     stg_publish}
impl_proptests! {puback,      stg_puback}
impl_proptests! {pubcomp,     stg_pubcomp}
impl_proptests! {pubrec,      stg_pubrec}
impl_proptests! {pubrel,      stg_pubrel}
impl_proptests! {subscribe,   stg_subscribe}
impl_proptests! {suback,      stg_suback}
impl_proptests! {unsubscribe, stg_unsubscribe}
impl_proptests! {unsuback,    stg_unsuback}
impl_proptests! {pingreq,     stg_pingreq}
impl_proptests! {pingresp,    stg_pingresp}
impl_proptests! {disconnect,  stg_disconnect}
