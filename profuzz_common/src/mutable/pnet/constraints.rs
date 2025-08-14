use pnet::util::MacAddr;
use pnet_layers::Layers;
use serde::{Deserialize, Serialize};
use std::{net::Ipv4Addr, str::FromStr};

/// This struct represents a single rule.
/// Every value which must be check MUST be NOT `Rpv::Any`
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default, Hash)]
pub struct RulePacket {
    /// eth source address
    pub eth_saddr: Rpv<String>,
    /// eth dest address
    pub eth_daddr: Rpv<String>,
    /// ethernet type
    pub eth_type: Rpv<u16>,
    /// eth payload
    pub eth_payload: EthPayload,
}

/// Different rules for the different layers
pub enum RuleLayer {
    /// ether
    Ether(RulePacket),
    /// vlan
    Vlan(RuleVlan),
    /// ipv4
    Ipv4(RuleIpv4),
    /// udp
    Udp(RuleUpd),
    /// tcp
    Tcp(RuleTcp),
    /// payload
    Payload(Payload),
}

/// Rule Packet Value
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default, Hash)]
pub enum Rpv<T: Clone> {
    #[default]
    /// Any
    Any,
    /// Equal
    Equal(T),
    /// Not Equal
    NotEqual(T),
    /// Greater Than
    GreaterThan(T),
    /// Contains
    Contains(Vec<T>),
    /// Not Contains
    NotContains(Vec<T>),
    /// Multiple
    Multiple(Vec<Rpv<T>>),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default)]
/// The different Rpv values
pub enum RpvType {
    #[default]
    /// Any
    Any,
    /// Equal
    Equal,
    /// Not equal
    NotEqual,
    /// Greater Than
    GreaterThan,
    /// contains
    Contains,
    /// not contains
    NotContains,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default, Hash)]
/// Ethernet payload
pub enum EthPayload {
    #[default]
    /// Any
    Any,
    /// Vlan
    Vlan(RuleVlan),
    /// Ipv4
    Ipv4(RuleIpv4),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default, Hash)]
/// Vlan rule params
pub struct RuleVlan {
    /// vlan id
    pub id: Rpv<u16>,
    /// Payload of the vlan
    pub payload: VlanPayload,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default, Hash)]
/// In autosar there is no vlan double tagging by design...
#[allow(clippy::large_enum_variant)]
pub enum VlanPayload {
    #[default]
    /// any
    Any,
    /// Ipv4
    Ipv4(RuleIpv4),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default, Hash)]
/// Contains ipv4 rule params
pub struct RuleIpv4 {
    /// mf
    pub mf: Rpv<bool>,
    /// fragment offset
    pub fragment_offset: Rpv<usize>,
    /// ihl
    pub ihl: Rpv<usize>,
    /// proto
    pub protocol: Rpv<u16>,
    /// destination ip
    pub daddr: Rpv<Ipv4Address>,
    /// source ip
    pub saddr: Rpv<Ipv4Address>,
    /// ipv4 payload
    pub payload: Ipv4Payload,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default, Hash)]
/// ipv4 payload
pub enum Ipv4Payload {
    #[default]
    /// any
    Any,
    /// udp
    Udp(RuleUpd),
    /// tcp
    Tcp(RuleTcp),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default, Hash)]
/// udp rule params
pub struct RuleUpd {
    /// source port
    pub sport: Rpv<Port>,
    /// dest port
    pub dport: Rpv<Port>,
    /// udp payload
    pub payload: Payload,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default, Hash)]
/// tcp rule params
pub struct RuleTcp {
    /// source port
    pub sport: Rpv<Port>,
    /// dest port
    pub dport: Rpv<Port>,
    /// tcp payload
    pub payload: Payload,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Default, Hash)]
/// packet payload
pub enum Payload {
    #[default]
    /// any
    Any,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Hash)]
/// Port either a range or a single port
pub enum Port {
    /// Range
    Range(u16, u16),
    /// Single
    Single(u16),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Clone, Deserialize, Hash)]
/// Ipv4 Address either a range or a single port
pub enum Ipv4Address {
    /// Range
    Range(String, String),
    /// Single
    Single(String),
}

impl<T: Clone + std::fmt::Debug> Rpv<T> {
    /// Checks if the Rpv is any
    pub fn is_any(&self) -> bool {
        matches!(self, Self::Any)
    }
}

impl RulePacket {
    /// Returns the rule for the given layer
    #[must_use]
    pub fn get_layer(&self, layer: &Layers) -> Option<RuleLayer> {
        if matches!(layer, Layers::Ether) {
            return Some(RuleLayer::Ether(self.to_owned()));
        }
        let payload = &self.eth_payload;
        let ipv4 = match &payload {
            EthPayload::Any => return None,
            EthPayload::Ipv4(ipv4) => ipv4,
            EthPayload::Vlan(vlan) => {
                if matches!(layer, Layers::Vlan) {
                    return Some(RuleLayer::Vlan(vlan.to_owned()));
                }
                match &vlan.payload {
                    VlanPayload::Any => return None,
                    VlanPayload::Ipv4(ipv4) => ipv4,
                }
            }
        };
        if matches!(layer, Layers::Ipv4) {
            return Some(RuleLayer::Ipv4(ipv4.to_owned()));
        }
        let payload = match &ipv4.payload {
            Ipv4Payload::Any => return None,
            Ipv4Payload::Tcp(tcp) => {
                if matches!(layer, Layers::Tcp) {
                    return Some(RuleLayer::Tcp(tcp.to_owned()));
                }
                &tcp.payload
            }
            Ipv4Payload::Udp(udp) => {
                if matches!(layer, Layers::Udp) {
                    return Some(RuleLayer::Udp(udp.to_owned()));
                }
                &udp.payload
            }
        };
        if matches!(layer, Layers::Payload) {
            return Some(RuleLayer::Payload(payload.to_owned()));
        }
        None
    }
}

impl Rpv<String> {
    /// Converts the `Rpv` to mac addr
    #[must_use]
    pub fn as_mac_addr(&self) -> Option<MacAddr> {
        match self {
            Rpv::Any | Rpv::NotEqual(_) => MacAddr::from_str("3c:ce:33:33:33:33").ok(),
            Rpv::Equal(a) => MacAddr::from_str(a).ok(),
            Rpv::Contains(a) => MacAddr::from_str(&a[0]).ok(),
            _ => {
                log::error!("{self:?} not allowed");
                None
            }
        }
    }
}

impl Rpv<Port> {
    /// Converts the `Rpv` to an port number
    #[must_use]
    pub fn as_port(&self) -> Option<u16> {
        match self {
            Rpv::Any => Some(9999),
            Rpv::Equal(Port::Single(a)) => Some(*a),
            Rpv::Contains(ports) => {
                for a in ports {
                    if let Port::Range(a, _b) = a {
                        return Some(*a + 1);
                    }
                    if let Port::Single(a) = a {
                        return Some(*a);
                    }
                }
                log::error!("Error contains in ipv4 not found");
                None
            }
            _ => todo!("{self:?} not allowed as port"),
        }
    }
}

impl Rpv<Ipv4Address> {
    /// Converts the `Rpv` to an Ipv4 address
    #[must_use]
    pub fn as_ipv4(&self) -> Option<Ipv4Addr> {
        match self {
            Rpv::Any => Some(Ipv4Addr::new(99, 99, 99, 99)),
            Rpv::Equal(Ipv4Address::Single(a)) => Ipv4Addr::from_str(a).ok(),
            Rpv::Contains(list) => {
                for a in list {
                    if let Ipv4Address::Range(a, _b) = a {
                        return Ipv4Addr::from_str(&a.replace('0', "9")).ok();
                    }
                    if let Ipv4Address::Single(a) = a {
                        return Ipv4Addr::from_str(a).ok();
                    }
                }
                log::error!("Error contains in ipv4 not found");
                None
            }
            _ => {
                log::error!("{self:?} not allowed");
                None
            }
        }
    }
}

impl Rpv<u16> {
    /// converts the rpv value to an u16
    #[must_use]
    pub fn as_u16(&self) -> u16 {
        match self {
            Rpv::Any => 1,
            Rpv::Equal(a) => *a,
            Rpv::Contains(a) => a[0],
            Rpv::NotContains(a) => a.iter().sum(),
            _ => todo!("{:?} not allowed as vlan id", self),
        }
    }
}

impl Rpv<usize> {
    /// converts the rpv value to an usize
    #[must_use]
    pub fn as_usize(&self) -> usize {
        match self {
            Rpv::Any => 1,
            Rpv::Equal(a) => *a,
            Rpv::Contains(a) => a[0],
            Rpv::NotContains(a) => a.iter().sum(),
            _ => todo!("{:?} not allowed as vlan id", self),
        }
    }
}
