//!
//! This file implements mutable for the `pnet_packets` with additional features
//! making it possible to select only relevant fields which should be mutable.
//!
//!

/// Create constraints for the pnet mutations
pub mod constraints;

use super::pnet::constraints::{Ipv4Payload, RuleLayer, RulePacket};
use pnet::packet::ethernet::EtherType;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::vlan::ClassOfService;
use pnet_layers::{EtherMut, Ipv4Mut, LayerMut, LayerMutable, Layers, PayloadMut, UdpMut, VlanMut};
use profuzz_core::mutator::Mutator;
use profuzz_core::traits::Corpus;
use profuzz_core::traits::Mutable;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::sync::RwLock;

static CONSTRAINTS: std::sync::LazyLock<Arc<RwLock<Vec<RulePacket>>>> =
    std::sync::LazyLock::new(Arc::default);

/// Set the mutation constraints for the pnet `Mutable` implementation
/// # Errors
pub fn set_mutation_constraints(update: Vec<RulePacket>) -> Result<(), String> {
    if let Ok(mut c) = CONSTRAINTS.write() {
        *c = update;
        Ok(())
    } else {
        Err(
            "Could not lock the static CONSTRAINTS to update the constraints for mutation."
                .to_string(),
        )
    }
}

#[derive(Clone)]
/// Wrapper for the `EtherMut` because of the orphan-rule :/
pub struct EtherMutatorOwned(pub(crate) EtherMut);

#[derive(Debug)]
pub(crate) struct LayerMutator<'a>(&'a mut LayerMut, &'a Option<RulePacket>);

#[derive(Debug)]
pub(crate) struct EtherMutator<'a>(&'a mut EtherMut, &'a Option<RulePacket>);

#[derive(Debug)]
pub(crate) struct VlanMutator<'a>(&'a mut VlanMut, &'a Option<RulePacket>);

#[derive(Debug)]
pub(crate) struct Ipv4Mutator<'a>(&'a mut Ipv4Mut, &'a Option<RulePacket>);

#[derive(Debug)]
pub(crate) struct UdpMutator<'a>(&'a mut UdpMut, &'a Option<RulePacket>);

#[derive(Debug)]
pub(crate) struct PayloadMutator<'a>(&'a mut PayloadMut);

impl Corpus for EtherMutatorOwned {
    fn from_bytes(buf: Vec<u8>) -> Option<EtherMutatorOwned> {
        Some(EtherMutatorOwned(EtherMut::from_buf(buf)?))
    }
    fn to_bytes(self) -> Vec<u8> {
        self.0.build().expect("should not fail")
    }
    fn build(self) -> Vec<u8> {
        self.0.build().expect("should not fail")
    }
    fn show(&self) -> String {
        format!("{}", self.0)
    }
}

impl Mutable for EtherMutatorOwned {
    fn mutate<R: rand::Rng>(&mut self, mutator: &mut Mutator<R>) {
        let mut constraint = None;

        if let Ok(c) = CONSTRAINTS.read()
            && !c.is_empty()
        {
            let index = mutator.gen_index("constraints", c.len());
            constraint = c.get(index).cloned();
        }

        EtherMutator(&mut self.0, &constraint).mutate(mutator);
    }
}

impl std::fmt::Display for LayerMutator<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Mutable for LayerMutator<'_> {
    fn mutate<R: rand::Rng>(&mut self, mutator: &mut Mutator<R>) {
        match self.0 {
            LayerMut::Ether(a) => EtherMutator(a, self.1).mutate(mutator),
            LayerMut::Vlan(a) => VlanMutator(a, self.1).mutate(mutator),
            LayerMut::Ipv4(a) => Ipv4Mutator(a, self.1).mutate(mutator),
            LayerMut::Udp(a) => UdpMutator(a, self.1).mutate(mutator),
            LayerMut::Payload(a) => PayloadMutator(a).mutate(mutator),
            // LayerMut::Tcp(a) => a.mutate(mutator, constraints),
            // LayerMut::Arp(a) => a.mutate(mutator, constraints),
            // LayerMut::Raw(a) => a.mutate(mutator, constraints),
            // should be never reached as it will be added by the udp layer
            _ => {
                log::error!("implement mutate for {self}");
            }
        }
    }
}

impl Mutable for EtherMutator<'_> {
    fn mutate<R: rand::Rng>(&mut self, mutator: &mut Mutator<R>) {
        if let Some(mut ether) = self.0.modify() {
            let mut mutate_eth_type = true;
            if let Some(rule) = self.1 {
                if let Some(daddr) = rule.eth_daddr.as_mac_addr() {
                    ether.set_destination(daddr);
                }
                if let Some(saddr) = rule.eth_saddr.as_mac_addr() {
                    ether.set_source(saddr);
                }
                if !rule.eth_type.is_any() {
                    ether.set_ethertype(EtherType::new(rule.eth_type.as_u16()));
                    mutate_eth_type = false;
                }
            }
            if mutate_eth_type && mutator.gen_chance(0.1) {
                let mut mutable = ether.get_ethertype().0;
                mutator.mutate(&mut mutable);
                ether.set_ethertype(EtherType::new(mutable));
            }
        }
        if let Some(upperlayer) = self.0.upper_layer.as_mut() {
            LayerMutator(upperlayer, self.1).mutate(mutator);
        }
    }
}

impl Mutable for VlanMutator<'_> {
    fn mutate<R: rand::Rng>(&mut self, mutator: &mut Mutator<R>) {
        if let Some(mut vlan) = self.0.modify() {
            let mut mutate_vlan_id = true;
            if let Some(rule) = self.1
                && let Some(RuleLayer::Vlan(rule)) = rule.get_layer(&Layers::Vlan)
                && !rule.id.is_any()
            {
                vlan.set_vlan_identifier(rule.id.as_u16());
                mutate_vlan_id = false;
            }
            if mutate_vlan_id && mutator.gen_chance(0.5) {
                let mut mutable = vlan.get_vlan_identifier();
                mutator.mutate(&mut mutable);
                vlan.set_vlan_identifier(mutable);
            }
            if mutator.gen_chance(0.5) {
                let mut mutable = vlan.get_priority_code_point().0;
                mutator.mutate(&mut mutable);
                vlan.set_priority_code_point(ClassOfService::new(mutable));
            }
            if mutator.gen_chance(0.5) {
                let mut mutable = vlan.get_drop_eligible_indicator();
                mutator.mutate(&mut mutable);
                vlan.set_drop_eligible_indicator(mutable);
            }
        }
        if let Some(upperlayer) = self.0.upper_layer.as_mut() {
            LayerMutator(upperlayer, self.1).mutate(mutator);
        }
    }
}

impl Mutable for Ipv4Mutator<'_> {
    fn mutate<R: rand::Rng>(&mut self, mutator: &mut Mutator<R>) {
        if let Some(mut ipv4) = self.0.modify() {
            let mut mutate_src_ip = true;
            let mut mutate_dst_ip = true;
            let mut mutate_next_protocol = true;
            if let Some(rule) = self.1
                && let Some(RuleLayer::Ipv4(rule)) = rule.get_layer(&Layers::Ipv4)
            {
                if !rule.saddr.is_any()
                    && let Some(addr) = rule.saddr.as_ipv4()
                {
                    ipv4.set_source(addr);
                    mutate_src_ip = false;
                }
                if !rule.daddr.is_any()
                    && let Some(addr) = rule.daddr.as_ipv4()
                {
                    ipv4.set_destination(addr);
                    mutate_dst_ip = false;
                }
                if matches!(rule.payload, Ipv4Payload::Udp(_)) {
                    ipv4.set_next_level_protocol(IpNextHeaderProtocols::Udp);
                    mutate_next_protocol = false;
                }
                if matches!(rule.payload, Ipv4Payload::Tcp(_)) {
                    ipv4.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
                    mutate_next_protocol = false;
                }
            }

            // Mutate next_level_protocol
            if mutate_next_protocol && mutator.gen_chance(0.01) {
                let mut next_level_protocol = ipv4.get_next_level_protocol();
                mutator.mutate(&mut next_level_protocol.0);
                ipv4.set_next_level_protocol(next_level_protocol);
            }

            // Mutate source address
            if mutate_src_ip && mutator.gen_chance(0.4) {
                let mut mutable = ipv4.get_source().octets();
                for byte in &mut mutable {
                    mutator.mutate(byte);
                }
                ipv4.set_source(Ipv4Addr::from(mutable));
            }

            // Mutate destination address
            if mutate_dst_ip && mutator.gen_chance(0.1) {
                let mut mutable = ipv4.get_destination().octets();
                for byte in &mut mutable {
                    mutator.mutate(byte);
                }
                ipv4.set_destination(Ipv4Addr::from(mutable));
            }

            // Mutate version
            if mutator.gen_chance(0.001) {
                let mut version = ipv4.get_version();
                mutator.mutate(&mut version);
                ipv4.set_version(version);
            }

            // Mutate header_length
            if mutator.gen_chance(0.01) {
                let mut header_length = ipv4.get_header_length();
                mutator.mutate(&mut header_length);
                ipv4.set_header_length(header_length);
            }

            // Mutate dscp
            if mutator.gen_chance(0.2) {
                let mut dscp = ipv4.get_dscp();
                mutator.mutate(&mut dscp);
                ipv4.set_dscp(dscp);
            }

            // Mutate ecn
            if mutator.gen_chance(0.2) {
                let mut ecn = ipv4.get_ecn();
                mutator.mutate(&mut ecn);
                ipv4.set_ecn(ecn);
            }

            // Mutate total_length
            if mutator.gen_chance(0.01) {
                let mut total_length = ipv4.get_total_length();
                mutator.mutate(&mut total_length);
                ipv4.set_total_length(total_length);
            }

            // Mutate identification
            if mutator.gen_chance(0.5) {
                let mut identification = ipv4.get_identification();
                mutator.mutate(&mut identification);
                ipv4.set_identification(identification);
            }

            // Mutate flags
            // if mutator.gen_chance(0.1) {
            //     let mut flags = ipv4.get_flags();
            //     mutator.mutate(&mut flags);
            //     ipv4.set_flags(flags);
            // }

            // Mutate fragment_offset
            // if mutator.gen_chance(0.01) {
            //     let mut fragment_offset = ipv4.get_fragment_offset();
            //     mutator.mutate(&mut fragment_offset);
            //     ipv4.set_fragment_offset(fragment_offset);
            // }

            // Mutate ttl
            // if mutator.gen_chance(0.5) {
            //     let mut ttl = ipv4.get_ttl();
            //     mutator.mutate(&mut ttl);
            //     ipv4.set_ttl(ttl);
            // }
        }
        if let Some(upperlayer) = self.0.upper_layer.as_mut() {
            LayerMutator(upperlayer, self.1).mutate(mutator);
        }
    }
}

impl Mutable for UdpMutator<'_> {
    fn mutate<R: rand::Rng>(&mut self, mutator: &mut Mutator<R>) {
        if let Some(mut udp) = self.0.modify() {
            let mut mutate_src_port = true;
            let mut mutate_dst_port = true;
            if let Some(rule) = self.1
                && let Some(RuleLayer::Udp(rule)) = rule.get_layer(&Layers::Udp)
            {
                if !rule.sport.is_any()
                    && let Some(addr) = rule.sport.as_port()
                {
                    udp.set_source(addr);
                    mutate_src_port = false;
                }
                if !rule.dport.is_any()
                    && let Some(addr) = rule.dport.as_port()
                {
                    udp.set_destination(addr);
                    mutate_dst_port = false;
                }
            }

            // Mutate source port
            if mutate_src_port && mutator.gen_chance(0.4) {
                let mut source = udp.get_source();
                mutator.mutate(&mut source);
                udp.set_source(source);
            }

            // Mutate destination port
            if mutate_dst_port == mutator.gen_chance(0.01) {
                let mut destination = udp.get_destination();
                mutator.mutate(&mut destination);
                udp.set_destination(destination);
            }

            // Mutate length
            if mutator.gen_chance(0.01) {
                let mut length = udp.get_length();
                mutator.mutate(&mut length);
                udp.set_length(length);
            }

            // Mutate checksum
            if mutator.gen_chance(0.001) {
                let mut checksum = udp.get_checksum();
                mutator.mutate(&mut checksum);
                udp.set_checksum(checksum);
            }
        }

        if let Some(upperlayer) = self.0.upper_layer.as_mut() {
            LayerMutator(upperlayer, self.1).mutate(mutator);
        }
    }
}

impl Mutable for PayloadMutator<'_> {
    fn mutate<R: rand::Rng>(&mut self, mutator: &mut Mutator<R>) {
        let payload = &mut self.0.buf;

        let size = if mutator.gen_chance(0.01) {
            // this slowes down the fuzzing
            mutator.gen_range(0, 1000)
        } else if mutator.gen_chance(0.5) {
            mutator.gen_range(0, 50)
        } else {
            payload.len()
        };

        payload.resize(size, 0);

        // let mut index = 0;
        // if let Some(rule) = self.1 {
        //     if let Some(RuleLayer::Payload(rule)) = rule.get_layer(&Layers::Payload) {
        //         // match rule {
        //         //     Payload::Any => (),
        //         // }
        //     }
        // }
    }
}
