use super::*;

const NETWORKS_AND_PROTOCOL_NAMES: [(Network, &str); 3] = [
    (Network::Xbt, "xbt"),
    (Network::XbtTestnet4, "xbt-testnet4"),
    (Network::XbtRegtest, "xbt-regtest"),
];
const BITCOIN_MAINNET_PROTOCOL_NAME: &str = "mainnet";

#[test]
fn networks_encode_as_their_protocol_names() {
    for (network, protocol_name) in NETWORKS_AND_PROTOCOL_NAMES {
        let encoded_network = minicbor_serde::to_vec(network).expect("the network should encode");
        let decoded_protocol_name: String =
            minicbor_serde::from_slice(&encoded_network).expect("the protocol name should decode");
        assert_eq!(decoded_protocol_name, protocol_name, "network {network:?}");
    }
}

#[test]
fn every_network_is_found_by_its_protocol_name() {
    for (network, protocol_name) in NETWORKS_AND_PROTOCOL_NAMES {
        assert_eq!(
            Network::from_protocol_name(protocol_name),
            Some(network),
            "protocol name {protocol_name}"
        );
    }
}

#[test]
fn bitcoin_mainnet_protocol_name_finds_no_network() {
    assert_eq!(
        Network::from_protocol_name(BITCOIN_MAINNET_PROTOCOL_NAME),
        None
    );
}

#[test]
fn all_constant_lists_every_network() {
    assert_eq!(
        Network::ALL,
        NETWORKS_AND_PROTOCOL_NAMES.map(|(network, _)| network)
    );
}
